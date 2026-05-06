//! IronKey colour palette, fonts, and custom iced Theme.

use iced::{Color, Theme, theme::Palette};

// ──────────────────────────────────────────────────────────────────────────────
// Colour palette constants
// ──────────────────────────────────────────────────────────────────────────────

pub const BG_BASE: Color = Color { r: 0.039, g: 0.047, b: 0.059, a: 1.0 };
pub const BG_PANEL: Color = Color { r: 0.059, g: 0.075, b: 0.094, a: 1.0 };
pub const BG_ELEVATED: Color = Color { r: 0.086, g: 0.106, b: 0.133, a: 1.0 };
pub const BORDER_DEFAULT: Color = Color { r: 0.118, g: 0.145, b: 0.188, a: 1.0 };
pub const BORDER_ACTIVE: Color = Color { r: 0.0, g: 0.278, b: 0.671, a: 1.0 };
pub const ACCENT_PRIMARY: Color = Color { r: 0.0, g: 0.831, b: 1.0, a: 1.0 };
pub const ACCENT_SECONDARY: Color = Color { r: 0.0, g: 0.278, b: 0.671, a: 1.0 };
pub const ACCENT_DANGER: Color = Color { r: 1.0, g: 0.231, b: 0.231, a: 1.0 };
pub const ACCENT_WARNING: Color = Color { r: 1.0, g: 0.722, b: 0.0, a: 1.0 };
pub const ACCENT_SUCCESS: Color = Color { r: 0.0, g: 1.0, b: 0.612, a: 1.0 };
pub const TEXT_PRIMARY: Color = Color { r: 0.910, g: 0.929, b: 0.953, a: 1.0 };
pub const TEXT_SECONDARY: Color = Color { r: 0.353, g: 0.392, b: 0.471, a: 1.0 };
pub const TEXT_CODE: Color = Color { r: 0.0, g: 0.831, b: 1.0, a: 1.0 };

// ──────────────────────────────────────────────────────────────────────────────
// Theme builder
// ──────────────────────────────────────────────────────────────────────────────

/// Build the IronKey custom `iced::Theme`.
pub fn ironkey_theme() -> Theme {
    Theme::custom(
        "IronKey".to_string(),
        Palette {
            background: BG_BASE,
            text: TEXT_PRIMARY,
            primary: ACCENT_PRIMARY,
            success: ACCENT_SUCCESS,
            warning: ACCENT_WARNING,
            danger: ACCENT_DANGER,
        },
    )
}

// ──────────────────────────────────────────────────────────────────────────────
// Helper: create a styled container background
// ──────────────────────────────────────────────────────────────────────────────

use iced::{Border, Shadow};
use iced::widget::container;

/// A panel surface style (slightly elevated background with default border).
pub fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG_PANEL.into()),
        border: Border {
            color: BORDER_DEFAULT,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(TEXT_PRIMARY),
        snap: false,
    }
}

/// A focused/active panel border (cobalt blue glow).
pub fn active_panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG_PANEL.into()),
        border: Border {
            color: BORDER_ACTIVE,
            width: 2.0,
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: Color { r: 0.0, g: 0.278, b: 0.671, a: 0.4 },
            offset: iced::Vector { x: 0.0, y: 0.0 },
            blur_radius: 8.0,
        },
        text_color: Some(TEXT_PRIMARY),
        snap: false,
    }
}

/// The header / status bar surface.
pub fn header_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BG_ELEVATED.into()),
        border: Border {
            color: BORDER_DEFAULT,
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(TEXT_PRIMARY),
        snap: false,
    }
}

/// A danger button background (destructive actions).
pub fn danger_button_style(_theme: &Theme) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Color { r: 0.6, g: 0.05, b: 0.05, a: 1.0 }.into()),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: ACCENT_DANGER,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
