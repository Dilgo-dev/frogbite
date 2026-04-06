//! Built-in color themes.
//!
//! The active theme is loaded from `Settings.theme` exactly once at startup
//! and cached in a `LazyLock<Theme>`. Changes to the theme require a restart
//! to take effect; live-reload is tracked as a follow-up.

use std::sync::{LazyLock, RwLock};

use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub surface: Color,
    pub fg: Color,
    pub muted: Color,
    pub green: Color,
    pub orange: Color,
    pub purple: Color,
    pub red: Color,
    pub yellow: Color,
    pub teal: Color,
}

impl Theme {
    pub const fn scooby() -> Self {
        Self {
            name: "scooby",
            bg: Color::Rgb(13, 27, 26),
            surface: Color::Rgb(22, 34, 32),
            fg: Color::Rgb(232, 232, 224),
            muted: Color::Rgb(107, 138, 107),
            green: Color::Rgb(124, 179, 66),
            orange: Color::Rgb(255, 111, 0),
            purple: Color::Rgb(156, 39, 176),
            red: Color::Rgb(211, 47, 47),
            yellow: Color::Rgb(255, 143, 0),
            teal: Color::Rgb(0, 137, 123),
        }
    }

    pub const fn mono() -> Self {
        Self {
            name: "mono",
            bg: Color::Rgb(10, 10, 10),
            surface: Color::Rgb(22, 22, 22),
            fg: Color::Rgb(232, 232, 232),
            muted: Color::Rgb(120, 120, 120),
            green: Color::Rgb(220, 220, 220),
            orange: Color::Rgb(200, 200, 200),
            purple: Color::Rgb(180, 180, 180),
            red: Color::Rgb(180, 90, 90),
            yellow: Color::Rgb(220, 220, 180),
            teal: Color::Rgb(170, 200, 200),
        }
    }

    pub const fn dracula() -> Self {
        Self {
            name: "dracula",
            bg: Color::Rgb(40, 42, 54),
            surface: Color::Rgb(55, 57, 70),
            fg: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(98, 114, 164),
            green: Color::Rgb(80, 250, 123),
            orange: Color::Rgb(255, 184, 108),
            purple: Color::Rgb(189, 147, 249),
            red: Color::Rgb(255, 85, 85),
            yellow: Color::Rgb(241, 250, 140),
            teal: Color::Rgb(139, 233, 253),
        }
    }

    pub const fn nord() -> Self {
        Self {
            name: "nord",
            bg: Color::Rgb(46, 52, 64),
            surface: Color::Rgb(59, 66, 82),
            fg: Color::Rgb(216, 222, 233),
            muted: Color::Rgb(136, 145, 158),
            green: Color::Rgb(163, 190, 140),
            orange: Color::Rgb(208, 135, 112),
            purple: Color::Rgb(180, 142, 173),
            red: Color::Rgb(191, 97, 106),
            yellow: Color::Rgb(235, 203, 139),
            teal: Color::Rgb(143, 188, 187),
        }
    }

    pub const fn solarized_dark() -> Self {
        Self {
            name: "solarized-dark",
            bg: Color::Rgb(0, 43, 54),
            surface: Color::Rgb(7, 54, 66),
            fg: Color::Rgb(238, 232, 213),
            muted: Color::Rgb(101, 123, 131),
            green: Color::Rgb(133, 153, 0),
            orange: Color::Rgb(203, 75, 22),
            purple: Color::Rgb(108, 113, 196),
            red: Color::Rgb(220, 50, 47),
            yellow: Color::Rgb(181, 137, 0),
            teal: Color::Rgb(42, 161, 152),
        }
    }
}

pub const ALL_THEMES: &[Theme] = &[
    Theme::scooby(),
    Theme::mono(),
    Theme::dracula(),
    Theme::nord(),
    Theme::solarized_dark(),
];

pub fn by_name(name: &str) -> Theme {
    ALL_THEMES
        .iter()
        .find(|t| t.name == name)
        .copied()
        .unwrap_or_else(Theme::scooby)
}

static ACTIVE: RwLock<Theme> = RwLock::new(Theme::scooby());
static INIT: LazyLock<()> = LazyLock::new(|| {});

/// Initializes the active theme from a settings name. Should be called once
/// at startup, before any draw call.
pub fn init(name: &str) {
    LazyLock::force(&INIT);
    if let Ok(mut t) = ACTIVE.write() {
        *t = by_name(name);
    }
}

#[inline]
pub fn theme() -> Theme {
    ACTIVE.read().map_or_else(|_| Theme::scooby(), |g| *g)
}
