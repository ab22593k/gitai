//! Color constants and quiet mode management for CLI output.

use parking_lot::Mutex;
use ratatui::style::Color;

pub const STARLIGHT: Color = Color::Rgb(255, 255, 240);
pub const NEBULA_PURPLE: Color = Color::Rgb(167, 132, 239);
pub const CELESTIAL_BLUE: Color = Color::Rgb(75, 115, 235);
pub const SOLAR_YELLOW: Color = Color::Rgb(255, 225, 100);
pub const AURORA_GREEN: Color = Color::Rgb(140, 255, 170);
pub const PLASMA_CYAN: Color = Color::Rgb(20, 255, 255);
pub const METEOR_RED: Color = Color::Rgb(255, 89, 70);
pub const GALAXY_PINK: Color = Color::Rgb(255, 162, 213);
pub const COMET_ORANGE: Color = Color::Rgb(255, 165, 0);
pub const BLACK_HOLE: Color = Color::Rgb(0, 0, 0);

/// Track quiet mode state
static QUIET_MODE: std::sync::LazyLock<Mutex<bool>> =
    std::sync::LazyLock::new(|| Mutex::new(false));

/// Enable or disable quiet mode
#[inline]
pub fn set_quiet_mode(enabled: bool) {
    *QUIET_MODE.lock() = enabled;
}

/// Check if quiet mode is enabled
#[inline]
pub fn is_quiet_mode() -> bool {
    *QUIET_MODE.lock()
}
