use crate::config::{Config, SortOrder};
use crate::feed::Feed;

pub fn sort_feeds(feeds: &mut Vec<Feed>, config: &Config) {
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

#[macro_export]
macro_rules! report {
    ($fallible:expr, $message:literal) => {
        match $fallible {
            Err(_) => {
                use log::error;
                error!($message)
            }
            _ => {}
        }
    };
}
