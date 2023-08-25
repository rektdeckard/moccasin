use crate::{config::Config, db::Database};
use anyhow::Result;
use rss::{Channel, Item};
use std::error;
use std::process::{Child, Command, Stdio};
use tui::widgets::{ListState, ScrollDirection, ScrollbarState};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    pub config: Config,
    pub db: Database,
    pub running: bool,
    pub feeds: StatefulList<Channel>,
    pub items: StatefulList<Item>,
    pub active_view: ActiveView,
    pub detail_scroll: ScrollbarState,
    pub detail_scroll_index: u16,
}

impl App {
    pub async fn init(config: Config) -> Result<Self> {
        let urls = config.feed_urls();
        let mut items: Vec<Channel> = Vec::with_capacity(urls.len());

        for url in urls {
            let res = reqwest::get(url).await?.bytes().await?;
            if let Ok(channel) = Channel::read_from(&res[..]) {
                items.push(channel);
            }
        }

        let db = Database::init(&config).await?;

        Ok(Self {
            config,
            db,
            running: true,
            feeds: StatefulList::<Channel>::with_items(items),
            items: StatefulList::<Item>::default(),
            active_view: ActiveView::Feeds,
            detail_scroll: ScrollbarState::default(),
            detail_scroll_index: 0,
        })
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

    pub fn next(&mut self) {
        match self.active_view {
            ActiveView::Feeds => {
                self.next_feed();
            }
            ActiveView::Items => {
                self.reset_detail_scroll();
                self.next_item();
            }
            ActiveView::Detail => {
                self.detail_scroll_index = self.detail_scroll_index.saturating_add(1);
                self.detail_scroll.next();
            }
        }
    }

    pub fn prev(&mut self) {
        match self.active_view {
            ActiveView::Feeds => {
                self.reset_detail_scroll();
                self.prev_feed();
            }
            ActiveView::Items => {
                self.reset_detail_scroll();
                self.prev_item();
            }
            ActiveView::Detail => {
                self.detail_scroll_index = self.detail_scroll_index.saturating_sub(1);
                self.detail_scroll.prev();
            }
        }
    }

    pub fn enter(&mut self) {
        match self.active_view {
            ActiveView::Feeds => {
                if let Some(feed) = self.current_feed() {
                    let link = feed.link();
                    let _ = App::open_link(link);
                }
            }
            ActiveView::Items => {
                if let Some(item) = self.current_item() {
                    if let Some(link) = item.link() {
                        let _ = App::open_link(link);
                    }
                }
            }
            _ => {}
        }
    }

    fn reset_detail_scroll(&mut self) {
        self.detail_scroll_index = 0;
        self.detail_scroll = self.detail_scroll.position(0);
    }

    fn open_link(link: &str) -> Option<Child> {
        let null = Stdio::null();
        if cfg!(target_os = "windows") {
            Command::new("rundll32")
                .args(["url.dll,FileProtocolHandler", link])
                .stdout(null)
                .spawn()
                .ok()
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg(link).stdout(null).spawn().ok()
        } else if cfg!(target_os = "linux") {
            Command::new("xdg-open").arg(link).stdout(null).spawn().ok()
        } else {
            None
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
