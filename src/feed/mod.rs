use anyhow;
use chrono::prelude::*;
use ducktype::DuckType;
use rss::{Channel, Item as ChannelItem};
use serde::{Deserialize, Serialize};
use std::io::BufRead;

mod html;

#[derive(Clone, Debug, Serialize, Deserialize, DuckType)]
pub struct Feed {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) categories: Vec<Category>,
    pub(crate) url: String,
    pub(crate) link: String,
    pub(crate) ttl: Option<String>,
    #[serde(skip)]
    pub(crate) items: Vec<Item>,
    pub(crate) pub_date: Option<String>,
    pub(crate) last_fetched: Option<String>,
}

impl Feed {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn link(&self) -> &str {
        &self.link
    }

    pub fn ttl(&self) -> Option<&str> {
        self.ttl.as_deref()
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn pub_date(&self) -> Option<&str> {
        self.pub_date.as_deref()
    }

    pub fn last_fetched(&self) -> Option<&str> {
        self.last_fetched.as_deref()
    }

    pub fn with_items(mut self, items: Vec<Item>) -> Self {
        self.items = items;
        self
    }

    fn from_channel_with_url(value: Channel, url: String) -> Self {
        let id = value
            .dublin_core_ext()
            .and_then(|dc| {
                if !dc.identifiers().is_empty() {
                    Some(dc.identifiers().concat())
                } else {
                    None
                }
            })
            .unwrap_or(value.link().to_owned());

        Self {
            title: value.title.clone(),
            description: value.description.clone(),
            url: url,
            link: value.link.clone(),
            ttl: value.ttl.clone(),
            categories: value
                .categories
                .iter()
                .map(|c| Category {
                    name: c.name.clone(),
                    domain: c.domain.clone(),
                })
                .collect(),
            items: value
                .items
                .iter()
                .map(|i| Item::with_parent(id.as_str(), i))
                .collect(),
            pub_date: value
                .pub_date
                .and_then(|s| DateTime::parse_from_rfc2822(&s).ok())
                .and_then(|s| Some(DateTime::to_rfc2822(&s))),
            last_fetched: None,
            id,
        }
    }

    pub fn read_from<R: BufRead>(reader: R, url: String) -> anyhow::Result<Feed> {
        let channel = Channel::read_from(reader)?;
        let mut feed = Feed::from_channel_with_url(channel, url);
        feed.last_fetched = Some(Local::now().to_rfc2822());
        Ok(feed)
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Item {
    id: String,
    feed_id: String,
    title: Option<String>,
    author: Option<String>,
    content: Option<String>,
    description: Option<String>,
    text_description: Option<String>,
    categories: Vec<Category>,
    link: Option<String>,
    pub_date: Option<String>,
}

impl Item {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn feed_id(&self) -> &str {
        &self.feed_id
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        if self.text_description.is_some() {
            self.text_description.as_deref()
        } else {
            self.description.as_deref()
        }
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }

    pub fn pub_date(&self) -> Option<&str> {
        self.pub_date.as_deref()
    }

    fn with_parent(feed_id: &str, value: &ChannelItem) -> Self {
        let id = value
            .guid()
            .and_then(|g| {
                if g.is_permalink() {
                    Some(g.value.clone())
                } else {
                    None
                }
            })
            .or(value.dublin_core_ext().and_then(|dc| {
                if !dc.identifiers().is_empty() {
                    Some(dc.identifiers().concat())
                } else {
                    None
                }
            }))
            .unwrap_or(format!(
                "{}:{}",
                feed_id,
                value.link().unwrap_or(
                    value
                        .title()
                        .expect("Holy cow there is nothing to identify this post item at all")
                )
            ));

        let author = value
            .author()
            .and_then(|s| Some(s.to_owned()))
            .or(value
                .itunes_ext()
                .and_then(|it| it.author().and_then(|auth| Some(auth.to_owned()))))
            .or(value.dublin_core_ext().and_then(|dc| {
                let creators = dc.creators().join(", ");
                if creators.is_empty() {
                    None
                } else {
                    Some(creators)
                }
            }));

        let text_description = if let Some(d) = value.description() {
            html::parse_html(&d).ok()
        } else {
            None
        };

        Self {
            id,
            feed_id: feed_id.to_owned(),
            title: value.title.clone(),
            author,
            content: value.content.clone(),
            description: value.description.clone(),
            text_description,
            categories: value
                .categories
                .iter()
                .map(|c| Category {
                    name: c.name.clone(),
                    domain: c.domain.clone(),
                })
                .collect(),
            link: value.link.clone(),
            pub_date: value.pub_date.clone(),
        }
    }
}

// impl From<&ChannelItem> for Item {
//     fn from(value: &ChannelItem) -> Self {
//         let author = value
//             .author()
//             .and_then(|s| Some(s.to_owned()))
//             .or(value
//                 .itunes_ext()
//                 .and_then(|it| it.author().and_then(|auth| Some(auth.to_owned()))))
//             .or(value.dublin_core_ext().and_then(|dc| {
//                 let creators = dc.creators().join(", ");
//                 if creators.is_empty() {
//                     None
//                 } else {
//                     Some(creators)
//                 }
//             }));

//         let text_description = if let Some(d) = value.description() {
//             html::parse_html(&d).ok()
//         } else {
//             None
//         };

//         Self {
//             title: value.title.clone(),
//             author,
//             content: value.content.clone(),
//             text_content: None,
//             description: value.description.clone(),
//             text_description,
//             categories: value
//                 .categories
//                 .iter()
//                 .map(|c| Category {
//                     name: c.name.clone(),
//                     domain: c.domain.clone(),
//                 })
//                 .collect(),
//             link: value.link.clone(),
//             pub_date: value.pub_date.clone(),
//         }
//     }
// }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub domain: Option<String>,
}
