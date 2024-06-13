use super::RepositoryEvent;
use super::storage::sqlite::SQLiteStorage;
use crate::config::Config;
use crate::feed::Feed;
use crate::repo::storage::{StorageError, StorageEvent};
use crate::report;
use crate::util::sort_feeds;
use anyhow::Result;
use std::fmt::Debug;
use std::task::Poll;
use std::thread;
use std::time::Duration;
use tokio::{
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
};

#[derive(Debug)]
enum FetchErr {
    Request,
    Deserialize,
    Parse,
}

pub struct Repository {
    storage: SQLiteStorage,
    app_tx: mpsc::UnboundedSender<RepositoryEvent>,
    storage_tx: mpsc::UnboundedSender<RepositoryEvent>,
    storage_rx: mpsc::UnboundedReceiver<RepositoryEvent>,
    handle_one: Option<JoinHandle<()>>,
    handle_many: Option<JoinHandle<()>>,
}

impl Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database {}")
    }
}

impl Repository {
    pub fn init(config: &Config, app_tx: UnboundedSender<RepositoryEvent>) -> Result<Self> {
        let storage = SQLiteStorage::init(config);

        let (storage_tx, storage_rx) = mpsc::unbounded_channel::<RepositoryEvent>();

        if config.refresh_interval() > 0 {
            let tick_rate = Duration::from_secs(config.refresh_interval());
            let tx = storage_tx.clone();
            thread::spawn(move || loop {
                tx.send(RepositoryEvent::Refresh)
                    .expect("Failed to send storage message");
                thread::sleep(tick_rate);
            });
        }

        Ok(Self {
            storage,
            app_tx,
            storage_tx,
            storage_rx,
            handle_one: None,
            handle_many: None,
        })
    }

    pub fn tick(&mut self, config: &Config) {
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        match self.storage_rx.poll_recv(&mut cx) {
            Poll::Ready(m) => match m {
                Some(RepositoryEvent::RetrievedAll(feeds)) => {
                    report!(self.storage.write_feeds(&feeds), "Failed to write feeds");
                    self.app_tx
                        .send(RepositoryEvent::RetrievedAll(feeds))
                        .expect("Failed to send app message");
                    self.handle_many = None;
                }
                Some(RepositoryEvent::RetrievedOne(feed)) => {
                    report!(self.storage.write_feed(&feed, None), "Failed to write feed");
                    self.app_tx
                        .send(RepositoryEvent::RetrievedOne(feed))
                        .expect("Failed to send app message");
                    self.handle_one = None;
                }
                Some(RepositoryEvent::Refresh) => {
                    self.refresh_all(config);
                }
                Some(_) => {}
                None => {}
            },
            Poll::Pending => {}
        }
    }

    pub fn read_all(&mut self, config: &Config) -> Result<Vec<Feed>, StorageError> {
        let res = self.storage.read_all(config);
        report!(res, "Failed to read from DB");
        res
    }

    pub fn add_feed_url(&mut self, url: &str, config: &Config) {
        let app_tx = self.app_tx.clone();
        if let Some(handle) = &self.handle_one {
            handle.abort();
            app_tx
                .send(RepositoryEvent::Aborted)
                .expect("Failed to send app event");
            self.handle_one = None;
        }

        let url = url.to_owned();
        let interval = config.refresh_timeout();
        let storage_tx = self.storage_tx.clone();

        app_tx
            .send(RepositoryEvent::Requesting(1))
            .expect("Failed to send app event");

        self.handle_one = Some(tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(interval))
                .timeout(Duration::from_secs(interval))
                .build()
                .expect("failed to build client");

            match make_feed_request(client.get(url).send()).await {
                Ok(feed) => {
                    app_tx
                        .send(RepositoryEvent::Requested((1, 1)))
                        .expect("Failed to send app event");
                    storage_tx
                        .send(RepositoryEvent::RetrievedOne(feed))
                        .expect("Failed to send app event");
                }
                Err(_) => {
                    app_tx
                        .send(RepositoryEvent::Errored)
                        .expect("Failed to make feed request");
                }
            }
        }));
    }

    pub fn remove_feed_url(&mut self, url: &str) -> Result<StorageEvent, StorageError> {
        self.storage.delete_feed_with_url(url)
    }

    pub fn refresh_all(&mut self, config: &Config) {
        let app_tx = self.app_tx.clone();
        if let Some(handle) = &self.handle_many {
            handle.abort();
            app_tx
                .send(RepositoryEvent::Aborted)
                .expect("Failed to send abort message");
            self.handle_many = None;
        }

        let storage_tx = self.storage_tx.clone();
        let config: Config = config.clone();
        let urls = config.feed_urls().clone();
        let count = urls.len();

        app_tx
            .send(RepositoryEvent::Requesting(count))
            .expect("Could not send app message");

        self.handle_many = Some(tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(config.refresh_timeout()))
                .timeout(Duration::from_secs(config.refresh_timeout()))
                .build()
                .expect("Failed to build client");
            let futures: Vec<_> = urls.into_iter().map(|url| client.get(url).send()).collect();
            let handles: Vec<_> = futures
                .into_iter()
                .enumerate()
                .map(|(n, req)| {
                    let app_tx = app_tx.clone();
                    tokio::task::spawn(async move {
                        let res = make_feed_request(req).await;
                        app_tx
                            .send(RepositoryEvent::Requested((n, count)))
                            .expect("Failed to send app message");
                        res
                    })
                })
                .collect();
            let results = futures::future::join_all(handles).await;
            let mut feeds: Vec<Feed> = results
                .into_iter()
                .filter_map(|handle| match handle {
                    Ok(res) => match res {
                        Ok(feed) => Some(feed),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();

            sort_feeds(&mut feeds, &config);
            storage_tx
                .send(RepositoryEvent::RetrievedAll(feeds))
                .expect("Failed to send storage message");
        }));
    }
}

async fn make_feed_request(
    req: impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
) -> Result<Feed, FetchErr> {
    match req.await {
        Ok(res) => {
            let url = res.url().to_string();
            match &res.bytes().await {
                Ok(bytes) => match Feed::read_from(&bytes[..], url) {
                    Ok(feed) => Ok(feed),
                    Err(_) => Err(FetchErr::Parse),
                },
                Err(_) => Err(FetchErr::Deserialize),
            }
        }
        Err(_) => Err(FetchErr::Request),
    }
}
