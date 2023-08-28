use crate::config::Config;
use crate::feed::Feed;
use anyhow::Result;
use polodb_core::{bson, bson::doc, Database};
use std::fmt::Debug;
use tokio::sync::mpsc::{self, UnboundedSender};

#[derive(Clone, Debug)]
pub enum StorageEvent {
    RetrievedAll(Vec<Feed>),
    Requesting(usize),
    Fetched((usize, usize)),
}

#[derive(Debug)]
enum FetchErr {
    Request,
    Deserialize,
    Parse,
}

pub struct Repository {
    db: Database,
    app_tx: mpsc::UnboundedSender<StorageEvent>,
    db_tx: mpsc::UnboundedSender<StorageEvent>,
    db_rx: mpsc::UnboundedReceiver<StorageEvent>,
}

impl Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database {}")
    }
}

impl Repository {
    pub async fn init(config: &Config, app_tx: UnboundedSender<StorageEvent>) -> Result<Self> {
        let db = Database::open_file(config.db_path()).expect("could not open db");

        let (db_tx, db_rx) = mpsc::unbounded_channel::<StorageEvent>();

        // let tick_rate = Duration::from_secs(config.refresh_interval());

        Ok(Self {
            db,
            app_tx,
            db_tx,
            db_rx,
        })
    }

    pub fn fetch_all_by_url(&mut self, _urls: &[String]) -> anyhow::Result<Vec<Feed>> {
        let feeds = self.db.collection::<Feed>("feeds");
        let cursor = feeds.find(None)?;

        let mut feeds = cursor
            .into_iter()
            .filter_map(|f| f.ok())
            .collect::<Vec<Feed>>();
        feeds.sort_by(|a, b| a.title().partial_cmp(b.title()).unwrap());
        Ok(feeds)
    }

    pub fn store_all(&self, feeds: &Vec<Feed>) -> anyhow::Result<()> {
        let collection = self.db.collection::<Feed>("feeds");
        for feed in feeds {
            let query = doc! {  "link": feed.link() };
            let update = bson::to_document(feed)?;

            match collection.find_one(query.clone()) {
                Ok(Some(_)) => {
                    let _ = collection.update_one(query, update);
                }
                Ok(None) => {
                    let _ = collection.insert_one(feed);
                }
                Err(_) => {}
            }
        }

        Ok(())
    }

    pub fn refresh_all_by_url(&mut self, urls: &[String]) {
        let app_tx = self.app_tx.clone();
        let urls = urls.to_vec();
        let count = urls.len();

        let _ = app_tx.send(StorageEvent::Requesting(count));

        tokio::spawn(async move {
            let futures: Vec<_> = urls.into_iter().map(reqwest::get).collect();
            let handles: Vec<_> = futures
                .into_iter()
                .enumerate()
                .map(|(n, req)| {
                    let app_tx = app_tx.clone();
                    tokio::task::spawn(async move {
                        let res = match req.await {
                            Ok(res) => match res.bytes().await {
                                Ok(bytes) => match Feed::read_from(&bytes[..]) {
                                    Ok(feed) => Ok(feed),
                                    Err(_) => Err(FetchErr::Parse),
                                },
                                Err(_) => Err(FetchErr::Deserialize),
                            },
                            Err(_) => Err(FetchErr::Request),
                        };
                        let _ = app_tx.send(StorageEvent::Fetched((n, count)));
                        res
                    })
                })
                .collect();
            let results = futures::future::join_all(handles).await;
            let mut feeds: Vec<_> = results
                .into_iter()
                .filter_map(|handle| match handle {
                    Ok(res) => match res {
                        Ok(channel) => Some(channel),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            feeds.sort_by(|a, b| a.title().partial_cmp(b.title()).unwrap());

            app_tx.send(StorageEvent::RetrievedAll(feeds))
        });
    }
}
