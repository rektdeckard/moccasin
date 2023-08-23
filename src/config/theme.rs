use super::ConfigVariant;
use serde::{Deserialize, Serialize};
use serde_yaml;
use tui::style::{Color, Modifier, Style, Stylize};

#[derive(Debug)]
enum ParseColorError {
    InvalidFormat,
    InvalidDigit,
    Empty,
}

fn make_color(c: &str) -> Color {
    if let Ok(c) = colorsys::Rgb::from_hex_str(c) {
        Color::Rgb(c.red() as u8, c.green() as u8, c.blue() as u8)
    } else {
        Color::Red
    }
}

#[derive(Debug)]
pub struct Theme {
    base: Style,
    active_selection: Option<Style>,
    inactive_selection: Option<Style>,
    active_panel: Option<Style>,
    inactive_panel: Option<Style>,
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

    pub fn inactive_selection(&self) -> Style {
        if let Some(s) = self.inactive_selection {
            s.to_owned()
        } else {
            self.active_selection()
        }
    }

    pub fn active_panel(&self) -> Style {
        if let Some(s) = self.active_panel {
            s.to_owned()
        } else {
            self.base.clone()
        }
    }

    pub fn inactive_panel(&self) -> Style {
        if let Some(s) = self.inactive_panel {
            s.to_owned()
        } else {
            self.active_panel().add_modifier(Modifier::DIM)
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

    pub fn jungle() -> Self {
        Self {
            base: Default::default(),
            active_selection: Some(Style::default().green().reversed()),
            active_panel: Some(Style::default().green()),
            inactive_selection: Some(Style::default().dim().reversed()),
            inactive_panel: Some(Style::default().dim()),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            base: Style::default(),
            active_selection: None,
            inactive_selection: None,
            active_panel: None,
            inactive_panel: None,
        }
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
