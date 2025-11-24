//! Theme system for adaptive TUI colors
//!
//! This module provides an adaptive theming system that works across different
//! terminal capabilities and user preferences.

use crate::common::ThemeMode;
use ratatui::style::{Color, Modifier};
use std::env;

/// Terminal color capability levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCapability {
    /// Basic 16 colors (traditional terminals)
    Basic16,
    /// 256 colors
    Color256,
    /// True color (24-bit)
    TrueColor,
}

/// Theme configuration with adaptive colors
#[derive(Debug, Clone)]
pub struct Theme {
    /// Terminal color capability
    pub capability: ColorCapability,

    /// Active theme mode
    pub mode: ThemeMode,

    /// Brand colors
    pub brand_primary: Color,

    /// Text colors
    pub text_default: Color,
    pub text_dimmed: Color,

    /// Background colors
    pub background_default: Color,
    pub background_elevated: Color,

    /// State colors
    pub state_success: Color,
    pub state_error: Color,
    pub state_warning: Color,
    pub state_info: Color,

    /// UI element colors
    pub accent: Color,
    pub accent_active: Color,
    pub border: Color,
    pub border_active: Color,

    /// Typography
    pub font_weight_regular: Modifier,
    pub font_weight_bold: Modifier,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeMode::default())
    }
}

impl Theme {
    /// Create a new theme with the specified mode
    pub fn new(mode: ThemeMode) -> Self {
        let capability = Self::detect_color_capability();
        let resolved_mode = mode.resolve();

        match capability {
            ColorCapability::TrueColor | ColorCapability::Color256 => {
                Self::modern_theme(capability, resolved_mode)
            }
            ColorCapability::Basic16 => Self::basic_theme(resolved_mode),
        }
    }

    /// Detect and create theme (legacy wrapper)
    pub fn detect_and_create() -> Self {
        Self::new(ThemeMode::default())
    }

    /// Detect terminal color capabilities
    fn detect_color_capability() -> ColorCapability {
        // Check environment variables for color support
        if let Ok(colorterm) = env::var("COLORTERM")
            && (colorterm.contains("truecolor") || colorterm.contains("24bit"))
        {
            return ColorCapability::TrueColor;
        }

        // Check TERM for color capabilities
        if let Ok(term) = env::var("TERM") {
            if term.contains("256color") || term.contains("xterm-256color") {
                return ColorCapability::Color256;
            }
            if term.contains("truecolor") || term.contains("24bit") {
                return ColorCapability::TrueColor;
            }
        }

        // Check for explicit color support
        if let Ok(clicolor_force) = env::var("CLICOLOR_FORCE")
            && clicolor_force == "1"
        {
            return ColorCapability::Color256;
        }

        // Default to basic 16 colors for safety
        ColorCapability::Basic16
    }

    /// Modern theme for terminals with good color support
    fn modern_theme(capability: ColorCapability, mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self {
                capability,
                mode,
                // Brand colors
                brand_primary: Color::Rgb(37, 99, 235), // Blue-600

                // Text colors
                text_default: Color::Rgb(17, 24, 39), // Gray-900
                text_dimmed: Color::Rgb(107, 114, 128), // Gray-500

                // Background colors
                background_default: Color::Rgb(255, 255, 255), // White
                background_elevated: Color::Rgb(243, 244, 246), // Gray-100

                // State colors
                state_success: Color::Rgb(22, 163, 74), // Green-600
                state_error: Color::Rgb(220, 38, 38),   // Red-600
                state_warning: Color::Rgb(217, 119, 6), // Amber-600
                state_info: Color::Rgb(37, 99, 235),    // Blue-600

                // UI element colors
                accent: Color::Rgb(37, 99, 235),         // Blue-600
                accent_active: Color::Rgb(59, 130, 246), // Blue-500
                border: Color::Rgb(209, 213, 219),       // Gray-300
                border_active: Color::Rgb(59, 130, 246), // Blue-500

                // Typography
                font_weight_regular: Modifier::empty(),
                font_weight_bold: Modifier::BOLD,
            },
            _ => Self {
                // Dark (Default)
                capability,
                mode,
                // Brand colors
                brand_primary: Color::Rgb(59, 130, 246), // Blue-500

                // Text colors
                text_default: Color::Rgb(255, 255, 255), // White
                text_dimmed: Color::Rgb(107, 114, 128),  // Gray-500

                // Background colors
                background_default: Color::Rgb(0, 0, 0), // Black
                background_elevated: Color::Rgb(31, 41, 55), // Gray-800

                // State colors
                state_success: Color::Rgb(34, 197, 94), // Green-500
                state_error: Color::Rgb(239, 68, 68),   // Red-500
                state_warning: Color::Rgb(245, 158, 11), // Amber-500
                state_info: Color::Rgb(59, 130, 246),   // Blue-500

                // UI element colors
                accent: Color::Rgb(59, 130, 246),        // Blue-500
                accent_active: Color::Rgb(96, 165, 250), // Blue-400
                border: Color::Rgb(55, 65, 81),          // Gray-700
                border_active: Color::Rgb(96, 165, 250), // Blue-400

                // Typography
                font_weight_regular: Modifier::empty(),
                font_weight_bold: Modifier::BOLD,
            },
        }
    }

    /// Basic theme for terminals with limited color support
    fn basic_theme(mode: ThemeMode) -> Self {
        let is_light = matches!(mode, ThemeMode::Light);

        Self {
            capability: ColorCapability::Basic16,
            mode,
            // Brand colors
            brand_primary: Color::Blue,

            // Text colors
            text_default: if is_light { Color::Black } else { Color::White },
            text_dimmed: Color::Gray,

            // Background colors
            background_default: if is_light { Color::White } else { Color::Black },
            background_elevated: if is_light {
                Color::Gray
            } else {
                Color::DarkGray
            },

            // State colors
            state_success: Color::Green,
            state_error: Color::Red,
            state_warning: Color::Yellow,
            state_info: Color::Blue,

            // UI element colors
            accent: Color::Blue,
            accent_active: Color::Cyan,
            border: if is_light {
                Color::Gray
            } else {
                Color::DarkGray
            },
            border_active: Color::Cyan,

            // Typography
            font_weight_regular: Modifier::empty(),
            font_weight_bold: Modifier::BOLD,
        }
    }

    /// Create a theme with custom colors (for testing or user configuration)
    pub fn custom() -> Self {
        Self::modern_theme(ColorCapability::TrueColor, ThemeMode::Dark)
    }

    /// Get appropriate color based on capability
    pub fn adaptive_color(&self, modern: Color, basic: Color) -> Color {
        match self.capability {
            ColorCapability::Basic16 => basic,
            _ => modern,
        }
    }

    /// Check if terminal supports true color
    pub fn supports_true_color(&self) -> bool {
        self.capability == ColorCapability::TrueColor
    }

    /// Check if terminal supports 256 colors
    pub fn supports_256_colors(&self) -> bool {
        matches!(
            self.capability,
            ColorCapability::Color256 | ColorCapability::TrueColor
        )
    }
}

use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

/// Get the global theme instance
pub fn get_theme() -> &'static Theme {
    THEME.get_or_init(Theme::default)
}

/// Set the global theme (only works if not already initialized)
pub fn set_theme(theme: Theme) {
    let _ = THEME.set(theme);
}

/// Initialize theme with specific mode
pub fn init_theme(mode: ThemeMode) {
    set_theme(Theme::new(mode));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = Theme::default();
        assert!(matches!(
            theme.capability,
            ColorCapability::Basic16 | ColorCapability::Color256 | ColorCapability::TrueColor
        ));
    }

    #[test]
    fn test_basic_theme() {
        let theme = Theme::basic_theme(ThemeMode::Dark);
        assert_eq!(theme.capability, ColorCapability::Basic16);
        assert_eq!(theme.brand_primary, Color::Blue);
        assert_eq!(theme.text_default, Color::White);
    }

    #[test]
    fn test_modern_theme() {
        let theme = Theme::modern_theme(ColorCapability::TrueColor, ThemeMode::Dark);
        assert_eq!(theme.capability, ColorCapability::TrueColor);
        // Test that RGB colors are used for modern theme
        match theme.brand_primary {
            Color::Rgb(_, _, _) => {} // Should be RGB
            _ => panic!("Modern theme should use RGB colors"),
        }
    }

    #[test]
    fn test_light_theme() {
        let theme = Theme::modern_theme(ColorCapability::TrueColor, ThemeMode::Light);
        assert_eq!(theme.background_default, Color::Rgb(255, 255, 255));
        assert_eq!(theme.text_default, Color::Rgb(17, 24, 39));
    }

    #[test]
    fn test_adaptive_color() {
        let basic_theme = Theme::basic_theme(ThemeMode::Dark);
        let modern_theme = Theme::modern_theme(ColorCapability::TrueColor, ThemeMode::Dark);

        let color = basic_theme.adaptive_color(Color::Rgb(255, 0, 0), Color::Red);
        assert_eq!(color, Color::Red); // Basic theme should use basic color

        let color = modern_theme.adaptive_color(Color::Rgb(255, 0, 0), Color::Red);
        assert_eq!(color, Color::Rgb(255, 0, 0)); // Modern theme should use RGB color
    }

    #[test]
    fn test_capability_detection() {
        // Test basic detection - should not panic
        let capability = Theme::detect_color_capability();
        assert!(matches!(
            capability,
            ColorCapability::Basic16 | ColorCapability::Color256 | ColorCapability::TrueColor
        ));
    }
}
