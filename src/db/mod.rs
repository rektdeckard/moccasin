use crate::config::Config;
use anyhow::Result;
use gluesql::{
    core::ast_builder::{generate_uuid, table, text, Execute},
    prelude::Glue,
    sled_storage::SledStorage,
};
use rss::Channel;
use std::{fmt::Debug, time::Duration};
use std::{future, thread};
use tokio::sync::mpsc::{self, UnboundedSender};

#[derive(Clone, Copy, Debug)]
pub enum StorageEvent {
    FetchAll(usize),
}

pub struct Database {
    storage: Glue<SledStorage>,
    app_tx: mpsc::UnboundedSender<StorageEvent>,
    db_tx: mpsc::UnboundedSender<StorageEvent>,
    db_rx: mpsc::UnboundedReceiver<StorageEvent>,
}

impl Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database {}")
    }
}

impl Database {
    pub async fn init(config: &Config, app_tx: UnboundedSender<StorageEvent>) -> Result<Self> {
        let path = config.db_path();
        let storage =
            SledStorage::new(path.to_str().expect("could not serialize config path")).unwrap();
        let mut storage = Glue::new(storage);
        let db_schema = String::from_utf8_lossy(include_bytes!("schema.sql"));
        let _ = storage.execute(db_schema)?;

        let tick_rate = Duration::from_secs(config.refresh_interval());
        let (db_tx, db_rx) = mpsc::unbounded_channel::<StorageEvent>();

        let handle = {
            let app_tx = app_tx.clone();
            let urls = config.feed_urls().clone();

            // tokio::spawn(async move {
            //     let futures: Vec<_> = urls.into_iter().map(reqwest::get).collect();
            //     let handles: Vec<_> = futures.into_iter().map(tokio::task::spawn).collect();
            //     let results = future::join!(handles);

            //     app_tx.send(StorageEvent::FetchAll(0))
            // })
        };

        Ok(Self {
            storage,
            app_tx,
            db_tx,
            db_rx,
        })
    }
}
