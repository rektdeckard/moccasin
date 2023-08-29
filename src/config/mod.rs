use crate::app::Args;
use anyhow::Result;
use directories::ProjectDirs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fs, fs::File};
use toml::{toml, Table, Value};
use toml_edit::{value, Document};

mod theme;

const DEFAULT_CONFIG_FILE: &'static str = "tabss.toml";
const DEFAULT_DB_FILE: &'static str = "tabss.db";
const DEFAULT_REFRESH_INTERVAL: u64 = 300;
const DEFAULT_REFRESH_TIMEOUT: u64 = 5;

#[derive(Debug, Default, Clone)]
pub struct Config {
    file_path: PathBuf,
    dir_path: PathBuf,
    feed_urls: Vec<String>,
    sort_order: SortOrder,
    refresh_interval: u64,
    refresh_timeout: u64,
    theme: theme::Theme,
}

#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    #[default]
    Az,
    Za,
    Newest,
    Oldest,
    Custom,
}

#[derive(Debug)]
pub struct SortOrderError;

impl FromStr for SortOrder {
    type Err = SortOrderError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "az" => Ok(SortOrder::Az),
            "za" => Ok(SortOrder::Za),
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
            let dir_path = ProjectDirs::from("com", "rektsoft", "tabss")
                .unwrap()
                .config_local_dir()
                .to_owned();
            let file_path = dir_path.join(DEFAULT_CONFIG_FILE).to_owned();
            (dir_path, file_path)
        };

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

    pub fn feed_urls(&self) -> &Vec<String> {
        &self.feed_urls
    }

    pub fn sort_order(&self) -> &SortOrder {
        &self.sort_order
    }

    pub fn refresh_interval(&self) -> u64 {
        self.refresh_interval
    }

    pub fn refresh_timeout(&self) -> u64 {
        self.refresh_timeout
    }

    pub fn write_config(&self) -> Result<()> {
        todo!();
        // let toml = fs::read_to_string(&self.file_path)?;
        // let toml = toml.parse::<Table>()?;
        // match toml.get("sources") {
        //     Some(Value::Table(sources)) => match sources.get("feeds") {
        //         Some(Value::Array(feeds)) => {
        //             *feeds = self.feed_urls.into();
        //         },
        //         _ => {}
        //     }
        //     _ => {}
        // }
        // let feeds = sources.get("feeds")?;
        // let file = fs::write(self.file_path, contents);
    }

    pub fn add_feed_url(&mut self, url: &str) -> Result<()> {
        self.feed_urls.push(url.into());
        // self.write_config()
        Ok(())
    }

    fn read_from_toml(args: Args, dir_path: PathBuf, file_path: PathBuf) -> Result<Self> {
        let toml = fs::read_to_string(&file_path)?;
        let table = toml.parse::<Table>()?;
        let feeds: Vec<String> = match table.get("sources") {
            Some(Value::Table(sources)) => match sources.get("feeds") {
                Some(Value::Array(els)) => els
                    .iter()
                    .filter_map(|v| v.as_str().and_then(|v| Some(v.to_owned())))
                    .collect(),
                Some(_) => {
                    panic!("unexpected config entry for [sources].feeds")
                }
                _ => vec![],
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
            .or_else(|| {
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
            .or_else(|| {
                preferences.and_then(|prefs| {
                    prefs.get("refresh_timeout").and_then(|i| match i {
                        Value::Integer(i) => Some(*i as u64),
                        _ => None,
                    })
                })
            })
            .unwrap_or(DEFAULT_REFRESH_INTERVAL);

        Ok(Self {
            file_path,
            dir_path,
            feed_urls: feeds,
            sort_order,
            refresh_interval,
            refresh_timeout,
            theme,
        })
    }

    fn create_initialized(args: Args, dir_path: PathBuf, file_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&dir_path)?;
        let cfg_path = Path::new(dir_path.as_path()).join(DEFAULT_CONFIG_FILE);
        let mut file = File::create(&cfg_path)?;
        let stub = include_str!("tabss.toml").parse::<Table>()?;
        let feed_urls = stub["sources"]["feeds"]
            .as_array()
            .expect("parse default feeds")
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<Vec<_>>();
        file.write(&stub.to_string().as_bytes())?;

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
