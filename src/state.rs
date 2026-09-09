use crate::audio::clicker::ClickerSetting;
use crate::audio::player::{AudioPlayer, EqPreset, RepeatMode, ShuffleMode};
use crate::discord::DiscordDisplayMode;
use crate::library::model::{Song, VideoCategory};
use crate::stats::{ListeningSession, ListeningStats};
use crate::theme::{ChassisColor, ColorTarget, CustomThemeCategory, CustomThemeConfig, DisplayTheme, NamedColorPreset, ShellStyle, default_starter_presets};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NowPlayingSubState {
    Standard,
    FullArtwork,
    Lyrics,
    Visualizer,
}

impl NowPlayingSubState {
    pub fn next(&self) -> Self {
        match self {
            NowPlayingSubState::Standard => NowPlayingSubState::FullArtwork,
            NowPlayingSubState::FullArtwork => NowPlayingSubState::Lyrics,
            NowPlayingSubState::Lyrics => NowPlayingSubState::Visualizer,
            NowPlayingSubState::Visualizer => NowPlayingSubState::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuItem {
    Music,
    Videos,
    ShuffleSongs,
    NowPlaying,
    Settings,
    Extras,
}

impl MainMenuItem {
    pub fn title(&self) -> &'static str {
        match self {
            MainMenuItem::Music => "Music",
            MainMenuItem::Videos => "Videos",
            MainMenuItem::ShuffleSongs => "Shuffle Songs",
            MainMenuItem::NowPlaying => "Now Playing",
            MainMenuItem::Settings => "Settings",
            MainMenuItem::Extras => "Extras",
        }
    }

    pub fn has_arrow(&self) -> bool {
        match self {
            MainMenuItem::Music | MainMenuItem::Videos | MainMenuItem::NowPlaying | MainMenuItem::Settings | MainMenuItem::Extras => true,
            MainMenuItem::ShuffleSongs => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenView {
    MainMenu,
    MusicMenu,
    PlaylistsList,
    PlaylistDetail { playlist_idx: usize },
    ArtistsList,
    ArtistAlbums { artist_name: String },
    AlbumsList,
    AlbumDetail { album_key: String },
    SongsList,
    GenresList,
    GenreDetail { genre_name: String },
    ComposersList,
    ComposerDetail { composer_name: String },
    SearchScreen { query: String },
    NowPlaying,
    VideosMenu,
    VideosCategoryList { category: VideoCategory },
    VideoPlayerScreen { video_id: usize },
    SettingsMenu,
    SoftwareUpdate,
    AboutScreen,
    MusicSources,
    CustomEqEditor,
    DiscordSettings,
    MainMenuCustomizer,
    EqSelector,
    BacklightSelector,
    BrightnessAdjuster,
    ThemeSettings,
    ColorThemeSelector,
    CustomColorCategoryList,
    CustomColorTargetList { category: CustomThemeCategory },
    ColorPicker { target: ColorTarget },
    ColorPresetsManager,
    PresetOptions { preset_id: String },
    PresetNameInput { editing_preset_id: Option<String> },
    DisplayThemeSelector,
    ShellStyleSelector,
    AlbumCoverConfig,
    AlbumCoverBlurAdjuster,
    AlbumCoverBrightnessAdjuster,
    PlaylistNameInput { editing_playlist_idx: Option<usize> },
    PlaylistAddSongs { playlist_idx: usize, search_query: String },
    PlaylistOptions { playlist_idx: usize },
    PlaylistViewSelector,
    ExtrasMenu,
    QueueList,
    QueueOptions { queue_pos: usize },
    SongQueueOptions { song_id: usize },
    StatsHub,
    StatsSongs,
    StatsArtists,
    StatsAlbums,
    MusicQuiz,
    SleepTimer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlaylistViewMode {
    Default,
    Modern,
}

impl Default for PlaylistViewMode {
    fn default() -> Self {
        PlaylistViewMode::Default
    }
}

impl PlaylistViewMode {
    pub const ALL: [PlaylistViewMode; 2] = [PlaylistViewMode::Default, PlaylistViewMode::Modern];

    pub fn name(&self) -> &'static str {
        match self {
            PlaylistViewMode::Default => "Default (iPod)",
            PlaylistViewMode::Modern => "Modern (Spotify)",
        }
    }

    #[allow(dead_code)]
    pub fn next(&self) -> Self {
        match self {
            PlaylistViewMode::Default => PlaylistViewMode::Modern,
            PlaylistViewMode::Modern => PlaylistViewMode::Default,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MainMenuConfig {
    pub show_music: bool,
    pub show_videos: bool,
    pub show_shuffle_songs: bool,
    pub show_now_playing: bool,
    pub show_settings: bool,
}

impl Default for MainMenuConfig {
    fn default() -> Self {
        Self {
            show_music: true,
            show_videos: true,
            show_shuffle_songs: true,
            show_now_playing: true,
            show_settings: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BacklightDuration {
    Off,
    TwoSec,
    FiveSec,
    TenSec,
    TwentySec,
    AlwaysOn,
}

impl BacklightDuration {
    pub fn name(&self) -> &'static str {
        match self {
            BacklightDuration::Off => "Off",
            BacklightDuration::TwoSec => "2 Seconds",
            BacklightDuration::FiveSec => "5 Seconds",
            BacklightDuration::TenSec => "10 Seconds",
            BacklightDuration::TwentySec => "20 Seconds",
            BacklightDuration::AlwaysOn => "Always On",
        }
    }

    pub fn seconds(&self) -> f32 {
        match self {
            BacklightDuration::Off => 0.0,
            BacklightDuration::TwoSec => 2.0,
            BacklightDuration::FiveSec => 5.0,
            BacklightDuration::TenSec => 10.0,
            BacklightDuration::TwentySec => 20.0,
            BacklightDuration::AlwaysOn => f32::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepTimerMode {
    Off,
    FifteenMinutes,
    ThirtyMinutes,
    FortyFiveMinutes,
    SixtyMinutes,
    EndOfSong,
}

impl SleepTimerMode {
    pub const ALL: [Self; 6] = [
        Self::Off,
        Self::FifteenMinutes,
        Self::ThirtyMinutes,
        Self::FortyFiveMinutes,
        Self::SixtyMinutes,
        Self::EndOfSong,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::FifteenMinutes => "15 Minutes",
            Self::ThirtyMinutes => "30 Minutes",
            Self::FortyFiveMinutes => "45 Minutes",
            Self::SixtyMinutes => "60 Minutes",
            Self::EndOfSong => "End of Current Song",
        }
    }

    pub fn seconds(self) -> Option<f32> {
        match self {
            Self::Off | Self::EndOfSong => None,
            Self::FifteenMinutes => Some(15.0 * 60.0),
            Self::ThirtyMinutes => Some(30.0 * 60.0),
            Self::FortyFiveMinutes => Some(45.0 * 60.0),
            Self::SixtyMinutes => Some(60.0 * 60.0),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MusicQuizState {
    pub choices: Vec<usize>,
    pub correct_choice: usize,
    pub score: u32,
    pub streak: u32,
    pub high_score: u32,
    pub question_number: u32,
    pub remaining_sec: f32,
    pub revealed: Option<bool>,
    pub reveal_remaining_sec: f32,
    pub wrong_strikes: u32,
    pub restart_pending: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EpodSettingsConfig {
    #[serde(default)]
    pub chassis_color: ChassisColor,
    #[serde(default)]
    pub custom_theme: CustomThemeConfig,
    #[serde(default = "default_starter_presets")]
    pub custom_presets: Vec<NamedColorPreset>,
    #[serde(default)]
    pub active_preset_id: Option<String>,
    #[serde(default)]
    pub display_theme: DisplayTheme,
    #[serde(default)]
    pub shell_style: ShellStyle,
    #[serde(default)]
    pub playlist_view_mode: PlaylistViewMode,
    pub clicker_setting: ClickerSetting,
    pub backlight_timer: BacklightDuration,
    pub brightness_percent: u8,
    pub main_menu_config: MainMenuConfig,
    pub volume: f32,
    pub shuffle: ShuffleMode,
    pub repeat: RepeatMode,
    pub sound_check: bool,
    #[serde(default)]
    pub crossfade_seconds: u8,
    pub eq: EqPreset,
    pub custom_eq_bands: [f32; 10],
    #[serde(default = "default_true")]
    pub discord_enabled: bool,
    #[serde(default)]
    pub discord_display_mode: Option<DiscordDisplayMode>,
    #[serde(default)]
    pub discord_client_id: Option<String>,
    #[serde(default = "default_album_blur")]
    pub album_art_blur: u8,
    #[serde(default = "default_album_brightness")]
    pub album_art_brightness: u8,
    #[serde(default)]
    pub quiz_high_score: u32,
}

fn default_true() -> bool {
    true
}

fn default_album_blur() -> u8 {
    15
}

fn default_album_brightness() -> u8 {
    65
}

impl Default for EpodSettingsConfig {
    fn default() -> Self {
        Self {
            chassis_color: ChassisColor::White,
            custom_theme: CustomThemeConfig::default(),
            custom_presets: default_starter_presets(),
            active_preset_id: None,
            display_theme: DisplayTheme::Default,
            shell_style: ShellStyle::Default,
            playlist_view_mode: PlaylistViewMode::Default,
            clicker_setting: ClickerSetting::Speaker,
            backlight_timer: BacklightDuration::TenSec,
            brightness_percent: 85,
            main_menu_config: MainMenuConfig::default(),
            volume: 0.6875,
            shuffle: ShuffleMode::Off,
            repeat: RepeatMode::Off,
            sound_check: false,
            crossfade_seconds: 0,
            eq: EqPreset::Off,
            custom_eq_bands: [0.0; 10],
            discord_enabled: true,
            discord_display_mode: Some(DiscordDisplayMode::SongAndArtist),
            discord_client_id: Some("1540631275717656648".to_string()),
            album_art_blur: 15,
            album_art_brightness: 65,
            quiz_high_score: 0,
        }
    }
}

pub struct AppState {
    pub view_stack: Vec<ScreenView>,
    pub selected_indices: HashMap<String, usize>,
    pub current_queue: Vec<usize>, // song IDs
    pub current_queue_idx: usize,
    pub now_playing_substate: NowPlayingSubState,
    pub lyrics_scroll_offset: f32,
    pub selected_eq_band: usize, // 0 to 9 for 10-band EQ
    pub is_enlarged: bool,
    
    // Hardware & App State
    pub is_hold_locked: bool,
    pub chassis_color: ChassisColor,
    pub custom_theme: CustomThemeConfig,
    pub custom_presets: Vec<NamedColorPreset>,
    pub active_preset_id: Option<String>,
    pub preset_name_buffer: String,
    pub search_query: String,
    pub search_char_idx: usize,
    #[allow(dead_code)]
    pub search_mode: usize, // 0 = results list, 1 = character ribbon
    pub color_picker_mode: usize, // 0 = Saturation, 1 = Hue, 2 = Value
    pub display_theme: DisplayTheme,
    pub shell_style: ShellStyle,
    pub playlist_view_mode: PlaylistViewMode,
    pub playlist_name_buffer: String,
    pub clicker_setting: ClickerSetting,
    pub backlight_timer: BacklightDuration,
    pub backlight_remaining_sec: f32,
    pub brightness_percent: u8, // 0 to 100
    pub main_menu_config: MainMenuConfig,
    pub discord_enabled: bool,
    pub discord_display_mode: DiscordDisplayMode,
    pub discord_client_id: String,
    pub album_art_blur: u8,
    pub album_art_brightness: u8,
    
    // UI Popups & Overlays
    pub volume_hud_timer: f32,
    pub alphabet_hud_timer: f32,
    pub alphabet_hud_char: char,
    pub status_message: Option<(String, f32)>, // message, remaining time

    // Queue, listening statistics, quiz, and sleep timer state
    pub listening_stats: ListeningStats,
    pub listening_session: Option<ListeningSession>,
    pub stats_save_accum: f32,
    pub music_quiz: MusicQuizState,
    pub quiz_saved_queue: Vec<usize>,
    pub quiz_saved_queue_idx: usize,
    pub quiz_saved_position_sec: f32,
    pub quiz_saved_was_playing: bool,
    pub sleep_timer_mode: SleepTimerMode,
    pub sleep_timer_remaining_sec: f32,
    pub pre_sleep_volume: Option<f32>,
    pub metadata_repair_requested: bool,
    pub update_status: crate::updater::UpdateStatus,
    pub update_tx: Option<std::sync::mpsc::Sender<crate::updater::UpdateStatus>>,

    // Timing of the last PREVIOUS press for restart vs. go-back detection
    pub last_previous_press: Option<std::time::Instant>,
    // Live LCD geometry/hover snapshot so wheel buttons can react to hovered rows
    pub lcd_hover_pos: Option<(f32, f32)>,
    pub lcd_view_rect: (f32, f32, f32, f32),
    pub lcd_scale: f32,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            view_stack: vec![ScreenView::MainMenu],
            selected_indices: HashMap::new(),
            current_queue: Vec::new(),
            current_queue_idx: 0,
            now_playing_substate: NowPlayingSubState::Standard,
            lyrics_scroll_offset: 0.0,
            selected_eq_band: 0,
            is_enlarged: false,
            
            is_hold_locked: false,
            chassis_color: ChassisColor::White,
            custom_theme: CustomThemeConfig::default(),
            custom_presets: default_starter_presets(),
            active_preset_id: None,
            preset_name_buffer: String::new(),
            search_query: String::new(),
            search_char_idx: 0,
            search_mode: 0,
            color_picker_mode: 0,
            display_theme: DisplayTheme::Default,
            shell_style: ShellStyle::Default,
            playlist_view_mode: PlaylistViewMode::Default,
            playlist_name_buffer: String::new(),
            clicker_setting: ClickerSetting::Speaker,
            backlight_timer: BacklightDuration::TenSec,
            backlight_remaining_sec: 10.0,
            brightness_percent: 85,
            main_menu_config: MainMenuConfig::default(),
            discord_enabled: true,
            discord_display_mode: DiscordDisplayMode::SongAndArtist,
            discord_client_id: "1540631275717656648".to_string(),
            album_art_blur: 15,
            album_art_brightness: 65,
            
            volume_hud_timer: 0.0,
            alphabet_hud_timer: 0.0,
            alphabet_hud_char: 'A',
            status_message: None,

            listening_stats: ListeningStats::load(),
            listening_session: None,
            stats_save_accum: 0.0,
            music_quiz: MusicQuizState::default(),
            quiz_saved_queue: Vec::new(),
            quiz_saved_queue_idx: 0,
            quiz_saved_position_sec: 0.0,
            quiz_saved_was_playing: false,
            sleep_timer_mode: SleepTimerMode::Off,
            sleep_timer_remaining_sec: 0.0,
            pre_sleep_volume: None,
            metadata_repair_requested: false,
            update_status: crate::updater::UpdateStatus::Idle,
            update_tx: None,
            last_previous_press: None,
            lcd_hover_pos: None,
            lcd_view_rect: (0.0, 0.0, 100.0, 100.0),
            lcd_scale: 1.0,
        }
    }

    fn settings_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                paths.push(parent.join("epod_settings.json"));
            }
        }
        paths.push(PathBuf::from("epod_settings.json"));
        paths.push(PathBuf::from("target/release/epod_settings.json"));
        paths
    }

    pub fn load_saved_settings(&mut self, player: &mut AudioPlayer) {
        for path in Self::settings_paths() {
            if path.exists() {
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(cfg) = serde_json::from_reader::<_, EpodSettingsConfig>(file) {
                        self.chassis_color = cfg.chassis_color;
                        self.custom_theme = cfg.custom_theme;
                        self.custom_presets = cfg.custom_presets;
                        self.active_preset_id = cfg.active_preset_id;
                        self.display_theme = cfg.display_theme;
                        self.shell_style = cfg.shell_style;
                        self.playlist_view_mode = cfg.playlist_view_mode;
                        self.clicker_setting = cfg.clicker_setting;
                        self.backlight_timer = cfg.backlight_timer;
                        self.backlight_remaining_sec = cfg.backlight_timer.seconds();
                        self.brightness_percent = cfg.brightness_percent;
                        self.main_menu_config = cfg.main_menu_config;
                        self.discord_enabled = cfg.discord_enabled;
                        self.discord_display_mode = cfg.discord_display_mode.unwrap_or(DiscordDisplayMode::SongAndArtist);
                        if let Some(id) = cfg.discord_client_id {
                            if !id.trim().is_empty() {
                                self.discord_client_id = id;
                            }
                        }
                        self.album_art_blur = cfg.album_art_blur;
                        self.album_art_brightness = cfg.album_art_brightness;
                        self.music_quiz.high_score = cfg.quiz_high_score;
                        
                        player.volume = cfg.volume;
                        player.shuffle = cfg.shuffle;
                        player.repeat = cfg.repeat;
                        player.sound_check = cfg.sound_check;
                        player.crossfade_seconds = cfg.crossfade_seconds;
                        player.eq = cfg.eq;
                        player.custom_eq_bands = cfg.custom_eq_bands;
                        player.set_volume(player.volume);
                        break;
                    }
                }
            }
        }
    }

    pub fn save_settings(&self, player: &AudioPlayer) {
        let cfg = EpodSettingsConfig {
            chassis_color: self.chassis_color,
            custom_theme: self.custom_theme.clone(),
            custom_presets: self.custom_presets.clone(),
            active_preset_id: self.active_preset_id.clone(),
            display_theme: self.display_theme,
            shell_style: self.shell_style,
            playlist_view_mode: self.playlist_view_mode,
            clicker_setting: self.clicker_setting,
            backlight_timer: self.backlight_timer,
            brightness_percent: self.brightness_percent,
            main_menu_config: self.main_menu_config.clone(),
            volume: player.volume,
            shuffle: player.shuffle,
            repeat: player.repeat,
            sound_check: player.sound_check,
            crossfade_seconds: player.crossfade_seconds,
            eq: player.eq,
            custom_eq_bands: player.custom_eq_bands,
            discord_enabled: self.discord_enabled,
            discord_display_mode: Some(self.discord_display_mode),
            discord_client_id: Some(self.discord_client_id.clone()),
            album_art_blur: self.album_art_blur,
            album_art_brightness: self.album_art_brightness,
            quiz_high_score: self.music_quiz.high_score,
        };
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let path = parent.join("epod_settings.json");
                if let Ok(file) = std::fs::File::create(path) {
                    let _ = serde_json::to_writer_pretty(file, &cfg);
                }
            }
        }
        if let Ok(file) = std::fs::File::create("epod_settings.json") {
            let _ = serde_json::to_writer_pretty(file, &cfg);
        }
    }

    pub fn get_active_main_menu_items(&self, player: &AudioPlayer) -> Vec<MainMenuItem> {
        let mut items = Vec::new();
        if self.main_menu_config.show_music {
            items.push(MainMenuItem::Music);
        }
        if self.main_menu_config.show_videos {
            items.push(MainMenuItem::Videos);
        }
        if self.main_menu_config.show_shuffle_songs {
            items.push(MainMenuItem::ShuffleSongs);
        }
        if self.main_menu_config.show_now_playing && (player.is_playing || player.current_time_sec > 0.0 || !self.current_queue.is_empty()) {
            items.push(MainMenuItem::NowPlaying);
        }
        if self.main_menu_config.show_settings {
            items.push(MainMenuItem::Settings);
        }
        items.push(MainMenuItem::Extras);
        items
    }

    pub fn current_view(&self) -> &ScreenView {
        self.view_stack.last().unwrap_or(&ScreenView::MainMenu)
    }

    pub fn current_view_mut(&mut self) -> &mut ScreenView {
        self.view_stack.last_mut().unwrap()
    }

    pub fn push_view(&mut self, view: ScreenView) {
        self.view_stack.push(view);
    }

    pub fn pop_view(&mut self) -> bool {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
            true
        } else {
            false
        }
    }

    pub fn get_selected_index(&self, view_key: &str) -> usize {
        *self.selected_indices.get(view_key).unwrap_or(&0)
    }

    pub fn set_selected_index(&mut self, view_key: &str, idx: usize) {
        self.selected_indices.insert(view_key.to_string(), idx);
    }

    pub fn move_selection(&mut self, view_key: &str, delta: i32, max_items: usize) {
        if max_items == 0 {
            self.set_selected_index(view_key, 0);
            return;
        }
        let current = self.get_selected_index(view_key) as i32;
        let new_idx = (current + delta).clamp(0, (max_items as i32) - 1) as usize;
        self.set_selected_index(view_key, new_idx);
    }

    pub fn trigger_user_activity(&mut self) {
        self.backlight_remaining_sec = self.backlight_timer.seconds();
    }

    pub fn show_volume_hud(&mut self) {
        self.volume_hud_timer = 1.8;
    }

    pub fn show_alphabet_hud(&mut self, c: char) {
        self.alphabet_hud_char = c;
        self.alphabet_hud_timer = 1.2;
    }

    pub fn set_status_message(&mut self, msg: &str, dur: f32) {
        self.status_message = Some((msg.to_string(), dur));
    }

    pub fn begin_listening_session(&mut self, song: &Song) {
        self.listening_session = Some(ListeningSession::from_song(song));
    }

    pub fn tick_listening_stats(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let mut play_to_record = None;
        if let Some(ref mut session) = self.listening_session {
            session.listened_sec += dt;
            let snapshot = session.clone();
            self.listening_stats.add_listening_time(&snapshot, dt);
            if !session.counted && session.listened_sec >= session.qualifying_seconds() {
                session.counted = true;
                play_to_record = Some(session.clone());
            }
        }
        if let Some(session) = play_to_record {
            self.listening_stats.record_play(&session);
        }
        self.stats_save_accum += dt;
        if self.stats_save_accum >= 10.0 {
            self.listening_stats.save();
            self.stats_save_accum = 0.0;
        }
    }

    pub fn finish_listening_session(&mut self, force_play: bool) {
        if let Some(mut session) = self.listening_session.take() {
            if !session.counted && (force_play || session.listened_sec >= session.qualifying_seconds()) {
                self.listening_stats.record_play(&session);
                session.counted = true;
            }
            self.listening_stats.save();
            self.stats_save_accum = 0.0;
        }
    }

    pub fn restart_listening_session(&mut self) {
        if let Some(mut session) = self.listening_session.take() {
            if !session.counted && session.listened_sec > 0.5 {
                self.listening_stats.record_play(&session);
            }
            session.listened_sec = 0.0;
            session.counted = false;
            self.listening_session = Some(session);
            self.listening_stats.save();
            self.stats_save_accum = 0.0;
        }
    }

    pub fn update_timers(&mut self, dt: f32) {
        if self.volume_hud_timer > 0.0 {
            self.volume_hud_timer = (self.volume_hud_timer - dt).max(0.0);
        }
        if self.alphabet_hud_timer > 0.0 {
            self.alphabet_hud_timer = (self.alphabet_hud_timer - dt).max(0.0);
        }
        if self.backlight_remaining_sec > 0.0 && self.backlight_timer != BacklightDuration::AlwaysOn {
            self.backlight_remaining_sec = (self.backlight_remaining_sec - dt).max(0.0);
        }
        if let Some((_, ref mut time)) = self.status_message {
            *time -= dt;
            if *time <= 0.0 {
                self.status_message = None;
            }
        }
    }
}
