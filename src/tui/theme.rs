use ratatui::style::Color;

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub title_gradient: Vec<Color>,
    pub celestial_blue: Color,
    pub aurora_green: Color,
    pub comet_orange: Color,
    pub galaxy_pink: Color,
    pub meteor_red: Color,
    pub nebula_purple: Color,
    pub plasma_cyan: Color,
    pub solar_yellow: Color,
    pub starlight: Color,
    pub border: Color,
    pub text: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::cosmic()
    }
}

impl Theme {
    pub fn cosmic() -> Self {
        Self {
            name: "Cosmic".to_string(),
            title_gradient: vec![
                Color::Rgb(72, 61, 139),
                Color::Rgb(93, 63, 211),
                Color::Rgb(138, 43, 226),
                Color::Rgb(139, 69, 255),
                Color::Rgb(171, 130, 255),
                Color::Rgb(189, 183, 255),
            ],
            celestial_blue: Color::Rgb(0, 191, 255),
            aurora_green: Color::Rgb(0, 255, 127),
            comet_orange: Color::Rgb(255, 140, 0),
            galaxy_pink: Color::Rgb(255, 20, 147),
            meteor_red: Color::Rgb(255, 69, 0),
            nebula_purple: Color::Rgb(147, 112, 219),
            plasma_cyan: Color::Rgb(0, 255, 255),
            solar_yellow: Color::Rgb(255, 255, 0),
            starlight: Color::Rgb(255, 255, 255),
            border: Color::Rgb(0, 191, 255),
            text: Color::Rgb(255, 255, 255),
            highlight_bg: Color::Rgb(0, 191, 255),
            highlight_fg: Color::Rgb(255, 255, 255),
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            title_gradient: vec![
                Color::Rgb(150, 150, 150),
                Color::Rgb(180, 180, 180),
                Color::Rgb(210, 210, 210),
                Color::Rgb(240, 240, 240),
            ],
            celestial_blue: Color::Rgb(100, 181, 246), // Lighter blue for dark bg
            aurora_green: Color::Rgb(129, 199, 132),   // Lighter green
            comet_orange: Color::Rgb(255, 183, 77),    // Lighter orange
            galaxy_pink: Color::Rgb(244, 143, 177),    // Lighter pink
            meteor_red: Color::Rgb(229, 115, 115),     // Lighter red
            nebula_purple: Color::Rgb(186, 104, 200),  // Lighter purple
            plasma_cyan: Color::Rgb(77, 208, 225),     // Lighter cyan
            solar_yellow: Color::Rgb(255, 238, 88),    // Bright yellow
            starlight: Color::Rgb(224, 224, 224),      // Light gray text
            border: Color::Rgb(81, 81, 81),            // Dark gray border
            text: Color::Rgb(224, 224, 224),           // Light text
            highlight_bg: Color::Rgb(66, 66, 66),      // Dark highlight bg
            highlight_fg: Color::Rgb(255, 255, 255),   // White highlight text
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            title_gradient: vec![
                Color::Rgb(0, 0, 0),
                Color::Rgb(50, 50, 50),
                Color::Rgb(100, 100, 100),
            ],
            celestial_blue: Color::Rgb(0, 86, 179), // Darker blue for better contrast
            aurora_green: Color::Rgb(21, 101, 192), // Darker green
            comet_orange: Color::Rgb(255, 152, 0),  // Darker orange
            galaxy_pink: Color::Rgb(216, 27, 96),   // Darker pink
            meteor_red: Color::Rgb(211, 47, 47),    // Darker red
            nebula_purple: Color::Rgb(69, 39, 160), // Darker purple
            plasma_cyan: Color::Rgb(0, 131, 143),   // Darker cyan
            solar_yellow: Color::Rgb(255, 193, 7),  // Keep bright for visibility
            starlight: Color::Rgb(33, 33, 33),      // Dark gray for text
            border: Color::Rgb(189, 189, 189),      // Light gray border
            text: Color::Rgb(33, 33, 33),           // Dark text
            highlight_bg: Color::Rgb(0, 123, 255),  // Bright blue highlight
            highlight_fg: Color::Rgb(255, 255, 255), // White text on highlight
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),
            title_gradient: vec![Color::Rgb(255, 255, 255), Color::Rgb(255, 255, 255)],
            celestial_blue: Color::Rgb(0, 0, 255),  // Pure blue
            aurora_green: Color::Rgb(0, 255, 0),    // Pure green
            comet_orange: Color::Rgb(255, 165, 0),  // Orange
            galaxy_pink: Color::Rgb(255, 0, 255),   // Magenta
            meteor_red: Color::Rgb(255, 0, 0),      // Pure red
            nebula_purple: Color::Rgb(128, 0, 128), // Purple
            plasma_cyan: Color::Rgb(0, 255, 255),   // Cyan
            solar_yellow: Color::Rgb(255, 255, 0),  // Yellow
            starlight: Color::Rgb(255, 255, 255),   // White text
            border: Color::Rgb(255, 255, 255),      // White borders
            text: Color::Rgb(255, 255, 255),        // White text
            highlight_bg: Color::Rgb(255, 255, 255), // White highlight bg
            highlight_fg: Color::Rgb(0, 0, 0),      // Black highlight text
        }
    }

    pub fn all_themes() -> Vec<Self> {
        vec![
            Self::cosmic(),
            Self::dark(),
            Self::light(),
            Self::high_contrast(),
        ]
    }
}
