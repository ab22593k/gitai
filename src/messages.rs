use crate::ui::{
    AURORA_GREEN, CELESTIAL_BLUE, COMET_ORANGE, GALAXY_PINK, METEOR_RED, NEBULA_PURPLE,
    PLASMA_CYAN, SOLAR_YELLOW, STARLIGHT,
};
use rand::seq::IndexedRandom;
use ratatui::style::Color;
use std::sync::LazyLock;

#[derive(Clone, Debug)]
pub struct ColoredMessage {
    pub text: String,
    pub color: Color,
}

impl ColoredMessage {
    fn from_static(text: &'static str, color: Color) -> Self {
        Self {
            text: text.to_string(),
            color,
        }
    }
}

macro_rules! messages {
    ($($text:expr => $color:expr),+ $(,)?) => {
        vec![
            $(ColoredMessage::from_static($text, $color)),+
        ]
    };
}

static WAITING_MESSAGES: LazyLock<Vec<ColoredMessage>> = LazyLock::new(|| {
    messages![
        "Consulting the cosmic commit oracle..." => NEBULA_PURPLE,
        "Aligning the celestial code spheres..." => CELESTIAL_BLUE,
        "Channeling the spirit of clean commits..." => AURORA_GREEN,
        "Launching commit ideas into the coding cosmos..." => METEOR_RED,
        "Exploring the galaxy of potential messages..." => PLASMA_CYAN,
        "Peering into the commit-verse for inspiration..." => SOLAR_YELLOW,
        "Casting a spell for the perfect commit message..." => GALAXY_PINK,
        "Harnessing the power of a thousand code stars..." => STARLIGHT,
        "Orbiting the planet of precise git descriptions..." => CELESTIAL_BLUE,
        "Weaving a tapestry of colorful commit prose..." => PLASMA_CYAN,
        "Igniting the fireworks of code brilliance..." => COMET_ORANGE,
        "Syncing with the collective coding consciousness..." => AURORA_GREEN,
        "Aligning the moon phases for optimal commit clarity..." => STARLIGHT,
        "Analyzing code particles at the quantum level..." => NEBULA_PURPLE,
        "Decoding the DNA of your changes..." => GALAXY_PINK,
        "Summoning the ancient spirits of version control..." => METEOR_RED,
        "Tuning into the frequency of flawless commits..." => CELESTIAL_BLUE,
        "Charging the commit crystals with cosmic energy..." => PLASMA_CYAN,
        "Translating your changes into universal code..." => AURORA_GREEN,
        "Distilling the essence of your modifications..." => SOLAR_YELLOW,
        "Unraveling the threads of your code tapestry..." => NEBULA_PURPLE,
        "Consulting the all-knowing git guardians..." => CELESTIAL_BLUE,
        "Harmonizing with the rhythms of the coding universe..." => GALAXY_PINK,
        "Diving into the depths of the code ocean..." => PLASMA_CYAN,
        "Seeking wisdom from the repository sages..." => AURORA_GREEN,
        "Calibrating the commit compass for true north..." => SOLAR_YELLOW,
        "Unlocking the secrets of the commit constellations..." => NEBULA_PURPLE,
        "Gathering stardust for your stellar commit..." => STARLIGHT,
        "Focusing the lens of the code telescope..." => CELESTIAL_BLUE,
        "Riding the waves of inspiration through the code cosmos..." => PLASMA_CYAN,
    ]
});

static REVIEW_WAITING_MESSAGES: LazyLock<Vec<ColoredMessage>> = LazyLock::new(|| {
    messages![
        "Scanning code dimensions for quality signatures..." => NEBULA_PURPLE,
        "Traversing the architecture cosmos for patterns..." => CELESTIAL_BLUE,
        "Invoking the guardians of code integrity..." => AURORA_GREEN,
        "Illuminating shadow bugs with code starlight..." => STARLIGHT,
        "Gazing into the crystal orb of future maintainability..." => PLASMA_CYAN,
        "Unrolling the ancient scrolls of best practices..." => SOLAR_YELLOW,
        "Distilling your code into its purest essence..." => GALAXY_PINK,
        "Weighing your code on the scales of elegance..." => CELESTIAL_BLUE,
        "Tracing the rainbow paths between your functions..." => AURORA_GREEN,
        "Magnifying the subtle harmonies in your algorithms..." => NEBULA_PURPLE,
        "Communing with the collective wisdom of master coders..." => METEOR_RED,
        "Diving into the depths of your code ocean..." => PLASMA_CYAN,
        "Consulting the monoliths of software architecture..." => COMET_ORANGE,
        "Sifting through the time sands of execution paths..." => SOLAR_YELLOW,
        "Assembling the puzzle pieces of your code story..." => GALAXY_PINK,
        "Analyzing code particles at quantum precision..." => CELESTIAL_BLUE,
        "Measuring the brightness of your code stars..." => STARLIGHT,
        "Following the threads of logic throughout your tapestry..." => AURORA_GREEN,
        "Summoning the trident of code quality dimensions..." => NEBULA_PURPLE,
        "Spiraling through nested layers of abstraction..." => PLASMA_CYAN,
        "Examining the ancient artifacts of your repository..." => METEOR_RED,
        "Unmasking the hidden characters in your code drama..." => GALAXY_PINK,
        "Warding off evil bugs with protective insights..." => CELESTIAL_BLUE,
        "Forging stronger code in the flames of analysis..." => COMET_ORANGE,
        "Nurturing the seeds of excellence in your codebase..." => AURORA_GREEN,
        "Pinpointing opportunities for cosmic refinement..." => SOLAR_YELLOW,
        "Mapping the intricate web of dependencies..." => NEBULA_PURPLE,
        "Calibrating the tools of code enlightenment..." => PLASMA_CYAN,
        "Computing the algorithms of optimal elegance..." => STARLIGHT,
        "Charting the trajectory of your code evolution..." => CELESTIAL_BLUE,
    ]
});

/// Returns a random waiting message for commit operations
pub fn get_waiting_message() -> &'static ColoredMessage {
    WAITING_MESSAGES
        .choose(&mut rand::rng())
        .expect("WAITING_MESSAGES should never be empty")
}

/// Returns a random waiting message for code review operations
pub fn get_review_waiting_message() -> &'static ColoredMessage {
    REVIEW_WAITING_MESSAGES
        .choose(&mut rand::rng())
        .expect("REVIEW_WAITING_MESSAGES should never be empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waiting_messages_not_empty() {
        assert!(!WAITING_MESSAGES.is_empty());
        assert!(!REVIEW_WAITING_MESSAGES.is_empty());
    }

    #[test]
    fn test_get_messages_returns_valid() {
        let msg = get_waiting_message();
        assert!(!msg.text.is_empty());

        let review_msg = get_review_waiting_message();
        assert!(!review_msg.text.is_empty());
    }
}
