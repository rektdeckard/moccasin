use crate::config::{Config, SortOrder};
use crate::feed::{Feed, Item};
use polodb_core::{
    bson,
    bson::doc,
    results::{DeleteResult, InsertManyResult, InsertOneResult, UpdateResult},
    Database, Error as PoloDBError,
};

pub struct Storage {
    db: Database,
}

pub enum StorageEvent {
    InsertOne(InsertOneResult),
    InsertMany(InsertManyResult),
    Update(UpdateResult),
    Delete(DeleteResult),
}

impl Storage {
    pub fn init(config: &Config) -> Storage {
        let db = if config.should_cache() {
            Database::open_file(config.db_path()).expect("could not open db")
        } else {
            Database::open_memory().expect("could not open db")
        };

        Self { db }
    }

    pub fn read_all(&mut self, config: &Config) -> Result<Vec<Feed>, PoloDBError> {
        let feeds = self.db.collection::<Feed>("feeds");
        let cursor = feeds.find(None)?;

        let mut feeds = cursor
            .into_iter()
            .filter_map(|f| f.ok())
            .collect::<Vec<Feed>>();

        self.sort_feeds(&mut feeds, config);
        Ok(feeds)
    }

    pub fn write_one(&self, feed: &Feed) -> Result<StorageEvent, PoloDBError> {
        let collection = self.db.collection::<Feed>("feeds");
        let query = doc! {  "link": feed.link() };
        let update = bson::to_document(feed)?;

        match collection.find_one(query.clone()) {
            Ok(Some(_)) => {
                let res = collection.update_one(query, update)?;
                Ok(StorageEvent::Update(res))
            }
            Ok(None) => {
                let res = collection.insert_one(feed)?;
                Ok(StorageEvent::InsertOne(res))
            }
            Err(e) => Err(e),
        }
    }

    pub fn write_all(&self, feeds: &Vec<Feed>) -> Result<Vec<StorageEvent>, PoloDBError> {
        let collection = self.db.collection::<Feed>("feeds");
        let mut successes = Vec::new();

        for feed in feeds {
            let query = doc! { "link": feed.link() };
            let update = bson::to_document(feed)?;

            match collection.find_one(query.clone())? {
                Some(_) => {
                    let res = collection.update_one(query, update)?;
                    successes.push(StorageEvent::Update(res));
                }
                None => {
                    let res = collection.insert_one(feed)?;
                    successes.push(StorageEvent::InsertOne(res));
                }
            }
        }

        Ok(successes)
    }

    pub fn delete_one(&mut self, url: &str) -> Result<StorageEvent, PoloDBError> {
        let collection = self.db.collection::<Feed>("feeds");
        let res = collection.delete_one(doc! { "link": url })?;
        Ok(StorageEvent::Delete(res))
    }

    fn sort_feeds(&self, feeds: &mut Vec<Feed>, config: &Config) {
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
}
