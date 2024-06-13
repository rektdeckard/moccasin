use crate::app::Args;
use anyhow::Result;
use directories::ProjectDirs;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fs, fs::File};
use toml::{Table, Value};
use toml_edit::{value, Array, Document};

use log::{info, warn};

mod theme;

const DEFAULT_CONFIG_FILE: &'static str = "moccasin.toml";
const DEFAULT_DB_FILE: &'static str = "moccasin.db";
const DEFAULT_REFRESH_INTERVAL: u64 = 300;
const DEFAULT_REFRESH_TIMEOUT: u64 = 5;

#[derive(Debug, Default, Clone)]
pub struct Config {
    file_path: PathBuf,
    dir_path: PathBuf,
    feed_urls: HashSet<String>,
    sort_order: SortOrder,
    cache_control: CacheControl,
    refresh_interval: u64,
    refresh_timeout: u64,
    theme: theme::Theme,
}

#[derive(Debug, Default, Clone)]
pub enum SortOrder {
    #[default]
    Az,
    Za,
    Unread,
    Newest,
    Oldest,
    Custom,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum CacheControl {
    #[default]
    Always,
    Never,
}

impl From<bool> for CacheControl {
    fn from(value: bool) -> Self {
        if value {
            Self::Always
        } else {
            Self::Never
        }
    }
}

#[derive(Debug)]
pub struct SortOrderError;

impl FromStr for SortOrder {
    type Err = SortOrderError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "a-z" => Ok(SortOrder::Az),
            "z-a" => Ok(SortOrder::Za),
            "newest" => Ok(SortOrder::Newest),
            "oldest" => Ok(SortOrder::Oldest),
            "custom" => Ok(SortOrder::Custom),
            _ => Ok(SortOrder::Az),
        }
    }
}

impl Config {
    pub fn new(args: Args) -> Result<Self> {
        let (dir_path, file_path): (PathBuf, PathBuf) = if let Some(path) = &args.config {
            let file_path = Path::new(&path);
            if !file_path.exists() {
                panic!(
                    "no config file found at '{}'",
                    file_path.to_owned().to_str().unwrap()
                )
            }

            let dir_path = file_path.parent().expect("could not find config directory");
            (dir_path.into(), file_path.into())
        } else {
            let dir_path = ProjectDirs::from("com", "rektsoft", "moccasin")
                .unwrap()
                .config_local_dir()
                .to_owned();
            let file_path = dir_path.join(DEFAULT_CONFIG_FILE).to_owned();
            (dir_path, file_path)
        };

        if cfg!(debug_assertions) {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(dir_path.join("moccasin.log"))
                .expect("could not open file for witing");
            simplelog::WriteLogger::init(
                simplelog::LevelFilter::Info,
                simplelog::Config::default(),
                file,
            )
            .expect("could not initialize logger");
        }

        if file_path.exists() {
            Self::read_from_toml(args, dir_path, file_path)
        } else {
            Self::create_initialized(args, dir_path, file_path)
        }
    }

    pub fn config_dir_path(&self) -> PathBuf {
        Path::new(&self.dir_path).to_owned()
    }

    pub fn config_file_path(&self) -> PathBuf {
        Path::new(&self.file_path).to_owned()
    }

    pub fn db_path(&self) -> PathBuf {
        self.config_dir_path().join(DEFAULT_DB_FILE)
    }

    pub fn themes_path(&self) -> PathBuf {
        self.config_dir_path().join("themes")
    }

    pub fn theme(&self) -> &theme::Theme {
        &self.theme
    }

    pub fn feed_urls(&self) -> &HashSet<String> {
        &self.feed_urls
    }

    pub fn sort_order(&self) -> &SortOrder {
        &self.sort_order
    }

    pub fn should_cache(&self) -> bool {
        self.cache_control == CacheControl::Always
    }

    pub fn refresh_interval(&self) -> u64 {
        self.refresh_interval
    }

    pub fn refresh_timeout(&self) -> u64 {
        self.refresh_timeout
    }

    pub fn write_config(&self) -> Result<()> {
        let toml = fs::read_to_string(&self.file_path)?;
        let mut toml = toml.parse::<Document>()?;

        let mut urls = Array::new();
        for url in self.feed_urls() {
            urls.push_formatted(url.into());
        }
        urls.set_trailing_comma(true);
        toml["sources"]["feeds"] = value(urls);

        let _ = fs::write(&self.file_path, toml.to_string())?;
        Ok(())
    }

    pub fn add_feed_url(&mut self, url: &str) -> Result<()> {
        if !self.feed_urls().contains(url) {
            info!("Adding new feed for {}", url);
            self.feed_urls.insert(url.into());
            self.write_config()?;
        }
        Ok(())
    }

    pub fn remove_feed_url(&mut self, url: &str) -> Result<()> {
        if self.feed_urls().contains(url) {
            info!("Deleting feed for {}", url);
            self.feed_urls.remove(url);
            self.write_config()?;
        }
        Ok(())
    }

    fn read_from_toml(args: Args, dir_path: PathBuf, file_path: PathBuf) -> Result<Self> {
        let toml = fs::read_to_string(&file_path)?;
        let table = toml.parse::<Table>()?;
        let feeds: HashSet<String> = match table.get("sources") {
            Some(Value::Table(sources)) => match sources.get("feeds") {
                Some(Value::Array(els)) => els
                    .iter()
                    .filter_map(|v| v.as_str().and_then(|v| Some(v.to_owned())))
                    .collect(),
                Some(_) => {
                    panic!("unexpected config entry for [sources].feeds")
                }
                _ => HashSet::new(),
            },
            _ => panic!("unexpected config entry for [sources]"),
        };

        let preferences = match table.get("preferences") {
            Some(Value::Table(prefs)) => Some(prefs),
            Some(_) => panic!("invalid config entry for [preferences]"),
            None => None,
        };

        // TODO: load from args if present
        let theme = args
            .color_scheme
            .and_then(|scheme| theme::Theme::from_str(&scheme).ok())
            .or(preferences.and_then(|prefs| {
                prefs
                    .get("color_scheme")
                    .and_then(|scheme| theme::Theme::try_from(scheme).ok())
            }))
            .unwrap_or_default();

        let sort_order: SortOrder = preferences
            .and_then(|prefs| {
                prefs.get("sort_feeds").and_then(|ord| match ord {
                    Value::String(ord) => Some(SortOrder::from_str(ord).unwrap()),
                    _ => None,
                })
            })
            .unwrap_or_default();

        let refresh_interval = args
            .interval
            .or({
                preferences.and_then(|prefs| {
                    prefs.get("refresh_interval").and_then(|i| match i {
                        Value::Integer(i) => Some(*i as u64),
                        _ => None,
                    })
                })
            })
            .unwrap_or(DEFAULT_REFRESH_INTERVAL);

        let refresh_timeout = args
            .timeout
            .or({
                preferences.and_then(|prefs| {
                    prefs.get("refresh_timeout").and_then(|i| match i {
                        Value::Integer(i) => Some(*i as u64),
                        _ => None,
                    })
                })
            })
            .unwrap_or(DEFAULT_REFRESH_TIMEOUT);

        let cache_control = if args.no_cache {
            CacheControl::Never
        } else {
            preferences
                .and_then(|prefs| {
                    prefs.get("cache_feeds").and_then(|i| match i {
                        Value::Boolean(b) => Some(CacheControl::from(*b)),
                        _ => None,
                    })
                })
                .unwrap_or(CacheControl::Always)
        };

        Ok(Self {
            file_path,
            dir_path,
            feed_urls: feeds,
            sort_order,
            cache_control,
            refresh_interval,
            refresh_timeout,
            theme,
        })
    }

    fn create_initialized(args: Args, dir_path: PathBuf, file_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&dir_path)?;
        let cfg_path = Path::new(dir_path.as_path()).join(DEFAULT_CONFIG_FILE);
        let mut file = File::create(&cfg_path)?;
        let toml = include_str!("moccasin.toml");
        let stub = toml.parse::<Table>()?;
        let feed_urls = stub["sources"]["feeds"]
            .as_array()
            .expect("parse default feeds")
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<HashSet<_>>();
        file.write(toml.as_bytes())?;

        // TODO: load theme from args if present
        Ok(Self {
            dir_path: dir_path.to_owned(),
            file_path: file_path.to_owned(),
            feed_urls,
            refresh_interval: args.interval.unwrap_or(DEFAULT_REFRESH_INTERVAL),
            ..Default::default()
        })
    }
}
