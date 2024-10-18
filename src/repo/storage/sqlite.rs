use super::{StorageError, StorageEvent};
use crate::config::Config;
use crate::feed::{Feed, Item};
use crate::util;
use rusqlite::{Connection, Result, Row, Transaction};

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

impl<'stmt> Item {
    fn from_row(row: &'stmt Row, feed_id: &str) -> Self {
        Item {
            id: row.get(0).unwrap(),
            feed_id: feed_id.into(),
            title: row.get(2).ok(),
            author: row.get(3).ok(),
            content: row.get(4).ok(),
            description: row.get(5).ok(),
            text_description: row.get(6).ok(),
            categories: vec![], // FIXME
            link: row.get(8).ok(),
            pub_date: row.get(9).ok(),
        }
    }
}

impl SQLiteStorage {
    pub fn write_feed_tx(
        &self,
        feed: &Feed,
        tx: &Transaction,
    ) -> Result<StorageEvent, StorageError> {
        let stmt = "INSERT OR REPLACE INTO feeds(
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

        let mut stmt = tx.prepare_cached(stmt).map_err(|err| {
            log::warn!("{:?}", err);
            StorageError
        })?;

        match stmt.execute([
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
                log::error!("{:?}", err);
                Err(StorageError)
            }
        }
    }
}

impl SQLiteStorage {
    pub fn init(config: &Config) -> Self {
        let conn = if config.should_cache() {
            Connection::open(config.db_path()).expect("Could not open database")
        } else {
            Connection::open_in_memory().expect("Could not open database")
        };

        conn.execute_batch(include_str!("schema.sql"))
            .expect("Failed to initialize DB schema");

        Self { conn }
    }

    pub fn read_all(&mut self, config: &Config) -> Result<Vec<Feed>, StorageError> {
        let stmt = "SELECT * FROM feeds";
        let mut stmt = self.conn.prepare_cached(stmt).map_err(|_| StorageError)?;

        let feeds_iter = stmt.query_map([], |row| {
            let mut feed = Feed::from_row(row);
            match self.read_items_for_feed_id(feed.id()) {
                Ok(items) => feed.items = items,
                Err(_) => {
                    log::error!("Failed to fetch items for feed {}", feed.id());
                }
            }
            Ok(feed)
        });
        let mut feeds = feeds_iter
            .expect("Could not unwrap feeds")
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        util::sort_feeds(&mut feeds, config);
        Ok(feeds)
    }

    pub fn read_items_for_feed_id(&self, id: &str) -> Result<Vec<Item>, StorageError> {
        let stmt = "SELECT * FROM items WHERE feed_id = ?1";
        let mut stmt = self.conn.prepare_cached(stmt).map_err(|_| StorageError)?;

        let items_iter = stmt.query_map([id], |r| Ok(Item::from_row(r, id)));
        let items = items_iter
            .expect("Could not unwrap items")
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        Ok(items)
    }

    pub fn write_feed(
        &self,
        feed: &Feed,
        tx: Option<&Transaction>,
    ) -> Result<StorageEvent, StorageError> {
        let stmt = "INSERT OR REPLACE INTO feeds(
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

        let mut stmt = (if let Some(tx) = tx {
            tx.prepare_cached(stmt)
        } else {
            self.conn.prepare_cached(stmt)
        })
        .map_err(|err| {
            log::warn!("{:?}", err);
            StorageError
        })?;

        match stmt.execute([
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
                log::error!("{:?}", err);
                Err(StorageError)
            }
        }
    }

    pub fn write_feeds(&mut self, feeds: &Vec<Feed>) -> Result<Vec<StorageEvent>, StorageError> {
        if let Ok(tx) = self.conn.transaction() {
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

            let item_stmt = "INSERT OR REPLACE INTO items(
                    id,
                    feed_id,
                    title,
                    author,
                    content,
                    description,
                    text_description,
                    categories,
                    link,
                    pub_date
                ) VALUES(
                    IFNULL((SELECT id FROM items WHERE id = ?1), ?1),
                    ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
                )";

            let mut feed_stmt = tx.prepare_cached(feed_stmt).map_err(|err| {
                log::warn!("{:?}", err);
                StorageError
            })?;

            let mut item_stmt = tx.prepare_cached(item_stmt).map_err(|err| {
                log::warn!("{:?}", err);
                StorageError
            })?;

            let mut events = vec![];

            for feed in feeds {
                if let Err(e) = feed_stmt.execute([
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
                    log::error!("{e:?}");
                    return Err(StorageError);
                }

                for item in feed.items() {
                    if let Err(e) = item_stmt.execute([
                        item.id(),
                        item.feed_id(),
                        item.title().unwrap_or("NULL"),
                        item.author().unwrap_or("NULL"),
                        item.content().unwrap_or("NULL"),
                        item.description().unwrap_or("NULL"),
                        item.description().unwrap_or("NULL"),
                        "[]",
                        item.link().unwrap_or("NULL"),
                        item.pub_date().unwrap_or("NULL"),
                    ]) {
                        log::error!("{e:?}");
                        return Err(StorageError);
                    }
                }

                events.push(StorageEvent::Insert);
            }
            return Ok(events);
        } else {
            log::error!("");
            Err(StorageError)
        }
    }

    pub fn write_item(&self, item: &Item) -> Result<StorageEvent, StorageError> {
        let stmt = "INSERT OR REPLACE INTO items(
            id,
            feed_id,
            title,
            author,
            content,
            description,
            text_description,
            categories,
            link,
            pub_date
        ) VALUES(
            IFNULL((SELECT id FROM items WHERE id = ?1), ?1),
            ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
        )";

        let mut stmt = self.conn.prepare_cached(stmt).map_err(|err| {
            log::warn!("{:?}", err);
            StorageError
        })?;

        match stmt.execute([
            item.id(),
            item.feed_id(),
            item.title().unwrap_or("NULL"),
            item.author().unwrap_or("NULL"),
            item.content().unwrap_or("NULL"),
            item.description().unwrap_or("NULL"),
            item.description().unwrap_or("NULL"),
            "[]",
            item.link().unwrap_or("NULL"),
            item.pub_date().unwrap_or("NULL"),
        ]) {
            Ok(_) => Ok(StorageEvent::Insert),
            Err(err) => {
                log::error!("{:?}", err);
                Err(StorageError)
            }
        }
    }

    pub fn delete_feed_with_url(&self, url: &str) -> Result<StorageEvent, StorageError> {
        let stmt = "DELETE FROM feeds WHERE url = ?1";
        let mut stmt = self.conn.prepare_cached(stmt).map_err(|_| StorageError)?;

        match stmt.execute([url]) {
            Ok(delete_count) if delete_count > 0 => Ok(StorageEvent::Delete),
            Ok(_) => Ok(StorageEvent::NoOp),
            Err(_) => {
                log::error!("Failed to delete feed with url {}", url);
                Err(StorageError)
            }
        }
    }
}
