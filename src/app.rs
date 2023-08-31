use crate::config::Config;
use crate::feed::{Feed, Item};
use crate::repo::{Repository, RepositoryEvent};
use anyhow::Result;
use clap::Parser;
use std::error;
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::task::Poll;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tui::widgets::{ListState, ScrollbarState};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Set a custom config file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Set a custom theme, either built-in or a path to a theme file
    #[arg(short = 's', long)]
    pub color_scheme: Option<String>,

    /// Set a custom refresh rate in seconds
    #[arg(short, long)]
    pub interval: Option<u64>,

    /// Set a custom request timeout in seconds
    #[arg(short, long)]
    pub timeout: Option<u64>,

    /// Do not cache feeds in local file-backed database
    #[arg(short, long)]
    pub no_cache: bool,
}

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub enum Status {
    Loading(usize, usize),
    Errored(String),
    Done,
}

#[derive(Debug)]
pub enum ConsoleCommand {
    AddFeed(String),
    DeleteFeed(Option<String>),
    Search(String),
}

#[derive(Debug)]
pub enum ConsoleCommandError {
    BadCommand,
    BadArgument,
}

impl FromStr for ConsoleCommand {
    type Err = ConsoleCommandError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts = s.split_whitespace().collect::<Vec<_>>();

        if let Some(cmd) = parts.get(0) {
            match *cmd {
                ":a" | ":add" => match parts.get(1) {
                    Some(url) => Ok(ConsoleCommand::AddFeed(url.to_string())),
                    None => Err(ConsoleCommandError::BadArgument),
                },
                ":s" | ":search" => {
                    let query = parts.iter().skip(1).copied().collect::<String>();
                    if query.is_empty() {
                        Err(ConsoleCommandError::BadArgument)
                    } else {
                        Ok(ConsoleCommand::Search(query))
                    }
                }
                ":d" | ":delete" => match parts.get(1) {
                    Some(url) => Ok(ConsoleCommand::DeleteFeed(Some(url.to_string()))),
                    None => Ok(ConsoleCommand::DeleteFeed(None)),
                },
                _ => Err(ConsoleCommandError::BadCommand),
            }
        } else {
            Err(ConsoleCommandError::BadCommand)
        }
    }
}

/// Application.
#[derive(Debug)]
pub struct App {
    pub config: Config,
    pub repo: Repository,
    pub running: bool,
    pub active_view: View,
    pub active_tab: Tab,
    pub feeds: StatefulList<Feed>,
    pub feeds_scroll: ScrollbarState,
    pub items: StatefulList<Item>,
    pub items_scroll: ScrollbarState,
    pub detail_scroll: ScrollbarState,
    pub detail_scroll_index: u16,
    pub show_keybinds: bool,
    pub status: Status,
    pub command_state: InputState,
    dimensions: (u16, u16),
    repo_rx: UnboundedReceiver<RepositoryEvent>,
}

impl App {
    pub fn init(dimensions: (u16, u16)) -> Result<Self> {
        let args = Args::parse();
        let config = Config::new(args)?;

        let (tx, rx) = mpsc::unbounded_channel::<RepositoryEvent>();
        let mut repo = Repository::init(&config, tx)?;

        let items = repo.read_all(&config).unwrap_or_default();
        let feeds_count = items.len() as u16;

        Ok(Self {
            config,
            repo,
            running: true,
            dimensions,
            active_view: View::MainList,
            active_tab: Tab::Browse,
            feeds: StatefulList::<Feed>::with_items(items),
            feeds_scroll: ScrollbarState::default().content_length(feeds_count),
            items: StatefulList::<Item>::default(),
            items_scroll: ScrollbarState::default(),
            detail_scroll: ScrollbarState::default(),
            detail_scroll_index: 0,
            status: Status::Done,
            show_keybinds: false,
            command_state: InputState::new(),
            repo_rx: rx,
        })
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        self.repo.tick(&self.config);

        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        loop {
            match self.repo_rx.poll_recv(&mut cx) {
                Poll::Ready(m) => match m {
                    Some(RepositoryEvent::Requesting(amount)) => {
                        self.status = match self.status {
                            Status::Loading(curr, total) => Status::Loading(curr, total + amount),
                            _ => Status::Loading(0, amount),
                        };
                    }
                    Some(RepositoryEvent::Requested(counts)) => {
                        let counts = match self.status {
                            Status::Loading(current, total) => ((current + 1).min(total), total),
                            _ => counts,
                        };
                        self.status = Status::Loading(counts.0, counts.1);
                    }
                    Some(RepositoryEvent::RetrievedAll(feeds)) => {
                        self.set_feeds(feeds);
                        self.status = Status::Done;
                        break;
                    }
                    Some(RepositoryEvent::RetrievedOne(feed)) => {
                        match self
                            .feeds
                            .items
                            .iter()
                            .enumerate()
                            .find(|(_, f)| f.link() == feed.link())
                        {
                            Some((i, f)) => {
                                self.feeds.items[i] = f.clone();
                            }
                            None => {
                                self.feeds.items.push(feed);
                            }
                        }

                        match self.status {
                            Status::Loading(_, _) => {
                                self.status = Status::Done;
                            }
                            _ => {}
                        }

                        break;
                    }
                    Some(RepositoryEvent::Errored) => {
                        self.status = Status::Errored("database transaction failed".into());
                        break;
                    }
                    Some(RepositoryEvent::Refresh) => {}
                    Some(RepositoryEvent::Aborted) => {
                        self.status = Status::Done;
                        break;
                    }
                    None => {
                        break;
                    }
                },
                Poll::Pending => {
                    break;
                }
            }
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn set_dimensions(&mut self, dimensions: (u16, u16)) {
        self.dimensions = dimensions;
    }

    pub fn should_render_feeds_scroll(&self) -> bool {
        self.feeds.items().len() as u16 > self.dimensions.1 - 8
    }

    pub fn should_render_items_scroll(&self) -> bool {
        self.items.items().len() as u16 > self.dimensions.1 - 8
    }

    pub fn should_render_detail_scroll(&self) -> bool {
        // TODO
        false
    }

    pub fn should_render_console(&self) -> bool {
        self.command_state.show_input
    }

    pub fn current_feed(&self) -> Option<&Feed> {
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
        self.feeds.next();
        self.feeds_scroll = self.feeds_scroll.position(
            self.feeds
                .state
                .selected()
                .unwrap_or(self.feeds.state.offset()) as u16,
        );

        if let Some(channel) = self.current_feed() {
            self.items.items = channel.items().into();
            self.items_scroll = self
                .items_scroll
                .content_length(self.items.items.len() as u16);
        }
    }

    pub fn prev_feed(&mut self) {
        self.feeds.previous();
        self.feeds_scroll = self.feeds_scroll.position(
            self.feeds
                .state
                .selected()
                .unwrap_or(self.feeds.state.offset()) as u16,
        );

        if let Some(channel) = self.current_feed() {
            self.items.items = channel.items().into();
            self.items_scroll = self
                .items_scroll
                .content_length(self.items.items.len() as u16);
        }
    }

    pub fn next_item(&mut self) {
        self.items.next();
        self.items_scroll = self.items_scroll.position(
            self.items
                .state
                .selected()
                .unwrap_or(self.items.state.offset()) as u16,
        );
    }

    pub fn prev_item(&mut self) {
        self.items.previous();
        self.items_scroll = self.items_scroll.position(
            self.items
                .state
                .selected()
                .unwrap_or(self.items.state.offset()) as u16,
        );
    }

    pub fn next_view(&mut self, wrap: bool) {
        let has_current_feed = self.current_feed().is_some();
        let has_current_item = self.current_item().is_some();

        if !has_current_feed {
            self.active_view = View::MainList;
            return;
        }

        if let Some(next_view) = match self.active_view {
            View::MainList => {
                if self.items.state.selected().is_none() {
                    self.next_item();
                }
                Some(View::SubList)
            }
            View::SubList => {
                if has_current_item {
                    Some(View::Detail)
                } else if wrap {
                    Some(View::MainList)
                } else {
                    None
                }
            }
            View::Detail => {
                if wrap {
                    Some(View::MainList)
                } else {
                    None
                }
            }
        } {
            self.active_view = next_view;
        }
    }

    pub fn prev_view(&mut self, wrap: bool) {
        let has_current_feed = self.current_feed().is_some();
        let has_current_item = self.current_item().is_some();

        if !has_current_feed {
            self.active_view = View::MainList;
            return;
        }

        if let Some(next_view) = match self.active_view {
            View::MainList => {
                if wrap && has_current_item {
                    Some(View::Detail)
                } else if wrap {
                    Some(View::SubList)
                } else {
                    None
                }
            }
            View::SubList => Some(View::MainList),
            View::Detail => Some(View::SubList),
        } {
            self.active_view = next_view;
        }
    }

    pub fn next(&mut self) {
        match self.active_view {
            View::MainList => {
                self.reset_items_scroll();
                self.reset_detail_scroll();
                self.next_feed();
            }
            View::SubList => {
                self.reset_detail_scroll();
                self.next_item();
            }
            View::Detail => {
                self.detail_scroll_index = self.detail_scroll_index.saturating_add(1);
                self.detail_scroll.next();
            }
        }
    }

    pub fn prev(&mut self) {
        match self.active_view {
            View::MainList => {
                self.reset_items_scroll();
                self.reset_detail_scroll();
                self.prev_feed();
            }
            View::SubList => {
                self.reset_detail_scroll();
                self.prev_item();
            }
            View::Detail => {
                self.detail_scroll_index = self.detail_scroll_index.saturating_sub(1);
                self.detail_scroll.prev();
            }
        }
    }

    pub fn next_tab(&mut self) {
        let next_tab = match self.active_tab {
            Tab::Browse => Tab::Favorites,
            Tab::Favorites => Tab::Tags,
            Tab::Tags => Tab::Browse,
        };

        self.active_tab = next_tab;
    }

    pub fn prev_tab(&mut self) {
        let prev_tab = match self.active_tab {
            Tab::Browse => Tab::Tags,
            Tab::Favorites => Tab::Browse,
            Tab::Tags => Tab::Favorites,
        };

        self.active_tab = prev_tab;
    }

    pub fn set_tab(&mut self, index: usize) {
        self.active_tab = Tab::from(index);
    }

    pub fn unselect(&mut self) {
        if self.current_item().is_some() {
            self.items.state.select(None);
        } else {
            self.feeds.state.select(None);
        }
        self.prev_view(false);
    }

    pub fn open(&mut self) {
        match self.active_view {
            View::MainList => {
                if let Some(feed) = self.current_feed() {
                    let link = feed.link();
                    let _ = App::open_link(link);
                }
            }
            View::SubList => {
                if let Some(item) = self.current_item() {
                    if let Some(link) = item.link() {
                        let _ = App::open_link(link);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn open_config(&self) -> Option<Child> {
        if let Some(cfg_path) = self.config.config_file_path().as_path().to_str() {
            Self::open_link(cfg_path)
        } else {
            None
        }
    }

    pub fn refresh_all(&mut self) {
        self.repo.refresh_all(&self.config)
    }

    pub fn toggle_keybinds(&mut self) {
        self.show_keybinds = !self.show_keybinds;
    }

    pub fn toggle_console(&mut self, cmd: Option<&str>) {
        if let Some(cmd) = cmd {
            self.command_state.input = cmd.into();
            self.command_state.cursor_position = self.clamp_cursor(cmd.len());
        } else {
            self.command_state.input.clear();
            self.reset_cursor();
        }
        self.command_state.show_input = !self.command_state.show_input;
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.command_state.cursor_position.saturating_sub(1);
        self.command_state.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.command_state.cursor_position.saturating_add(1);
        self.command_state.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        self.command_state
            .input
            .insert(self.command_state.cursor_position, new_char);
        self.move_cursor_right();
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.command_state.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.command_state.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self
                .command_state
                .input
                .chars()
                .take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.command_state.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.command_state.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.command_state.input.len())
    }

    fn reset_cursor(&mut self) {
        self.command_state.cursor_position = 0;
    }

    pub fn submit_command(&mut self) {
        match self.command_state.input.parse::<ConsoleCommand>() {
            Ok(ConsoleCommand::AddFeed(url)) => {
                self.config.add_feed_url(&url);
                self.repo.add_feed_url(&url, &self.config);
            }
            Ok(ConsoleCommand::DeleteFeed(maybe_url)) => {
                if let Some(url) =
                    maybe_url.or(self.current_feed().and_then(|f| Some(f.link().into())))
                {
                    self.config.remove_feed_url(&url);
                    self.repo.remove_feed_url(&url);

                    // TODO: refactor, this is so bad
                    self.feeds.items.retain(|u| u.link() != url);
                    self.feeds.state.select(None);
                    self.reset_items_scroll();
                    self.reset_detail_scroll();
                }
            }
            Ok(ConsoleCommand::Search(_)) => todo!(),
            _ => self.status = Status::Errored("unrecognized command".into()),
        }

        self.command_state.input.clear();
        self.reset_cursor();
        self.toggle_console(None);
    }

    fn set_feeds(&mut self, feeds: Vec<Feed>) {
        self.feeds.items = feeds;
        // self.items.state.select(None);
        // self.active_view = ActiveView::Feeds;
    }

    fn reset_items_scroll(&mut self) {
        self.items.state.select(None);
        self.items_scroll = self.items_scroll.position(0);
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
pub enum View {
    MainList,
    SubList,
    Detail,
}

#[derive(Debug, PartialEq)]
pub enum Tab {
    Browse,
    Favorites,
    Tags,
}

impl ToString for Tab {
    fn to_string(&self) -> String {
        match self {
            Self::Browse => "Browse".into(),
            Self::Favorites => "Favorites".into(),
            Self::Tags => "Tags".into(),
        }
    }
}

impl Tab {
    pub fn index_of(&self) -> usize {
        match self {
            Self::Browse => 0,
            Self::Favorites => 1,
            Self::Tags => 2,
        }
    }
}

impl From<usize> for Tab {
    fn from(value: usize) -> Self {
        match value {
            1 => Tab::Favorites,
            2 => Tab::Tags,
            _ => Tab::Browse,
        }
    }
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

    #[allow(dead_code)]
    fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn items(&self) -> &Vec<T> {
        &self.items
    }
}

#[derive(Debug)]
pub struct InputState {
    pub input: String,
    pub cursor_position: usize,
    show_input: bool,
}

impl InputState {
    fn new() -> Self {
        Self {
            input: String::new(),
            cursor_position: 0,
            show_input: false,
        }
    }
}
