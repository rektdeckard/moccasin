use crate::config::Config;
use crate::feed::{Feed, Item};
use anyhow::Result;
use polodb_core::results::InsertManyResult;
use polodb_core::{bson::doc, Database};
use std::fmt::Debug;
use tokio::sync::mpsc::{self, UnboundedSender};

#[derive(Clone, Debug)]
pub enum StorageEvent {
    FetchAll(Vec<Feed>),
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

    pub fn fetch_all_by_url(&mut self, urls: &[String]) -> anyhow::Result<Vec<Feed>> {
        let feeds = self.db.collection::<Feed>("feeds");
        // let mut concat = "[".to_string();
        // concat.push_str(
        //     &urls
        //         .iter()
        //         .map(|s| {
        //             let mut a = "\"".to_string();
        //             a.push_str(s);
        //             a.push_str("\"");
        //             a
        //         })
        //         .collect::<Vec<String>>()
        //         .join(", "),
        // );
        // concat.push_str("]");

        // println!(
        //     "{:?}",
        //     doc! {
        //         "link": { "$in": ["https://atomicbird.com/index.xml", "https://stackoverflow.blog/feed/", "https://www.joshwcomeau.com/rss.xml", "https://buffer.com/resources/overflow/rss/", "https://feeds.simplecast.com/dLRotFGk", "https://www.nasa.gov/rss/dyn/breaking_news.rss", "https://shirtloadsofscience.libsyn.com/rss"] }
        //     }
        // );
        let cursor = feeds.find(None)?;

        Ok(cursor
            .into_iter()
            .filter_map(|f| f.ok())
            .collect::<Vec<Feed>>())
    }

    pub fn store_all(&self, feeds: &Vec<Feed>) -> polodb_core::Result<InsertManyResult> {
        self.db.collection::<Feed>("feeds").insert_many(feeds)
    }

    pub async fn refresh_all_by_url(&mut self, urls: &[String]) {
        let app_tx = self.app_tx.clone();
        let urls = urls.to_vec();

        tokio::spawn(async move {
            let futures: Vec<_> = urls.into_iter().map(reqwest::get).collect();
            let handles: Vec<_> = futures
                .into_iter()
                .map(|req| {
                    tokio::task::spawn(async move {
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
                    })
                })
                .collect();
            let results = futures::future::join_all(handles).await;
            let feeds: Vec<_> = results
                .into_iter()
                .filter_map(|handle| match handle {
                    Ok(res) => match res {
                        Ok(channel) => Some(channel),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();

            app_tx.send(StorageEvent::FetchAll(feeds))
        });
    }
}
