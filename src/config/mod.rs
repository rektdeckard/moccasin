use crate::app::Args;
use anyhow::Result;
use directories::ProjectDirs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, fs::File};
use toml::{toml, Table, Value};

mod theme;

const DEFAULT_CONFIG_FILE: &'static str = "tabss.toml";
const DEFAULT_DB_FILE: &'static str = "tabss.db";
const DEFAULT_REFRESH_INTERVAL: u64 = 300;

#[derive(Debug, Default)]
pub struct Config {
    file_path: PathBuf,
    dir_path: PathBuf,
    feed_urls: Vec<String>,
    refresh_interval: u64,
    theme: theme::Theme,
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

    pub fn refresh_interval(&self) -> u64 {
        self.refresh_interval
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
        let theme = preferences
            .and_then(|prefs| {
                prefs
                    .get("color_scheme")
                    .and_then(|scheme| theme::Theme::try_from(scheme).ok())
            })
            .unwrap_or_default();

        Ok(Self {
            file_path,
            dir_path,
            feed_urls: feeds,
            refresh_interval: args.interval.unwrap_or(DEFAULT_REFRESH_INTERVAL),
            theme,
        })
    }

    fn create_initialized(args: Args, dir_path: PathBuf, file_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&dir_path)?;
        let cfg_path = Path::new(dir_path.as_path()).join(DEFAULT_CONFIG_FILE);
        let mut file = File::create(&cfg_path)?;
        let stub = toml! {
            [sources]
            feeds = []

            [preferences]
            color_scheme = "default"
            refresh_interval = DEFAULT_REFRESH_INTERVAL
        };
        file.write(toml::to_string_pretty(&stub).unwrap().as_bytes())?;

        // TODO: load theme from args if present

        Ok(Self {
            dir_path: dir_path.to_owned(),
            file_path: file_path.to_owned(),
            refresh_interval: args.interval.unwrap_or(DEFAULT_REFRESH_INTERVAL),
            ..Default::default()
        })
    }
}
