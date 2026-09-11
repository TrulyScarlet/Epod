#![windows_subsystem = "windows"]

mod audio;
mod discord;
mod library;
mod metadata_fetcher;
mod online_lyrics;
mod state;
mod stats;
mod theme;
mod ui;
mod updater;
mod video;

use audio::clicker::ClickerAudio;
use audio::player::AudioPlayer;
use eframe::egui::{self, Key, ViewportBuilder};
use library::model::Library;
use library::scanner::{AsyncScanner, ScanProgress};
use state::{AppState, ScreenView};
use ui::wheel::{WheelAction, WheelButton};
use ui::EpodUi;
use video::engine::VideoPlayer;

struct EpodApp {
    state: AppState,
    library: Library,
    player: AudioPlayer,
    video_player: VideoPlayer,
    clicker: ClickerAudio,
    ui: EpodUi,
    async_scanner: Option<AsyncScanner>,
    update_rx: std::sync::mpsc::Receiver<updater::UpdateStatus>,
    last_frame_time: std::time::Instant,
}

impl EpodApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let library = Library::new();
        let mut player = AudioPlayer::new();
        let mut state = AppState::new();

        // Initialize Software Updater channel & background check
        let (update_tx, update_rx) = std::sync::mpsc::channel();
        state.update_tx = Some(update_tx.clone());
        updater::spawn_update_check(update_tx);

        // Instant Load Saved Settings (Theme, 10-Band EQ, Volume, Discord RPC, etc.)
        state.load_saved_settings(&mut player);

        // Start non-blocking background scanning for music sources
        let cached_tracks = library.cached_tracks_map();
        let async_scanner = Some(AsyncScanner::start(library.music_sources.clone(), cached_tracks));

        let ui = EpodUi::new(state.discord_enabled, state.discord_display_mode, state.discord_client_id.clone());

        Self {
            state,
            library,
            player,
            video_player: VideoPlayer::new(),
            clicker: ClickerAudio::new(),
            ui,
            async_scanner,
            update_rx,
            last_frame_time: std::time::Instant::now(),
        }
    }
}

impl eframe::App for EpodApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Transparent clear color so the Epod shell is the widget itself!
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;

        // Process incoming background scan results without UI delay
        if let Some(ref scanner) = self.async_scanner {
            let mut finished = false;
            while let Ok(msg) = scanner.receiver.try_recv() {
                match msg {
                    ScanProgress::Batch { songs, videos } => {
                        let songs_len = songs.len();
                        for mut s in songs {
                            // If this file already exists in library (e.g. modified), replace it
                            if let Some(ref path) = s.file_path {
                                if let Some(existing) = self.library.songs.iter_mut().find(|ex| ex.file_path.as_ref() == Some(path)) {
                                    s.id = existing.id;
                                    *existing = s;
                                    continue;
                                }
                            }
                            s.id = self.library.songs.len();
                            self.library.songs.push(s);
                        }
                        for mut v in videos {
                            if let Some(ref path) = v.file_path {
                                if let Some(existing) = self.library.videos.iter_mut().find(|ex| ex.file_path.as_ref() == Some(path)) {
                                    v.id = existing.id;
                                    *existing = v;
                                    continue;
                                }
                            }
                            v.id = self.library.videos.len();
                            self.library.videos.push(v);
                        }
                        if songs_len > 0 {
                            self.library.rebuild_indices();
                        }
                    }
                    ScanProgress::Finished { total_songs, removed_paths, .. } => {
                        let has_removed = !removed_paths.is_empty();
                        if has_removed {
                            self.library.songs.retain(|s| {
                                s.file_path.as_ref().map_or(true, |p| !removed_paths.contains(p))
                            });
                            for (idx, s) in self.library.songs.iter_mut().enumerate() {
                                s.id = idx;
                            }
                        }
                        self.library.rebuild_indices();
                        if total_songs > 0 || has_removed {
                            self.library.save_library_cache();
                            self.state.set_status_message(&format!("Library updated ({} new/updated)", total_songs), 2.5_f32);
                        }
                        finished = true;
                    }
                }
            }
            if finished {
                self.async_scanner = None;
            }
        }

        // Process incoming software updater events
        while let Ok(status) = self.update_rx.try_recv() {
            self.state.update_status = status;
        }

        if matches!(
            self.state.update_status,
            updater::UpdateStatus::Checking | updater::UpdateStatus::Downloading(_)
        ) {
            ctx.request_repaint();
        }

        // Process Global Keyboard Shortcuts
        let is_text_input_screen = match self.state.current_view() {
            ScreenView::SearchScreen { .. }
            | ScreenView::PresetNameInput { .. }
            | ScreenView::PlaylistNameInput { .. }
            | ScreenView::PlaylistAddSongs { .. } => true,
            _ => false,
        };

        ctx.input(|i| {
            if i.key_pressed(Key::H) && !is_text_input_screen {
                self.state.is_hold_locked = !self.state.is_hold_locked;
            }

            if !self.state.is_hold_locked {
                // Rotary Wheel Shortcuts
                if i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::CloseBracket) {
                    self.state.trigger_user_activity();
                    self.clicker.play_rotary_tick(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Tick(1),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                }
                if i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::OpenBracket) {
                    self.state.trigger_user_activity();
                    self.clicker.play_rotary_tick(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Tick(-1),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                }

                // Button Clicks
                if i.key_pressed(Key::Enter) {
                    self.state.trigger_user_activity();
                    self.clicker.play_button_click(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Click(WheelButton::Select),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                }
                // Only treat Backspace/Escape/M as Menu navigation when not actively typing on an input screen
                if !is_text_input_screen && (i.key_pressed(Key::Escape) || i.key_pressed(Key::Backspace) || i.key_pressed(Key::M)) {
                    self.state.trigger_user_activity();
                    self.clicker.play_button_click(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Click(WheelButton::Menu),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                } else if is_text_input_screen && i.key_pressed(Key::Escape) {
                    self.state.trigger_user_activity();
                    self.clicker.play_button_click(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Click(WheelButton::Menu),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                }
                if !is_text_input_screen && (i.key_pressed(Key::Space) || i.key_pressed(Key::P)) {
                    self.state.trigger_user_activity();
                    self.clicker.play_button_click(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Click(WheelButton::PlayPause),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                }
                if !is_text_input_screen && (i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::Comma)) {
                    self.state.trigger_user_activity();
                    self.clicker.play_button_click(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Click(WheelButton::Previous),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                }
                if !is_text_input_screen && (i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Period)) {
                    self.state.trigger_user_activity();
                    self.clicker.play_button_click(self.state.clicker_setting);
                    ui::handle_wheel_action(
                        WheelAction::Click(WheelButton::Next),
                        &mut self.state,
                        &mut self.library,
                        &mut self.player,
                        &mut self.video_player,
                    );
                }
            }
        });

        // Frameless widget panel with zero margins
        let frame = egui::Frame::none().fill(egui::Color32::TRANSPARENT);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            self.ui.update_and_render(
                ui,
                &mut self.state,
                &mut self.library,
                &mut self.player,
                &mut self.video_player,
                &self.clicker,
                dt,
            );
        });

        // NOTE: Repainting is event-driven inside update_and_render() via
        // request_repaint_after() — this keeps idle CPU usage near 0%.
    }
}

// Single instance Windows Mutex check
#[cfg(windows)]
fn ensure_single_instance() -> bool {
    extern "system" {
        fn CreateMutexW(lpMutexAttributes: *const std::ffi::c_void, bInitialOwner: i32, lpName: *const u16) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }
    const ERROR_ALREADY_EXISTS: u32 = 183;
    let mutex_name: Vec<u16> = "Global\\Epod_Unique_SingleInstance_Mutex\0".encode_utf16().collect();
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
        if handle.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            return false;
        }
    }
    true
}

#[cfg(not(windows))]
fn ensure_single_instance() -> bool {
    true
}

fn main() -> eframe::Result<()> {
    if !ensure_single_instance() {
        return Ok(());
    }

    // Pixel-art Epod app icon (generated sprite, see assets/themes/pixel)
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/themes/pixel/icon.png"))
        .expect("Failed to decode Epod app icon");

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([370.0, 606.0])
            .with_min_inner_size([320.0, 520.0])
            .with_title("Epod")
            .with_icon(std::sync::Arc::new(icon))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Epod",
        options,
        Box::new(|cc| Ok(Box::new(EpodApp::new(cc)))),
    )
}
