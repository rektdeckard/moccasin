use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use toml::Value;
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
    selection_active: Option<Style>,
    border: Option<Style>,
    border_active: Option<Style>,
    scrollbar: Option<Style>,
}

impl Theme {
    pub fn base(&self) -> Style {
        self.base.clone()
    }

    pub fn selection(&self) -> Style {
        if let Some(s) = self.selection {
            s.to_owned()
        } else {
            self.active_selection()
        }
    }

    pub fn active_selection(&self) -> Style {
        if let Some(s) = self.selection_active {
            s.to_owned()
        } else {
            self.base.clone().add_modifier(Modifier::REVERSED)
        }
    }

    pub fn border(&self) -> Style {
        if let Some(s) = self.border {
            s.to_owned()
        } else {
            self.active_border().add_modifier(Modifier::DIM)
        }
    }

    pub fn active_border(&self) -> Style {
        if let Some(s) = self.border_active {
            s.to_owned()
        } else {
            self.base.clone()
        }
    }

    pub fn scrollbar_thumb(&self) -> Style {
        if let Some(s) = self.scrollbar {
            if let Some(fg) = s.fg {
                Style::default().fg(fg)
            } else {
                Style::default()
            }
        } else {
            Style::default()
        }
    }

    pub fn scrollbar_track(&self) -> Style {
        if let Some(s) = self.scrollbar {
            if let Some(bg) = s.bg.or(self.base.bg) {
                Style::default().fg(bg)
            } else {
                Style::default().dim()
            }
        } else {
            self.base().dim()
        }
    }

    pub fn borland() -> Self {
        let white = make_color("#FFFFFF");
        let gray = make_color("#bbbbbb");
        let midnight = make_color("#000080");
        let yellow = make_color("#fefd72");

        Self {
            base: Style::default().fg(white).bg(midnight),
            border: Some(Style::default().fg(gray)),
            border_active: Some(Style::default().fg(white)),
            selection: Some(Style::default().fg(midnight).bg(gray)),
            selection_active: Some(Style::default().fg(midnight).bg(yellow)),
            scrollbar: Some(Style::default().fg(white).bg(gray)),
        }
    }

    pub fn jungle() -> Self {
        Self {
            base: Default::default(),
            selection_active: Some(Style::default().green().reversed()),
            border_active: Some(Style::default().green()),
            selection: Some(Style::default().dim().reversed()),
            border: Some(Style::default().dim()),
            scrollbar: Some(Style::default().dim()),
        }
    }

    pub fn matrix() -> Self {
        let bright_green = make_color("#49f27e");
        let mid_green = make_color("#29ad53");
        let dark_green = make_color("#04100a");

        Self {
            base: Style::default().fg(bright_green).bg(dark_green),
            border: Some(Style::default().fg(mid_green)),
            border_active: Some(Style::default().fg(bright_green)),
            selection: Some(Style::default().fg(dark_green).bg(mid_green)),
            selection_active: Some(Style::default().fg(dark_green).bg(bright_green)),
            scrollbar: Some(Style::default()),
        }
    }

    pub fn redshift() -> Self {
        Self {
            base: Style::default().red().on_black(),
            selection_active: Some(Style::default().red().reversed()),
            border_active: Some(Style::default().red()),
            selection: Some(Style::default().dim().reversed()),
            border: Some(Style::default().dim()),
            scrollbar: Some(Style::default().dim()),
        }
    }

    pub fn wyse() -> Self {
        let bright_amber = make_color("#eaac1f");
        let dark_amber = make_color("#936708");
        let black = make_color("#231d17");

        Self {
            base: Style::default().fg(bright_amber).bg(black),
            border: Some(Style::default().fg(dark_amber)),
            border_active: Some(Style::default().fg(bright_amber)),
            selection: Some(Style::default().fg(black).bg(dark_amber)),
            selection_active: Some(Style::default().fg(black).bg(bright_amber)),
            scrollbar: Some(Style::default()),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            base: Style::default(),
            selection_active: None,
            selection: None,
            border_active: None,
            border: None,
            scrollbar: Some(Style::default().dim()),
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
                selection_active: scheme
                    .get("selection_active")
                    .and_then(|v| try_style_from_toml(v).ok()),
                selection: scheme
                    .get("selection")
                    .and_then(|v| try_style_from_toml(v).ok()),
                border: scheme
                    .get("border")
                    .and_then(|v| try_style_from_toml(v).ok()),
                border_active: scheme
                    .get("border_active")
                    .and_then(|v| try_style_from_toml(v).ok()),
                scrollbar: scheme
                    .get("scrollbar")
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