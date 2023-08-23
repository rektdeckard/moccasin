use anyhow;
use rss::Channel;
use std::io::BufRead;

#[derive(Debug, Default)]
pub struct Feed {}

impl Feed {
    pub fn read_from<R: BufRead>(reader: R) -> anyhow::Result<Feed> {
        let channel = Channel::read_from(reader)?;
        Ok(channel.into())
    }
}

impl From<Channel> for Feed {
    fn from(value: Channel) -> Self {
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct Item {}
