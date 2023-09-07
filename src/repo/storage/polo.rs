use super::{Storage, StorageEvent};
use crate::config::Config;
use crate::feed::{Feed, Item};
use crate::util;
use log::{info, warn};
use polodb_core::{bson, bson::doc, ClientSession, Database, Error as PoloDBError};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    current: bool,
    version: String,
    duck_type: String,
}

impl PartialEq for Metadata {
    fn eq(&self, other: &Self) -> bool {
        self.duck_type.eq(&other.duck_type)
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

pub struct PoloStorage {
    db: Database,
}

impl Storage<PoloDBError> for PoloStorage {
    fn init(config: &Config) -> PoloStorage {
        let db = if config.should_cache() {
            Database::open_file(config.db_path()).expect("Could not open database")
        } else {
            Database::open_memory().expect("Could not open database")
        };

        PoloStorage::check_migration(&db);

        Self { db }
    }

    fn read_items_for_feed_id(&self, id: &str) -> Result<Vec<Item>, PoloDBError> {
        let collection = self.db.collection::<Item>("items");
        let query = doc! { "feed_id": id };

        let cursor = collection.find(query)?;
        Ok(cursor.into_iter().filter_map(|i| i.ok()).collect())
    }

    fn read_all(&mut self, config: &Config) -> Result<Vec<Feed>, PoloDBError> {
        let feeds = self.db.collection::<Feed>("feeds");
        let cursor = feeds.find(None)?;

        let mut feeds = cursor
            .into_iter()
            .filter_map(|f| match f {
                Err(_) => None,
                Ok(f) => match self.read_items_for_feed_id(f.id()) {
                    Ok(items) => Some(f.with_items(items)),
                    Err(_) => None,
                },
            })
            .collect::<Vec<Feed>>();

        util::sort_feeds(&mut feeds, config);
        Ok(feeds)
    }

    #[allow(dead_code)]
    fn write_item(&self, item: &Item) -> Result<StorageEvent, PoloDBError> {
        let collection = self.db.collection::<Item>("items");
        let query = doc! {  "id": item.id() };

        match collection.find_one(query.clone()) {
            Ok(Some(_)) => {
                let update = bson::to_document(item)?;
                collection.update_one(query, update)?;
                Ok(StorageEvent::Update)
            }
            Ok(None) => {
                collection.insert_one(item)?;
                Ok(StorageEvent::Insert)
            }
            Err(e) => Err(e),
        }
    }

    fn write_feed(&self, feed: &Feed) -> Result<StorageEvent, PoloDBError> {
        let collection = self.db.collection::<Feed>("feeds");
        let query = doc! {  "id": feed.id() };

        match collection.find_one(query.clone()) {
            Ok(Some(_)) => {
                let update = bson::to_document(feed)?;
                info!(
                    "{}",
                    doc! {
                        "$set": &update,
                    }
                );
                collection.update_one(
                    query,
                    doc! {
                        "$set": update,
                    },
                )?;
                for item in feed.items() {
                    self.write_item(item)?;
                }
                Ok(StorageEvent::Update)
            }
            Ok(None) => {
                collection.insert_one(feed)?;
                for item in feed.items() {
                    self.write_item(item)?;
                }
                Ok(StorageEvent::Insert)
            }
            Err(e) => Err(e),
        }
    }

    fn write_feeds(&self, feeds: &Vec<Feed>) -> Result<Vec<StorageEvent>, PoloDBError> {
        feeds.iter().map(|f| self.write_feed(f)).collect()
    }

    fn delete_feed_with_url(&self, url: &str) -> Result<StorageEvent, PoloDBError> {
        let mut session = self.db.start_session()?;
        session.start_transaction(None).unwrap();
        let collection = self.db.collection::<Feed>("feeds");
        let query = doc! { "url": url };

        match collection.find_one(query.clone()) {
            Ok(Some(f)) => {
                let res = collection.delete_one_with_session(query, &mut session)?;
                if res.deleted_count > 0 {
                    info!("Deleted feed for {}", url);
                    self.delete_items_for_feed_id_with_session(f.id(), &mut session)?;
                }
                session.commit_transaction()?;
                Ok(StorageEvent::Delete)
            }
            Ok(None) => {
                info!("Did not find feed");
                session.abort_transaction()?;
                Ok(StorageEvent::NoOp)
            }
            Err(e) => {
                warn!("Error finding feed to delete");
                session.abort_transaction()?;
                Err(e)
            }
        }
    }
}

impl PoloStorage {
    fn check_migration(db: &Database) {
        let current: Metadata = Metadata {
            current: true,
            version: option_env!("CARGO_PKG_VERSION").unwrap_or("unknown").into(),
            duck_type: Feed::duck_type(),
        };

        let query = doc! { "current": true };
        let meta = db.collection::<Metadata>("meta");

        match meta.find_one(query.clone()) {
            Ok(Some(existing)) => {
                if current.ne(&existing) {
                    PoloStorage::migrate_db(db, &current);
                }
            }
            _ => {
                PoloStorage::migrate_db(db, &current);
            }
        }
    }

    fn migrate_db(db: &Database, current: &Metadata) {
        for collection_name in db.list_collection_names().unwrap() {
            let coll = db.collection::<PhantomData<bool>>(&collection_name);
            coll.drop().expect("Could not drop table");
        }

        let meta = db.collection::<Metadata>("meta");
        meta.insert_one(current)
            .expect("could not update db schema metadata");
    }

    #[allow(dead_code)]
    pub fn write_item_with_session(
        &self,
        item: &Item,
        session: &mut ClientSession,
    ) -> Result<StorageEvent, PoloDBError> {
        let collection = self.db.collection::<Item>("items");
        let query = doc! {  "id": item.id() };

        match collection.find_one(query.clone()) {
            Ok(Some(_)) => {
                let update = bson::to_document(item)?;
                collection.update_one_with_session(query, update, session)?;
                Ok(StorageEvent::Update)
            }
            Ok(None) => {
                collection.insert_one_with_session(item, session)?;
                Ok(StorageEvent::Insert)
            }
            Err(e) => Err(e),
        }
    }

    pub fn delete_items_for_feed_id_with_session(
        &self,
        id: &str,
        session: &mut ClientSession,
    ) -> Result<StorageEvent, PoloDBError> {
        let collection = self.db.collection::<Item>("items");
        let query = doc! { "feed_id": id };

        match collection.delete_many_with_session(query, session) {
            Ok(r) => {
                info!("Deleted {} items for feed id {}", r.deleted_count, id);
                Ok(StorageEvent::Delete)
            }
            Err(e) => Err(e),
        }
    }
}
