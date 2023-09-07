use crate::config::Config;
use crate::feed::{Feed, Item};

pub mod polo;
pub mod sqlite;

pub enum StorageEvent {
    Insert,
    Update,
    Delete,
    NoOp,
}

pub struct StorageError;

pub trait Storage<E: Sized> {
    fn init(config: &Config) -> Self;
    fn read_all(&mut self, config: &Config) -> Result<Vec<Feed>, E>;
    fn read_items_for_feed_id(&self, id: &str) -> Result<Vec<Item>, E>;
    fn write_feed(&self, feed: &Feed) -> Result<StorageEvent, E>;
    fn write_feeds(&self, feeds: &Vec<Feed>) -> Result<Vec<StorageEvent>, E>;
    fn write_item(&self, item: &Item) -> Result<StorageEvent, E>;
    fn delete_feed_with_url(&self, url: &str) -> Result<StorageEvent, E>;
}
