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
                Color::Rgb(48, 40, 92),
                Color::Rgb(62, 42, 140),
                Color::Rgb(92, 28, 150),
                Color::Rgb(93, 46, 170),
                Color::Rgb(114, 87, 170),
                Color::Rgb(126, 122, 170),
            ],
            celestial_blue: Color::Rgb(0, 120, 215),
            aurora_green: Color::Rgb(0, 200, 100),
            comet_orange: Color::Rgb(255, 100, 0),
            galaxy_pink: Color::Rgb(200, 16, 115),
            meteor_red: Color::Rgb(220, 40, 0),
            nebula_purple: Color::Rgb(115, 89, 175),
            plasma_cyan: Color::Rgb(0, 200, 200),
            solar_yellow: Color::Rgb(220, 220, 0),
            starlight: Color::Rgb(240, 240, 240),
            border: Color::Rgb(0, 120, 215),
            text: Color::Rgb(240, 240, 240),
            highlight_bg: Color::Rgb(0, 120, 215),
            highlight_fg: Color::Rgb(255, 255, 255),
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            title_gradient: vec![
                Color::Rgb(120, 120, 120),
                Color::Rgb(160, 160, 160),
                Color::Rgb(200, 200, 200),
                Color::Rgb(240, 240, 240),
            ],
            celestial_blue: Color::Rgb(70, 150, 230),
            aurora_green: Color::Rgb(90, 180, 100),
            comet_orange: Color::Rgb(255, 150, 50),
            galaxy_pink: Color::Rgb(220, 100, 150),
            meteor_red: Color::Rgb(220, 80, 80),
            nebula_purple: Color::Rgb(150, 80, 180),
            plasma_cyan: Color::Rgb(50, 190, 210),
            solar_yellow: Color::Rgb(255, 220, 60),
            starlight: Color::Rgb(240, 240, 240),
            border: Color::Rgb(100, 100, 100),
            text: Color::Rgb(240, 240, 240),
            highlight_bg: Color::Rgb(80, 80, 80),
            highlight_fg: Color::Rgb(255, 255, 255),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            title_gradient: vec![
                Color::Rgb(0, 0, 0),
                Color::Rgb(40, 40, 40),
                Color::Rgb(80, 80, 80),
            ],
            celestial_blue: Color::Rgb(0, 60, 150),
            aurora_green: Color::Rgb(0, 120, 0),
            comet_orange: Color::Rgb(220, 100, 0),
            galaxy_pink: Color::Rgb(180, 20, 80),
            meteor_red: Color::Rgb(200, 30, 30),
            nebula_purple: Color::Rgb(100, 50, 160),
            plasma_cyan: Color::Rgb(0, 150, 150),
            solar_yellow: Color::Rgb(220, 180, 0),
            starlight: Color::Rgb(20, 20, 20),
            border: Color::Rgb(150, 150, 150),
            text: Color::Rgb(20, 20, 20),
            highlight_bg: Color::Rgb(0, 80, 220),
            highlight_fg: Color::Rgb(255, 255, 255),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),
            title_gradient: vec![Color::Rgb(255, 255, 255), Color::Rgb(255, 255, 255)],
            celestial_blue: Color::Rgb(0, 0, 255),
            aurora_green: Color::Rgb(0, 255, 0),
            comet_orange: Color::Rgb(255, 140, 0),
            galaxy_pink: Color::Rgb(255, 0, 255),
            meteor_red: Color::Rgb(255, 0, 0),
            nebula_purple: Color::Rgb(160, 0, 160),
            plasma_cyan: Color::Rgb(0, 255, 255),
            solar_yellow: Color::Rgb(255, 255, 0),
            starlight: Color::Rgb(255, 255, 255),
            border: Color::Rgb(255, 255, 255),
            text: Color::Rgb(255, 255, 255),
            highlight_bg: Color::Rgb(255, 255, 255),
            highlight_fg: Color::Rgb(0, 0, 0),
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
