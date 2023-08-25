use serde::{Deserialize, Serialize};
use serde_yaml;
use std::error::Error;
use std::fmt;
use toml::{toml, Table, Value};
use tui::style::{Color, Modifier, Style, Stylize};

#[derive(Debug)]
enum ParseColorError {
    InvalidFormat,
    InvalidDigit,
    Empty,
}

#[derive(Debug)]
pub struct ParseThemeError;

impl fmt::Display for ParseThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error parsing theme")
    }
}

impl Error for ParseThemeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

fn make_color(c: &str) -> Color {
    if let Ok(c) = colorsys::Rgb::from_hex_str(c) {
        Color::Rgb(c.red() as u8, c.green() as u8, c.blue() as u8)
    } else {
        Color::Reset
    }
}

#[derive(Debug)]
pub struct Theme {
    base: Style,
    selection: Option<Style>,
    active_selection: Option<Style>,
    border: Option<Style>,
    active_border: Option<Style>,
}

impl Theme {
    pub fn base(&self) -> Style {
        self.base.clone()
    }

    pub fn active_selection(&self) -> Style {
        if let Some(s) = self.active_selection {
            s.to_owned()
        } else {
            self.base.clone().add_modifier(Modifier::REVERSED)
        }
    }

    pub fn selection(&self) -> Style {
        if let Some(s) = self.selection {
            s.to_owned()
        } else {
            self.active_selection()
        }
    }

    pub fn active_border(&self) -> Style {
        if let Some(s) = self.active_border {
            s.to_owned()
        } else {
            self.base.clone()
        }
    }

    pub fn border(&self) -> Style {
        if let Some(s) = self.border {
            s.to_owned()
        } else {
            self.active_border().add_modifier(Modifier::DIM)
        }
    }

    pub fn from_yaml(s: &str) -> anyhow::Result<Self> {
        let cs = serde_yaml::from_str::<ColorSchemeFile>(s)?;

        let style = match cs.colors.primary {
            Some(p) => {
                let mut s = Style::default();
                if let Some(fg) = p.foreground {
                    s = s.fg(make_color(&fg));
                }
                if let Some(bg) = p.background {
                    s = s.bg(make_color(&bg));
                }
                s
            }
            None => Default::default(),
        };

        Ok(Self {
            base: style,
            ..Default::default()
        })
    }

    pub fn borland() -> Self {
        let white = make_color("#FFFFFF");
        let gray = make_color("#bbbbbb");
        let midnight = make_color("#000080");
        let yellow = make_color("#fefd72");

        Self {
            base: Style::default().fg(white).bg(midnight),
            border: Some(Style::default().fg(gray)),
            active_border: Some(Style::default().fg(white)),
            selection: Some(Style::default().fg(midnight).bg(gray)),
            active_selection: Some(Style::default().fg(midnight).bg(yellow)),
        }
    }

    pub fn jungle() -> Self {
        Self {
            base: Default::default(),
            active_selection: Some(Style::default().green().reversed()),
            active_border: Some(Style::default().green()),
            selection: Some(Style::default().dim().reversed()),
            border: Some(Style::default().dim()),
        }
    }

    pub fn matrix() -> Self {
        let bright_green = make_color("#49f27e");
        let mid_green = make_color("#29ad53");
        let dark_green = make_color("#04100a");

        Self {
            base: Style::default().fg(bright_green).bg(dark_green),
            border: Some(Style::default().fg(mid_green)),
            active_border: Some(Style::default().fg(bright_green)),
            selection: Some(Style::default().fg(dark_green).bg(mid_green)),
            active_selection: Some(Style::default().fg(dark_green).bg(bright_green)),
        }
    }

    pub fn redshift() -> Self {
        Self {
            base: Style::default().red(),
            active_selection: Some(Style::default().red().reversed()),
            active_border: Some(Style::default().red()),
            selection: Some(Style::default().dim().reversed()),
            border: Some(Style::default().dim()),
        }
    }

    pub fn wyse() -> Self {
        let bright_amber = make_color("#eaac1f");
        let dark_amber = make_color("#936708");
        let black = make_color("#231d17");

        Self {
            base: Style::default().fg(bright_amber).bg(black),
            border: Some(Style::default().fg(dark_amber)),
            active_border: Some(Style::default().fg(bright_amber)),
            selection: Some(Style::default().fg(black).bg(dark_amber)),
            active_selection: Some(Style::default().fg(black).bg(bright_amber)),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            base: Style::default(),
            active_selection: None,
            selection: None,
            active_border: None,
            border: None,
        }
    }
}

impl TryFrom<&toml::Value> for Theme {
    type Error = ParseThemeError;

    fn try_from(value: &toml::Value) -> Result<Self, Self::Error> {
        match value {
            toml::Value::String(name) => match name.as_str() {
                "default" => Ok(Self::default()),
                "borland" => Ok(Self::borland()),
                "jungle" => Ok(Self::jungle()),
                "matrix" => Ok(Self::matrix()),
                "redshift" => Ok(Self::redshift()),
                "wyse" => Ok(Self::wyse()),
                file => {
                    if std::path::Path::new(file).exists() {
                        let contents = std::fs::read_to_string(file).or(Err(ParseThemeError))?;
                        let table = contents.parse::<Value>().or(Err(ParseThemeError))?;
                        Self::try_from(&table).or(Err(ParseThemeError))
                    } else {
                        Err(ParseThemeError)
                    }
                }
            },
            toml::Value::Table(scheme) => Ok(Self {
                base: scheme
                    .get("base")
                    .and_then(|v| try_style_from_toml(v).ok())
                    .unwrap_or_default(),
                active_selection: scheme
                    .get("active_selection")
                    .and_then(|v| try_style_from_toml(v).ok()),
                selection: scheme
                    .get("selection")
                    .and_then(|v| try_style_from_toml(v).ok()),
                active_border: scheme
                    .get("active_border")
                    .and_then(|v| try_style_from_toml(v).ok()),
                border: scheme
                    .get("border")
                    .and_then(|v| try_style_from_toml(v).ok()),
            }),
            _ => Err(ParseThemeError),
        }
    }
}

fn try_style_from_toml(value: &toml::Value) -> Result<Style, ParseColorError> {
    match value {
        toml::Value::String(name) => match name.to_lowercase().as_str() {
            "black" => Ok(Style::default().black()),
            "red" => Ok(Style::default().red()),
            "green" => Ok(Style::default().green()),
            "yellow" => Ok(Style::default().yellow()),
            "blue" => Ok(Style::default().blue()),
            "magenta" => Ok(Style::default().magenta()),
            "cyan" => Ok(Style::default().cyan()),
            "gray" => Ok(Style::default().gray()),
            "lightblack" | "darkgray" => Ok(Style::default().dark_gray()),
            "lightred" => Ok(Style::default().light_red()),
            "lightgreen" => Ok(Style::default().light_green()),
            "lightyellow" => Ok(Style::default().light_yellow()),
            "lightblue" => Ok(Style::default().light_blue()),
            "lightmagenta" => Ok(Style::default().light_magenta()),
            "lightcyan" => Ok(Style::default().light_cyan()),
            "white" => Ok(Style::default().white()),
            hex if hex.starts_with('#') => Ok(Style::default().fg(make_color(hex))),
            _ => Err(ParseColorError::InvalidFormat),
        },

        toml::Value::Table(record) => {
            let style = record
                .get("fg")
                .and_then(|v| try_style_from_toml(v).ok())
                .unwrap_or_default();

            if let Some(bg) = record.get("bg").and_then(|v| try_style_from_toml(v).ok()) {
                Ok(style.bg(bg.fg.unwrap()))
            } else {
                Ok(style)
            }
        }

        _ => Err(ParseColorError::InvalidFormat),
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorSchemeFile {
    colors: Colors,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Colors {
    name: Option<String>,
    author: Option<String>,
    primary: Option<PrimaryColors>,
    cursor: Option<CursorColors>,
    normal: Option<VariantColors>,
    bright: Option<VariantColors>,
    dim: Option<VariantColors>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PrimaryColors {
    foreground: Option<String>,
    background: Option<String>,
    bright_foreground: Option<String>,
    dim_foreground: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CursorColors {
    text: Option<String>,
    cursor: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct VariantColors {
    black: Option<String>,
    red: Option<String>,
    green: Option<String>,
    yellow: Option<String>,
    blue: Option<String>,
    magenta: Option<String>,
    cyan: Option<String>,
    white: Option<String>,
}
