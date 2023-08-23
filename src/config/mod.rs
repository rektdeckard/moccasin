use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{collections::HashMap, fs, fs::File, path::Path};
use toml::{Table, Value};

mod theme;

const CONFIG_FILE_VARIANTS: [&'static str; 3] = ["tabss.toml", "tabss.yaml", "tabss.yml"];

#[derive(Debug, Clone)]
enum ConfigVariant {
    Toml(String),
    Yaml(String),
}

impl Default for ConfigVariant {
    fn default() -> Self {
        let path = ProjectDirs::from("com", "rektsoft", "tabss")
            .unwrap()
            .config_dir()
            .join(CONFIG_FILE_VARIANTS[0])
            .as_path()
            .to_str()
            .unwrap()
            .to_owned();
        ConfigVariant::Toml(path)
    }
}

#[derive(Debug, Default)]
pub struct Config {
    variant: ConfigVariant,
    feed_urls: Vec<String>,
    theme: theme::Theme,
}

impl Config {
    pub fn config_path(&self) -> String {
        match &self.variant {
            ConfigVariant::Toml(s) => s.clone(),
            ConfigVariant::Yaml(s) => s.clone(),
        }
    }

    pub fn theme(&self) -> &theme::Theme {
        &self.theme
    }

    pub fn feed_urls(&self) -> &Vec<String> {
        &self.feed_urls
    }

    fn find_config_file(base_path: &Path) -> Option<ConfigVariant> {
        for (i, v) in CONFIG_FILE_VARIANTS.iter().enumerate() {
            let path = base_path.join(v);
            if path.exists() {
                if i == 0 {
                    return Some(ConfigVariant::Toml(path.to_str().unwrap().to_owned()));
                } else {
                    return Some(ConfigVariant::Yaml(path.to_str().unwrap().to_owned()));
                }
            }
        }

        None
    }

    pub fn read_from_path(path: Option<&str>) -> Result<Self> {
        let default_dir = ProjectDirs::from("com", "rektsoft", "tabss").unwrap();
        let cfg_dir = path.map_or(default_dir.config_dir(), Path::new);

        if let Some(variant) = Config::find_config_file(cfg_dir) {
            // let theme = fs::read_to_string(cfg_dir.join("themes/Chalk.light.yml"))?;
            // let color_scheme = colorscheme::ColorScheme::from_yaml(&theme)?;
            let color_scheme = theme::Theme::jungle();

            match &variant {
                ConfigVariant::Toml(cfg_path) => {
                    let toml = fs::read_to_string(&cfg_path)?;
                    let table = toml.parse::<Table>()?;
                    let feeds: Vec<String> = match table.get("data") {
                        Some(Value::Table(data)) => match data.get("feeds") {
                            Some(Value::Array(els)) => els
                                .iter()
                                .filter_map(|v| v.as_str().and_then(|v| Some(v.to_owned())))
                                .collect(),
                            Some(_) => {
                                panic!("unexpected config.toml value for 'data.feeds'")
                            }
                            _ => vec![],
                        },
                        _ => panic!("unexpected config.toml value for [data]"),
                    };

                    Ok(Self {
                        variant: variant.clone(),
                        feed_urls: feeds,
                        theme: color_scheme,
                    })
                }
                ConfigVariant::Yaml(cfg_path) => {
                    #[derive(Debug, PartialEq, Serialize, Deserialize)]
                    struct Yaml {
                        data: HashMap<String, Vec<String>>,
                    }

                    let yaml = fs::read_to_string(&cfg_path)?;
                    let table = serde_yaml::from_str::<Yaml>(&yaml)?;
                    match table.data.get("feeds") {
                        Some(feeds) => Ok(Self {
                            variant: variant.clone(),
                            feed_urls: feeds.clone(),
                            theme: color_scheme,
                        }),
                        _ => panic!("unexpected config.toml value for 'data.feeds'"),
                    }
                }
            }
        } else {
            fs::create_dir_all(cfg_dir)?;
            let cfg_path = Path::new(cfg_dir).join(CONFIG_FILE_VARIANTS[0]);
            let _ = File::create(&cfg_path)?;

            Ok(Self {
                variant: ConfigVariant::Toml(cfg_path.to_str().unwrap().to_owned()),
                ..Default::default()
            })
        }
    }
}
