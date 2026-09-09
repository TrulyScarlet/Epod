use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChassisColor {
    White,
    Black,
    U2Edition,
    Green,
    Red,
    Brown,
    Yellow,
    Blue,
    Custom,
    #[serde(other)]
    #[serde(skip_serializing)]
    Unknown,
}

impl Default for ChassisColor {
    fn default() -> Self {
        ChassisColor::White
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CustomThemeCategory {
    Chassis,
    Display,
    Typography,
    Accents,
}

impl CustomThemeCategory {
    pub const ALL: [CustomThemeCategory; 4] = [
        CustomThemeCategory::Chassis,
        CustomThemeCategory::Display,
        CustomThemeCategory::Typography,
        CustomThemeCategory::Accents,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            CustomThemeCategory::Chassis => "Chassis & Hardware",
            CustomThemeCategory::Display => "Display & Screen",
            CustomThemeCategory::Typography => "Text & Typography",
            CustomThemeCategory::Accents => "Accents & Highlights",
        }
    }

    pub fn targets(&self) -> &'static [ColorTarget] {
        match self {
            CustomThemeCategory::Chassis => &[
                ColorTarget::Shell,
                ColorTarget::Rim,
                ColorTarget::ScrollWheel,
                ColorTarget::CenterButton,
                ColorTarget::WheelText,
                ColorTarget::HoldSwitch,
            ],
            CustomThemeCategory::Display => &[
                ColorTarget::ScreenBg,
                ColorTarget::StatusBarBg,
                ColorTarget::StatusBarText,
                ColorTarget::Divider,
            ],
            CustomThemeCategory::Typography => &[
                ColorTarget::TitleText,
                ColorTarget::ArtistText,
                ColorTarget::SelectedText,
                ColorTarget::ArrowIndicator,
            ],
            CustomThemeCategory::Accents => &[
                ColorTarget::SelectionHighlight,
                ColorTarget::ProgressBar,
                ColorTarget::StarRating,
                ColorTarget::BatteryFill,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ColorTarget {
    // 1. Chassis & Hardware
    Shell,
    Rim,
    ScrollWheel,
    CenterButton,
    WheelText,
    HoldSwitch,

    // 2. Display & Screen
    ScreenBg,
    StatusBarBg,
    StatusBarText,
    Divider,

    // 3. Text & Typography
    TitleText,
    ArtistText,
    SelectedText,
    ArrowIndicator,

    // 4. Accents & Highlights
    SelectionHighlight,
    ProgressBar,
    StarRating,
    BatteryFill,
}

impl ColorTarget {
    #[allow(dead_code)]
    pub const ALL: [ColorTarget; 18] = [
        ColorTarget::Shell,
        ColorTarget::Rim,
        ColorTarget::ScrollWheel,
        ColorTarget::CenterButton,
        ColorTarget::WheelText,
        ColorTarget::HoldSwitch,
        ColorTarget::ScreenBg,
        ColorTarget::StatusBarBg,
        ColorTarget::StatusBarText,
        ColorTarget::Divider,
        ColorTarget::TitleText,
        ColorTarget::ArtistText,
        ColorTarget::SelectedText,
        ColorTarget::ArrowIndicator,
        ColorTarget::SelectionHighlight,
        ColorTarget::ProgressBar,
        ColorTarget::StarRating,
        ColorTarget::BatteryFill,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ColorTarget::Shell => "Shell / Faceplate",
            ColorTarget::Rim => "Metal Bevel Rim",
            ColorTarget::ScrollWheel => "Scroll Wheel Body",
            ColorTarget::CenterButton => "Center Select Button",
            ColorTarget::WheelText => "Wheel Lettering",
            ColorTarget::HoldSwitch => "Hold Switch / Jack",
            ColorTarget::ScreenBg => "LCD Background",
            ColorTarget::StatusBarBg => "Status Bar Header",
            ColorTarget::StatusBarText => "Status Bar Icons / Text",
            ColorTarget::Divider => "Split View Divider",
            ColorTarget::TitleText => "Title / Primary Text",
            ColorTarget::ArtistText => "Artist / Secondary Text",
            ColorTarget::SelectedText => "Selected Item Text",
            ColorTarget::ArrowIndicator => "Arrow Indicators (>)",
            ColorTarget::SelectionHighlight => "Selection Highlight",
            ColorTarget::ProgressBar => "Progress / Scrubber Bar",
            ColorTarget::StarRating => "Star Ratings (★)",
            ColorTarget::BatteryFill => "Battery Level Fill",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomThemeConfig {
    // 1. Chassis & Hardware
    #[serde(default = "default_custom_shell")]
    pub shell_color: [u8; 3],
    #[serde(default = "default_custom_rim")]
    pub rim_color: [u8; 3],
    #[serde(default = "default_custom_wheel")]
    pub wheel_color: [u8; 3],
    #[serde(default = "default_custom_button")]
    pub center_button_color: [u8; 3],
    #[serde(default = "default_custom_wheel_text")]
    pub wheel_text_color: [u8; 3],
    #[serde(default = "default_custom_hold")]
    pub hold_switch_color: [u8; 3],

    // 2. Display & Screen
    #[serde(default = "default_custom_screen_bg")]
    pub screen_bg_color: [u8; 3],
    #[serde(default = "default_custom_status_bg")]
    pub status_bar_bg_color: [u8; 3],
    #[serde(default = "default_custom_status_text")]
    pub status_bar_text_color: [u8; 3],
    #[serde(default = "default_custom_divider")]
    pub divider_color: [u8; 3],

    // 3. Text & Typography
    #[serde(default = "default_custom_title")]
    pub title_color: [u8; 3],
    #[serde(default = "default_custom_artist")]
    pub artist_color: [u8; 3],
    #[serde(default = "default_custom_selected_text")]
    pub selected_text_color: [u8; 3],
    #[serde(default = "default_custom_arrow")]
    pub arrow_color: [u8; 3],

    // 4. Accents & Highlights
    #[serde(default = "default_custom_selection")]
    pub selection_highlight_color: [u8; 3],
    #[serde(default = "default_custom_progress")]
    pub progress_bar_color: [u8; 3],
    #[serde(default = "default_custom_star")]
    pub star_rating_color: [u8; 3],
    #[serde(default = "default_custom_battery")]
    pub battery_fill_color: [u8; 3],
}

fn default_custom_shell() -> [u8; 3] { [70, 130, 215] }
fn default_custom_rim() -> [u8; 3] { [210, 215, 222] }
fn default_custom_wheel() -> [u8; 3] { [230, 240, 252] }
fn default_custom_button() -> [u8; 3] { [70, 130, 215] }
fn default_custom_wheel_text() -> [u8; 3] { [45, 95, 160] }
fn default_custom_hold() -> [u8; 3] { [245, 120, 30] }
fn default_custom_screen_bg() -> [u8; 3] { [255, 255, 255] }
fn default_custom_status_bg() -> [u8; 3] { [225, 230, 238] }
fn default_custom_status_text() -> [u8; 3] { [20, 20, 20] }
fn default_custom_divider() -> [u8; 3] { [218, 220, 225] }
fn default_custom_title() -> [u8; 3] { [20, 20, 25] }
fn default_custom_artist() -> [u8; 3] { [120, 125, 135] }
fn default_custom_selected_text() -> [u8; 3] { [255, 255, 255] }
fn default_custom_arrow() -> [u8; 3] { [140, 145, 155] }
fn default_custom_selection() -> [u8; 3] { [40, 120, 235] }
fn default_custom_progress() -> [u8; 3] { [40, 120, 235] }
fn default_custom_star() -> [u8; 3] { [40, 110, 230] }
fn default_custom_battery() -> [u8; 3] { [76, 217, 100] }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NamedColorPreset {
    pub id: String,
    pub name: String,
    pub config: CustomThemeConfig,
}

impl NamedColorPreset {
    pub fn new(name: String, config: CustomThemeConfig) -> Self {
        let id = format!("preset_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
        Self { id, name, config }
    }
}

pub fn default_starter_presets() -> Vec<NamedColorPreset> {
    vec![
        NamedColorPreset {
            id: "preset_cyberpunk".to_string(),
            name: "Cyberpunk Neon".to_string(),
            config: CustomThemeConfig {
                shell_color: [214, 0, 110],
                rim_color: [40, 20, 50],
                wheel_color: [0, 229, 255],
                center_button_color: [17, 0, 34],
                wheel_text_color: [17, 0, 34],
                hold_switch_color: [255, 230, 0],
                screen_bg_color: [15, 10, 28],
                status_bar_bg_color: [30, 15, 50],
                status_bar_text_color: [0, 229, 255],
                divider_color: [60, 25, 90],
                title_color: [255, 230, 0],
                artist_color: [0, 229, 255],
                selected_text_color: [255, 255, 255],
                arrow_color: [255, 0, 128],
                selection_highlight_color: [214, 0, 110],
                progress_bar_color: [0, 229, 255],
                star_rating_color: [255, 230, 0],
                battery_fill_color: [0, 229, 255],
            },
        },
        NamedColorPreset {
            id: "preset_pastel_mint".to_string(),
            name: "Pastel Mint".to_string(),
            config: CustomThemeConfig {
                shell_color: [126, 200, 167],
                rim_color: [210, 225, 218],
                wheel_color: [251, 247, 238],
                center_button_color: [126, 200, 167],
                wheel_text_color: [45, 95, 70],
                hold_switch_color: [245, 160, 90],
                screen_bg_color: [255, 255, 255],
                status_bar_bg_color: [225, 240, 232],
                status_bar_text_color: [30, 60, 45],
                divider_color: [210, 228, 218],
                title_color: [25, 55, 40],
                artist_color: [90, 130, 110],
                selected_text_color: [255, 255, 255],
                arrow_color: [90, 140, 115],
                selection_highlight_color: [75, 175, 135],
                progress_bar_color: [75, 175, 135],
                star_rating_color: [75, 175, 135],
                battery_fill_color: [75, 175, 135],
            },
        },
        NamedColorPreset {
            id: "preset_sunset_orange".to_string(),
            name: "Sunset Orange".to_string(),
            config: CustomThemeConfig {
                shell_color: [255, 107, 53],
                rim_color: [225, 200, 185],
                wheel_color: [255, 230, 167],
                center_button_color: [255, 107, 53],
                wheel_text_color: [140, 50, 20],
                hold_switch_color: [255, 200, 50],
                screen_bg_color: [255, 255, 255],
                status_bar_bg_color: [255, 235, 220],
                status_bar_text_color: [60, 25, 15],
                divider_color: [240, 215, 200],
                title_color: [45, 20, 12],
                artist_color: [140, 80, 55],
                selected_text_color: [255, 255, 255],
                arrow_color: [200, 80, 40],
                selection_highlight_color: [245, 95, 40],
                progress_bar_color: [245, 95, 40],
                star_rating_color: [255, 140, 30],
                battery_fill_color: [255, 150, 40],
            },
        },
        NamedColorPreset {
            id: "preset_midnight_stealth".to_string(),
            name: "Midnight Stealth".to_string(),
            config: CustomThemeConfig {
                shell_color: [18, 19, 22],
                rim_color: [45, 48, 55],
                wheel_color: [38, 42, 48],
                center_button_color: [18, 19, 22],
                wheel_text_color: [160, 165, 175],
                hold_switch_color: [230, 57, 70],
                screen_bg_color: [10, 10, 12],
                status_bar_bg_color: [22, 23, 28],
                status_bar_text_color: [240, 242, 248],
                divider_color: [40, 42, 50],
                title_color: [245, 246, 250],
                artist_color: [145, 150, 165],
                selected_text_color: [255, 255, 255],
                arrow_color: [160, 165, 175],
                selection_highlight_color: [230, 57, 70],
                progress_bar_color: [230, 57, 70],
                star_rating_color: [230, 57, 70],
                battery_fill_color: [76, 217, 100],
            },
        },
    ]
}

impl Default for CustomThemeConfig {
    fn default() -> Self {
        Self {
            shell_color: default_custom_shell(),
            rim_color: default_custom_rim(),
            wheel_color: default_custom_wheel(),
            center_button_color: default_custom_button(),
            wheel_text_color: default_custom_wheel_text(),
            hold_switch_color: default_custom_hold(),
            screen_bg_color: default_custom_screen_bg(),
            status_bar_bg_color: default_custom_status_bg(),
            status_bar_text_color: default_custom_status_text(),
            divider_color: default_custom_divider(),
            title_color: default_custom_title(),
            artist_color: default_custom_artist(),
            selected_text_color: default_custom_selected_text(),
            arrow_color: default_custom_arrow(),
            selection_highlight_color: default_custom_selection(),
            progress_bar_color: default_custom_progress(),
            star_rating_color: default_custom_star(),
            battery_fill_color: default_custom_battery(),
        }
    }
}

impl CustomThemeConfig {
    pub fn get_color(&self, target: ColorTarget) -> [u8; 3] {
        match target {
            ColorTarget::Shell => self.shell_color,
            ColorTarget::Rim => self.rim_color,
            ColorTarget::ScrollWheel => self.wheel_color,
            ColorTarget::CenterButton => self.center_button_color,
            ColorTarget::WheelText => self.wheel_text_color,
            ColorTarget::HoldSwitch => self.hold_switch_color,
            ColorTarget::ScreenBg => self.screen_bg_color,
            ColorTarget::StatusBarBg => self.status_bar_bg_color,
            ColorTarget::StatusBarText => self.status_bar_text_color,
            ColorTarget::Divider => self.divider_color,
            ColorTarget::TitleText => self.title_color,
            ColorTarget::ArtistText => self.artist_color,
            ColorTarget::SelectedText => self.selected_text_color,
            ColorTarget::ArrowIndicator => self.arrow_color,
            ColorTarget::SelectionHighlight => self.selection_highlight_color,
            ColorTarget::ProgressBar => self.progress_bar_color,
            ColorTarget::StarRating => self.star_rating_color,
            ColorTarget::BatteryFill => self.battery_fill_color,
        }
    }

    pub fn set_color(&mut self, target: ColorTarget, color: [u8; 3]) {
        match target {
            ColorTarget::Shell => self.shell_color = color,
            ColorTarget::Rim => self.rim_color = color,
            ColorTarget::ScrollWheel => self.wheel_color = color,
            ColorTarget::CenterButton => self.center_button_color = color,
            ColorTarget::WheelText => self.wheel_text_color = color,
            ColorTarget::HoldSwitch => self.hold_switch_color = color,
            ColorTarget::ScreenBg => self.screen_bg_color = color,
            ColorTarget::StatusBarBg => self.status_bar_bg_color = color,
            ColorTarget::StatusBarText => self.status_bar_text_color = color,
            ColorTarget::Divider => self.divider_color = color,
            ColorTarget::TitleText => self.title_color = color,
            ColorTarget::ArtistText => self.artist_color = color,
            ColorTarget::SelectedText => self.selected_text_color = color,
            ColorTarget::ArrowIndicator => self.arrow_color = color,
            ColorTarget::SelectionHighlight => self.selection_highlight_color = color,
            ColorTarget::ProgressBar => self.progress_bar_color = color,
            ColorTarget::StarRating => self.star_rating_color = color,
            ColorTarget::BatteryFill => self.battery_fill_color = color,
        }
    }

    pub fn get_color32(&self, target: ColorTarget) -> Color32 {
        let [r, g, b] = self.get_color(target);
        Color32::from_rgb(r, g, b)
    }

    pub fn get_hex(&self, target: ColorTarget) -> String {
        let [r, g, b] = self.get_color(target);
        rgb_to_hex(r, g, b)
    }
}

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let h = if delta <= 1e-5 {
        0.0
    } else if (max - rf).abs() < 1e-5 {
        60.0 * (((gf - bf) / delta).rem_euclid(6.0))
    } else if (max - gf).abs() < 1e-5 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let s = if max <= 1e-5 { 0.0 } else { delta / max };
    let v = max;
    (h, s, v)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let h_prime = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - ((h_prime.rem_euclid(2.0)) - 1.0).abs());
    let (r1, g1, b1) = if h_prime >= 0.0 && h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime >= 1.0 && h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime >= 2.0 && h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime >= 3.0 && h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime >= 4.0 && h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    [
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

#[allow(dead_code)]
pub fn hex_to_rgb(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

impl ChassisColor {
    pub const ALL: [ChassisColor; 9] = [
        ChassisColor::White,
        ChassisColor::Black,
        ChassisColor::U2Edition,
        ChassisColor::Green,
        ChassisColor::Red,
        ChassisColor::Brown,
        ChassisColor::Yellow,
        ChassisColor::Blue,
        ChassisColor::Custom,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ChassisColor::White => "White",
            ChassisColor::Black => "Black",
            ChassisColor::U2Edition => "U2 Special Edition",
            ChassisColor::Green => "Green",
            ChassisColor::Red => "Red",
            ChassisColor::Brown => "Brown",
            ChassisColor::Yellow => "Yellow",
            ChassisColor::Blue => "Blue",
            ChassisColor::Custom => "Custom",
            ChassisColor::Unknown => "White",
        }
    }

    pub fn is_dark(&self) -> bool {
        matches!(self, ChassisColor::Black | ChassisColor::U2Edition | ChassisColor::Brown | ChassisColor::Blue)
    }

    #[allow(dead_code)]
    pub fn is_custom(&self) -> bool {
        matches!(self, ChassisColor::Custom)
    }

    pub fn body_color_with_custom(&self, custom: &CustomThemeConfig) -> Color32 {
        match self {
            ChassisColor::White | ChassisColor::Unknown => Color32::from_rgb(245, 246, 248),
            ChassisColor::Black => Color32::from_rgb(22, 22, 24),
            ChassisColor::U2Edition => Color32::from_rgb(18, 18, 20),
            ChassisColor::Green => Color32::from_rgb(88, 172, 108),
            ChassisColor::Red => Color32::from_rgb(208, 30, 45),
            ChassisColor::Brown => Color32::from_rgb(105, 68, 54),
            ChassisColor::Yellow => Color32::from_rgb(238, 172, 34),
            ChassisColor::Blue => Color32::from_rgb(42, 118, 202),
            ChassisColor::Custom => Color32::from_rgb(custom.shell_color[0], custom.shell_color[1], custom.shell_color[2]),
        }
    }

    pub fn wheel_color_with_custom(&self, custom: &CustomThemeConfig) -> Color32 {
        match self {
            ChassisColor::White | ChassisColor::Unknown => Color32::from_rgb(225, 227, 230),
            ChassisColor::Black => Color32::from_rgb(45, 46, 50),
            ChassisColor::U2Edition => Color32::from_rgb(215, 30, 35),
            ChassisColor::Green => Color32::from_rgb(228, 244, 232),
            ChassisColor::Red => Color32::from_rgb(248, 248, 250),
            ChassisColor::Brown => Color32::from_rgb(236, 220, 208),
            ChassisColor::Yellow => Color32::from_rgb(255, 246, 222),
            ChassisColor::Blue => Color32::from_rgb(222, 238, 252),
            ChassisColor::Custom => Color32::from_rgb(custom.wheel_color[0], custom.wheel_color[1], custom.wheel_color[2]),
        }
    }

    pub fn wheel_text_color_with_custom(&self, custom: &CustomThemeConfig) -> Color32 {
        match self {
            ChassisColor::White | ChassisColor::Unknown => Color32::from_rgb(150, 153, 160),
            ChassisColor::Black => Color32::from_rgb(180, 182, 190),
            ChassisColor::U2Edition => Color32::from_rgb(255, 255, 255),
            ChassisColor::Green => Color32::from_rgb(60, 115, 75),
            ChassisColor::Red => Color32::from_rgb(180, 30, 42),
            ChassisColor::Brown => Color32::from_rgb(105, 68, 54),
            ChassisColor::Yellow => Color32::from_rgb(140, 98, 12),
            ChassisColor::Blue => Color32::from_rgb(28, 76, 138),
            ChassisColor::Custom => {
                let [r, g, b] = custom.wheel_color;
                let lum = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
                if lum > 140.0 {
                    Color32::from_rgb(
                        (r as f32 * 0.45).round() as u8,
                        (g as f32 * 0.45).round() as u8,
                        (b as f32 * 0.45).round() as u8,
                    )
                } else {
                    Color32::from_rgb(250, 250, 255)
                }
            }
        }
    }

    pub fn center_button_color_with_custom(&self, custom: &CustomThemeConfig) -> Color32 {
        match self {
            ChassisColor::White | ChassisColor::Unknown => Color32::from_rgb(248, 249, 250),
            ChassisColor::Black => Color32::from_rgb(22, 22, 24),
            ChassisColor::U2Edition => Color32::from_rgb(18, 18, 20),
            ChassisColor::Green => Color32::from_rgb(88, 172, 108),
            ChassisColor::Red => Color32::from_rgb(208, 30, 45),
            ChassisColor::Brown => Color32::from_rgb(105, 68, 54),
            ChassisColor::Yellow => Color32::from_rgb(238, 172, 34),
            ChassisColor::Blue => Color32::from_rgb(42, 118, 202),
            ChassisColor::Custom => Color32::from_rgb(custom.center_button_color[0], custom.center_button_color[1], custom.center_button_color[2]),
        }
    }

    #[allow(dead_code)]
    pub fn body_color(&self) -> Color32 {
        self.body_color_with_custom(&CustomThemeConfig::default())
    }

    #[allow(dead_code)]
    pub fn wheel_color(&self) -> Color32 {
        self.wheel_color_with_custom(&CustomThemeConfig::default())
    }

    #[allow(dead_code)]
    pub fn wheel_text_color(&self) -> Color32 {
        self.wheel_text_color_with_custom(&CustomThemeConfig::default())
    }

    #[allow(dead_code)]
    pub fn center_button_color(&self) -> Color32 {
        self.center_button_color_with_custom(&CustomThemeConfig::default())
    }

    pub fn rim_color_with_custom(&self, custom: &CustomThemeConfig) -> Color32 {
        match self {
            ChassisColor::Custom => Color32::from_rgb(custom.rim_color[0], custom.rim_color[1], custom.rim_color[2]),
            _ => Color32::from_rgb(210, 215, 222),
        }
    }

    #[allow(dead_code)]
    pub fn rim_color(&self) -> Color32 {
        Color32::from_rgb(210, 215, 222)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DisplayTheme {
    Default,
    NightmodeBlack,
    DarkGrey,
    AlbumCover,
    #[serde(other)]
    #[serde(skip_serializing)]
    Unknown,
}

impl Default for DisplayTheme {
    fn default() -> Self {
        DisplayTheme::Default
    }
}

impl DisplayTheme {
    pub const ALL: [DisplayTheme; 4] = [
        DisplayTheme::Default,
        DisplayTheme::NightmodeBlack,
        DisplayTheme::DarkGrey,
        DisplayTheme::AlbumCover,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            DisplayTheme::Default => "Default",
            DisplayTheme::NightmodeBlack => "Nightmode Black",
            DisplayTheme::DarkGrey => "Dark Grey",
            DisplayTheme::AlbumCover => "Album Cover",
            DisplayTheme::Unknown => "Default",
        }
    }

    pub fn is_dark(&self) -> bool {
        match self {
            DisplayTheme::Default | DisplayTheme::Unknown => false,
            DisplayTheme::NightmodeBlack | DisplayTheme::DarkGrey | DisplayTheme::AlbumCover => true,
        }
    }

    pub fn is_album_cover(&self) -> bool {
        matches!(self, DisplayTheme::AlbumCover)
    }

    pub fn bg_color(&self) -> Color32 {
        match self {
            DisplayTheme::Default | DisplayTheme::Unknown => Color32::from_rgb(255, 255, 255),
            DisplayTheme::NightmodeBlack => Color32::from_rgb(10, 10, 12),
            DisplayTheme::DarkGrey => Color32::from_rgb(32, 34, 39),
            DisplayTheme::AlbumCover => Color32::from_rgb(56, 60, 67),
        }
    }

    #[allow(dead_code)]
    pub fn text_primary(&self) -> Color32 {
        match self {
            DisplayTheme::AlbumCover => LcdPalette::ALBUM_COVER_TEXT_PRIMARY,
            DisplayTheme::NightmodeBlack | DisplayTheme::DarkGrey => Color32::from_rgb(250, 250, 255),
            DisplayTheme::Default | DisplayTheme::Unknown => LcdPalette::TEXT_BLACK,
        }
    }

    #[allow(dead_code)]
    pub fn text_secondary(&self) -> Color32 {
        match self {
            DisplayTheme::AlbumCover => LcdPalette::ALBUM_COVER_TEXT_SECONDARY,
            DisplayTheme::NightmodeBlack | DisplayTheme::DarkGrey => Color32::from_rgb(175, 180, 195),
            DisplayTheme::Default | DisplayTheme::Unknown => LcdPalette::TEXT_GRAY,
        }
    }

    #[allow(dead_code)]
    pub fn border(&self) -> Color32 {
        if self.is_dark() {
            Color32::from_white_alpha(45)
        } else {
            LcdPalette::BORDER_GRAY
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShellStyle {
    Default,
    PixelArt,
    #[serde(other)]
    #[serde(skip_serializing)]
    Unknown,
}

impl Default for ShellStyle {
    fn default() -> Self {
        ShellStyle::Default
    }
}

impl ShellStyle {
    pub const ALL: [ShellStyle; 2] = [
        ShellStyle::Default,
        ShellStyle::PixelArt,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ShellStyle::Default => "Default",
            ShellStyle::PixelArt => "Pixel Art",
            ShellStyle::Unknown => "Default",
        }
    }

    pub fn is_pixel(&self) -> bool {
        matches!(self, ShellStyle::PixelArt)
    }
}

// LCD 320x240 Color Palette
pub struct LcdPalette;

impl LcdPalette {
    #[allow(dead_code)]
    pub const BG_WHITE: Color32 = Color32::from_rgb(255, 255, 255);
    pub const TEXT_BLACK: Color32 = Color32::from_rgb(15, 15, 15);
    pub const TEXT_GRAY: Color32 = Color32::from_rgb(130, 133, 140);
    pub const TEXT_SELECTED: Color32 = Color32::from_rgb(255, 255, 255);
    pub const BORDER_GRAY: Color32 = Color32::from_rgb(218, 220, 225);

    // Warm ivory / cream & muted stone palette (like Humpty / Mitski screenshot)
    pub const ALBUM_COVER_TEXT_PRIMARY: Color32 = Color32::from_rgb(238, 232, 212); // #EEE8D4
    pub const ALBUM_COVER_TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 155, 140); // #A09B8C

    // Adaptive helpers (accepts is_dark bool)
    pub fn text_primary(is_dark: bool) -> Color32 {
        if is_dark {
            Self::ALBUM_COVER_TEXT_PRIMARY
        } else {
            Self::TEXT_BLACK
        }
    }

    pub fn text_secondary(is_dark: bool) -> Color32 {
        if is_dark {
            Self::ALBUM_COVER_TEXT_SECONDARY
        } else {
            Self::TEXT_GRAY
        }
    }

    #[allow(dead_code)]
    pub fn border(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_white_alpha(45)
        } else {
            Self::BORDER_GRAY
        }
    }
    
    // iPod Classic Blue Gloss Selection Gradient
    pub const SEL_TOP: Color32 = Color32::from_rgb(70, 145, 245);
    pub const SEL_BOTTOM: Color32 = Color32::from_rgb(15, 78, 192);
    
    // Status Bar Gradient
    pub const STATUS_TOP: Color32 = Color32::from_rgb(235, 238, 242);
    pub const STATUS_BOTTOM: Color32 = Color32::from_rgb(195, 200, 208);
    pub const STATUS_BORDER: Color32 = Color32::from_rgb(160, 165, 175);
    pub const STATUS_TEXT: Color32 = Color32::from_rgb(20, 20, 20);

    // Battery Colors
    pub const BATTERY_SHELL: Color32 = Color32::from_rgb(70, 75, 85);
    pub const BATTERY_FILL: Color32 = Color32::from_rgb(76, 217, 100);

    // Scrubber
    pub const SCRUBBER_BAR_BG: Color32 = Color32::from_rgb(200, 205, 215);
    pub const SCRUBBER_BAR_FILL: Color32 = Color32::from_rgb(40, 120, 235);
}
