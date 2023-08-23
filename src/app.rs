use std::error;

use crate::feed;
use rss::{Channel, Item};
use tui::widgets::ListState;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub feeds: StatefulList<Channel>,
    pub items: StatefulList<Item>,
    pub active_view: ActiveView,
}

impl Default for App {
    fn default() -> Self {
        const ITEMS: [&'static str; 16] = [
            "https://100r.co/links/rss.xml",
            "https://abeautifulmess.com/feed",
            "https://ayearofreadingtheworld.com/feed/",
            "http://feeds.bbci.co.uk/news/world/rss.xml",
            "https://medium.com/feed/better-programming",
            "https://overreacted.io/rss.xml",
            "https://feeds.feedburner.com/dancarlin/history?format=xml",
            "http://feeds.arstechnica.com/arstechnica/index/",
            "https://hackaday.com/blog/feed/",
            "https://medium.com/feed/hackernoon",
            "https://blog.jetbrains.com/feed",
            "https://www.newinbooks.com/feed/",
            "https://www.theonion.com/rss",
            "https://tty1.blog/feed/",
            "https://www.wired.com/feed/rss",
            "https://xkcd.com/rss.xml",
        ];

        let items = ITEMS
            .iter()
            .filter_map(|url| {
                let res = reqwest::blocking::get(*url).unwrap().bytes().unwrap();
                Channel::read_from(&res[..]).ok()
            })
            .collect::<Vec<Channel>>();

        Self {
            running: true,
            feeds: StatefulList::<Channel>::with_items(items),
            items: StatefulList::<Item>::default(),
            active_view: ActiveView::Feeds,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn current_feed(&self) -> Option<&Channel> {
        self.feeds
            .state
            .selected()
            .and_then(|i| self.feeds.items().get(i))
    }

    pub fn current_item(&self) -> Option<&Item> {
        self.items
            .state
            .selected()
            .and_then(|i| self.items.items().get(i))
    }

    pub fn next_feed(&mut self) {
        self.items.state.select(Some(0));
        self.feeds.next();

        if let Some(channel) = self.current_feed() {
            self.items.items = channel.items().into();
        }
    }

    pub fn prev_feed(&mut self) {
        self.items.state.select(Some(0));
        self.feeds.previous();

        if let Some(channel) = self.current_feed() {
            self.items.items = channel.items().into();
        }
    }

    pub fn next_item(&mut self) {
        self.items.next();
    }

    pub fn prev_item(&mut self) {
        self.items.previous();
    }

    pub fn next_view(&mut self) {
        if let Some(next_view) = match self.active_view {
            ActiveView::Feeds => Some(ActiveView::Items),
            ActiveView::Items => Some(ActiveView::Detail),
            ActiveView::Detail => None,
        } {
            self.active_view = next_view;
        }
    }

    pub fn prev_view(&mut self) {
        if let Some(next_view) = match self.active_view {
            ActiveView::Feeds => None,
            ActiveView::Items => Some(ActiveView::Feeds),
            ActiveView::Detail => Some(ActiveView::Items),
        } {
            self.active_view = next_view;
        }
    }

    pub fn next_element(&mut self) {
        match self.active_view {
            ActiveView::Feeds => self.next_feed(),
            ActiveView::Items => self.next_item(),
            _ => {}
        }
    }

    pub fn prev_element(&mut self) {
        match self.active_view {
            ActiveView::Feeds => self.prev_feed(),
            ActiveView::Items => self.prev_item(),
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ActiveView {
    Feeds,
    Items,
    Detail,
}

#[derive(Default, Debug)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        if self.items.len() == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.items.len() == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i <= 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn items(&self) -> &Vec<T> {
        &self.items
    }
}
