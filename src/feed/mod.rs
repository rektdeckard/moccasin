use anyhow;
use chrono::DateTime;
use rss::{Channel, Item as ChannelItem};
use serde::{Deserialize, Serialize};
use std::io::BufRead;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feed {
    title: String,
    description: String,
    categories: Vec<Category>,
    link: String,
    ttl: Option<String>,
    items: Vec<Item>,
    pub_date: Option<String>,
    last_fetched: Option<String>,
}

impl Feed {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
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

    pub fn read_from<R: BufRead>(reader: R) -> anyhow::Result<Feed> {
        let channel = Channel::read_from(reader)?;
        Ok(channel.into())
    }
}

impl From<Channel> for Feed {
    fn from(value: Channel) -> Self {
        Self {
            title: value.title.clone(),
            description: value.description.clone(),
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
            items: value.items.iter().map(Item::from).collect(),
            pub_date: value
                .pub_date
                .and_then(|s| DateTime::parse_from_rfc2822(&s).ok())
                .and_then(|s| Some(DateTime::to_rfc2822(&s))),
            last_fetched: None,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Item {
    title: Option<String>,
    author: Option<String>,
    content: Option<String>,
    description: Option<String>,
    categories: Vec<Category>,
    link: Option<String>,
    pub_date: Option<String>,
}

impl Item {
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
        self.description.as_deref()
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
}

impl From<&ChannelItem> for Item {
    fn from(value: &ChannelItem) -> Self {
        // todo!()
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

        Self {
            title: value.title.clone(),
            author,
            content: value.content.clone(),
            description: value.description.clone(),
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub domain: Option<String>,
}
