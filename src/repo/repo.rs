use super::storage::{Storage, StorageEvent};
use super::RepositoryEvent;
use crate::config::{Config, SortOrder};
use crate::feed::Feed;
use anyhow::Result;
use polodb_core::Error as PoloDBError;
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

enum StorageRequest {
    InsertOne,
    UpsertMany,
}

pub struct Repository {
    storage: Storage,
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

fn sort_feeds(feeds: &mut Vec<Feed>, config: &Config) {
    match config.sort_order() {
        SortOrder::Az => {
            feeds.sort_by(|a, b| a.title().partial_cmp(b.title()).unwrap());
        }
        SortOrder::Za => {
            feeds.sort_by(|a, b| b.title().partial_cmp(a.title()).unwrap());
        }
        SortOrder::Custom => {
            let urls = config.feed_urls();
            feeds.sort_by(|a, b| {
                let a_index = urls.iter().position(|u| a.link() == u).unwrap_or_default();
                let b_index = urls.iter().position(|u| b.link() == u).unwrap_or_default();
                a_index.cmp(&b_index)
            })
        }
        SortOrder::Unread => {
            unimplemented!()
        }
        SortOrder::Newest => feeds.sort_by(|a, b| a.last_fetched().cmp(&b.last_fetched())),
        SortOrder::Oldest => feeds.sort_by(|a, b| b.last_fetched().cmp(&a.last_fetched())),
    }
}

impl Repository {
    pub fn init(config: &Config, app_tx: UnboundedSender<RepositoryEvent>) -> Result<Self> {
        let storage = Storage::init(config);
        let tick_rate = Duration::from_secs(config.refresh_interval());

        let (storage_tx, storage_rx) = mpsc::unbounded_channel::<RepositoryEvent>();

        let tx = storage_tx.clone();
        thread::spawn(move || loop {
            tx.send(RepositoryEvent::Refresh);
            thread::sleep(tick_rate);
        });

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

        loop {
            match self.storage_rx.poll_recv(&mut cx) {
                Poll::Ready(m) => match m {
                    Some(RepositoryEvent::RetrievedAll(feeds)) => {
                        self.storage.write_all(&feeds);
                        self.app_tx.send(RepositoryEvent::RetrievedAll(feeds));
                        self.handle_many = None;
                    }
                    Some(RepositoryEvent::RetrievedOne(feed)) => {
                        self.storage.write_one(&feed);
                        self.app_tx.send(RepositoryEvent::RetrievedOne(feed));
                        self.handle_one = None;
                    }
                    Some(RepositoryEvent::Refresh) => {
                        self.refresh_all(config);
                    }
                    Some(_) => {
                        // Skip other messages
                    }
                    None => {
                        break;
                    }
                },
                Poll::Pending => {
                    break;
                }
            }
        }
    }

    pub fn read_all(&mut self, config: &Config) -> Result<Vec<Feed>, PoloDBError> {
        self.storage.read_all(config)
    }

    pub fn add_feed_url(&mut self, url: &str, config: &Config) {
        let app_tx = self.app_tx.clone();
        if let Some(handle) = &self.handle_one {
            handle.abort();
            app_tx.send(RepositoryEvent::Aborted);
            self.handle_one = None;
        }

        let url = url.to_owned();
        let interval = config.refresh_timeout();
        let storage_tx = self.storage_tx.clone();

        let _ = app_tx.send(RepositoryEvent::Requesting(1));

        self.handle_one = Some(tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(interval))
                .timeout(Duration::from_secs(interval))
                .build()
                .expect("failed to build client");

            match make_feed_request(client.get(url).send()).await {
                Ok(feed) => {
                    let _ = app_tx.send(RepositoryEvent::Requested((1, 1)));
                    let _ = storage_tx.send(RepositoryEvent::RetrievedOne(feed));
                }
                Err(_) => {
                    let _ = app_tx.send(RepositoryEvent::Errored);
                }
            }
        }));
    }

    pub fn remove_feed_url(&mut self, url: &str) -> Result<StorageEvent, PoloDBError> {
        self.storage.delete_one(url)
    }

    pub fn refresh_all(&mut self, config: &Config) {
        let app_tx = self.app_tx.clone();
        if let Some(handle) = &self.handle_many {
            handle.abort();
            app_tx
                .send(RepositoryEvent::Aborted)
                .expect("SENDING ABORT");
            self.handle_many = None;
        }

        let storage_tx = self.storage_tx.clone();
        let config: Config = config.clone();
        let urls = config.feed_urls().clone();
        let count = urls.len();

        let _ = app_tx.send(RepositoryEvent::Requesting(count));

        self.handle_many = Some(tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(config.refresh_timeout()))
                .timeout(Duration::from_secs(config.refresh_timeout()))
                .build()
                .expect("failed to build client");
            let futures: Vec<_> = urls.into_iter().map(|url| client.get(url).send()).collect();
            let handles: Vec<_> = futures
                .into_iter()
                .enumerate()
                .map(|(n, req)| {
                    let app_tx = app_tx.clone();
                    tokio::task::spawn(async move {
                        let res = make_feed_request(req).await;
                        let _ = app_tx.send(RepositoryEvent::Requested((n, count)));
                        res
                    })
                })
                .collect();
            let results = futures::future::join_all(handles).await;
            let mut feeds: Vec<Feed> = results
                .into_iter()
                .filter_map(|handle| match handle {
                    Ok(res) => match res {
                        Ok(channel) => Some(channel),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();

            sort_feeds(&mut feeds, &config);
            let _ = storage_tx.send(RepositoryEvent::RetrievedAll(feeds));
        }));
    }
}

async fn make_feed_request(
    req: impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
) -> Result<Feed, FetchErr> {
    match req.await {
        Ok(res) => match res.bytes().await {
            Ok(bytes) => match Feed::read_from(&bytes[..]) {
                Ok(feed) => Ok(feed),
                Err(_) => Err(FetchErr::Parse),
            },
            Err(_) => Err(FetchErr::Deserialize),
        },
        Err(_) => Err(FetchErr::Request),
    }
}
