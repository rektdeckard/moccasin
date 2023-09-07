use super::{Storage, StorageError, StorageEvent};
use crate::config::Config;
use crate::feed::{Feed, Item};
use crate::util;
use log::{error, info, warn};
use rusqlite::{params_from_iter, Connection, Result, Row, ToSql};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub struct SQLiteStorage {
    conn: Connection,
}

trait FromRow<'stmt> {
    fn from_row(row: &'stmt Row) -> Self;
}

impl<'stmt> FromRow<'stmt> for Feed {
    fn from_row(row: &'stmt Row) -> Feed {
        Feed {
            id: row.get(0).unwrap(),
            title: row.get(1).unwrap(),
            description: row.get(2).unwrap(),
            categories: vec![],
            url: row.get(4).unwrap(),
            link: row.get(5).unwrap(),
            ttl: row.get(6).ok(),
            items: vec![],
            pub_date: row.get(7).ok(),
            last_fetched: row.get(8).ok(),
        }
    }
}

impl Storage<StorageError> for SQLiteStorage {
    fn init(config: &Config) -> Self {
        let conn = if config.should_cache() {
            Connection::open(config.db_path()).expect("Could not open database")
        } else {
            Connection::open_in_memory().expect("Could not open database")
        };

        conn.execute_batch(include_str!("schema.sql"))
            .expect("Failed to initialize DB schema");

        Self { conn }
    }

    fn read_all(&mut self, config: &Config) -> Result<Vec<Feed>, StorageError> {
        let stmt = "SELECT * FROM feeds";
        let mut stmt = self.conn.prepare_cached(stmt).map_err(|_| StorageError)?;

        let feeds_iter = stmt.query_map([], |row| Ok(Feed::from_row(row)));
        let mut feeds = feeds_iter
            .expect("Could not unwrap query")
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        util::sort_feeds(&mut feeds, config);
        Ok(feeds)
    }

    fn read_items_for_feed_id(&self, id: &str) -> Result<Vec<Item>, StorageError> {
        todo!()
    }

    fn write_feed(&self, feed: &Feed) -> Result<StorageEvent, StorageError> {
        let feed_stmt = "INSERT OR REPLACE INTO feeds(
            id,
            title,
            description,
            categories,
            url,
            link,
            ttl,
            pub_date,
            last_fetched
        ) VALUES(
            IFNULL((SELECT id FROM feeds WHERE id = ?1), ?1),
            ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
        )";

        let mut feed_stmt = self.conn.prepare_cached(feed_stmt).map_err(|err| {
            warn!("{:?}", err);
            StorageError
        })?;

        match feed_stmt.execute([
            feed.id(),
            feed.title(),
            feed.description(),
            "[]",
            feed.url(),
            feed.link(),
            feed.ttl().unwrap_or("NULL"),
            feed.pub_date().unwrap_or("NULL"),
            feed.last_fetched().unwrap_or("NULL"),
        ]) {
            Ok(_) => {
                for item in feed.items() {
                    self.write_item(item)?;
                }

                Ok(StorageEvent::Insert)
            }
            Err(err) => {
                error!("{:?}", err);
                Err(StorageError)
            }
        }
    }

    fn write_feeds(&self, feeds: &Vec<Feed>) -> Result<Vec<StorageEvent>, StorageError> {
        feeds.iter().map(|f| self.write_feed(f)).collect()
    }

    fn write_item(&self, item: &Item) -> Result<StorageEvent, StorageError> {
        todo!()
    }

    fn delete_feed_with_url(&self, url: &str) -> Result<StorageEvent, StorageError> {
        todo!()
    }
}
