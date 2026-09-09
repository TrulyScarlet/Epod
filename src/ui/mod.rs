pub mod art_cache;
pub mod body;
pub mod lcd;
pub mod pixel_body;
pub mod views;
pub mod wheel;

use crate::audio::clicker::ClickerAudio;
use crate::audio::player::{AudioPlayer, EqPreset, RepeatMode, ShuffleMode};
use crate::discord::{DiscordDisplayMode, DiscordRpc, DiscordTrackInfo};
use crate::library::model::{Library, VideoCategory};
use crate::library::scanner::LibraryScanner;
use crate::metadata_fetcher::{start_metadata_repair, MetadataRepairResult};
use crate::state::{AppState, BacklightDuration, MainMenuItem, NowPlayingSubState, PlaylistViewMode, ScreenView, SleepTimerMode};
use crate::theme::{ChassisColor, CustomThemeCategory, CustomThemeConfig, DisplayTheme, NamedColorPreset, ShellStyle, hsv_to_rgb, rgb_to_hsv};
use crate::video::engine::VideoPlayer;
use art_cache::ArtCache;
use egui::{Color32, Context, FontId, Painter, Pos2, Rect, ResizeDirection, Sense, Stroke, Ui, Vec2, ViewportCommand};
use lcd::LcdRenderer;
use views::menu_view::MenuView;
use views::now_playing_view::NowPlayingView;
use views::playlist_view::PlaylistView;
use views::settings_view::SettingsView;
use views::video_view::VideoView;
use wheel::{ClickWheelWidget, WheelAction, WheelButton};
use std::sync::mpsc::{Receiver, TryRecvError};

pub struct EpodUi {
    pub click_wheel: ClickWheelWidget,
    pub art_cache: ArtCache,
    pub discord_rpc: DiscordRpc,
    pub pixel_texture: Option<(ChassisColor, CustomThemeConfig, egui::TextureHandle)>,
    last_discord_snapshot: Option<DiscordSnapshot>,
    metadata_repair_receiver: Option<Receiver<MetadataRepairResult>>,
    metadata_repair_count: usize,
}

/// Cheap equality snapshot used to avoid spamming the Discord IPC worker every frame.
#[derive(Clone, PartialEq)]
struct DiscordSnapshot {
    enabled: bool,
    mode: DiscordDisplayMode,
    client_id: String,
    title: String,
    artist: String,
    album: String,
    is_playing: bool,
    elapsed_bucket: u32,
    duration_bucket: u32,
}

fn build_discord_snapshot(state: &AppState, library: &Library, player: &AudioPlayer) -> DiscordSnapshot {
    let (title, artist, album, duration) =
        if let Some(&song_id) = state.current_queue.get(state.current_queue_idx) {
            if let Some(song) = library.songs.get(song_id) {
                (
                    song.title.clone(),
                    song.artist.clone(),
                    song.album.clone(),
                    song.duration_sec,
                )
            } else {
                (String::new(), String::new(), String::new(), 0.0)
            }
        } else {
            (String::new(), String::new(), String::new(), 0.0)
        };

    DiscordSnapshot {
        enabled: state.discord_enabled,
        mode: state.discord_display_mode,
        client_id: state.discord_client_id.clone(),
        title,
        artist,
        album,
        is_playing: player.is_playing,
        elapsed_bucket: (player.current_time_sec / 2.0) as u32,
        duration_bucket: (duration / 2.0) as u32,
    }
}

impl EpodUi {
    pub fn new(discord_enabled: bool, discord_mode: DiscordDisplayMode, client_id: String) -> Self {
        Self {
            click_wheel: ClickWheelWidget::new(),
            art_cache: ArtCache::new(),
            discord_rpc: DiscordRpc::new(discord_enabled, discord_mode, client_id),
            pixel_texture: None,
            last_discord_snapshot: None,
            metadata_repair_receiver: None,
            metadata_repair_count: 0,
        }
    }

    pub fn update_and_render(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        library: &mut Library,
        player: &mut AudioPlayer,
        video_player: &mut VideoPlayer,
        clicker: &ClickerAudio,
        dt: f32,
    ) {
        state.update_timers(dt);

        if state.metadata_repair_requested && self.metadata_repair_receiver.is_none() {
            state.metadata_repair_requested = false;
            self.metadata_repair_count = 0;
            self.metadata_repair_receiver = Some(start_metadata_repair(&library.songs));
            state.set_status_message("Repairing missing metadata…", 3.0);
        }
        if let Some(receiver) = self.metadata_repair_receiver.take() {
            let mut finished = false;
            loop {
                match receiver.try_recv() {
                    Ok(result) => {
                        if let Some(song) = library.songs.get_mut(result.song_id) {
                            if let Some(artist) = result.artist { song.artist = artist; }
                            if let Some(album) = result.album { song.album = album; }
                            if let Some(art) = result.artwork_bytes { song.artwork_bytes = Some(art); }
                            if let Some(lyrics) = result.lyrics {
                                song.parsed_lyrics = crate::audio::metadata::parse_lrc_or_plain_lyrics(&lyrics);
                                song.lyrics = Some(lyrics);
                            }
                            self.metadata_repair_count += 1;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
            if finished {
                library.rebuild_indices();
                state.set_status_message(&format!("Metadata repair finished: {} tracks", self.metadata_repair_count), 4.0);
            } else {
                self.metadata_repair_receiver = Some(receiver);
            }
        }

        let was_playing = player.is_playing;
        if was_playing && !matches!(state.current_view(), ScreenView::MusicQuiz) {
            state.tick_listening_stats(dt);
        }

        let can_crossfade = state.sleep_timer_mode != SleepTimerMode::EndOfSong
            && player.repeat != RepeatMode::One
            && (state.current_queue_idx + 1 < state.current_queue.len()
                || (player.repeat == RepeatMode::All && !state.current_queue.is_empty()));
        if can_crossfade && player.should_start_crossfade() {
            state.finish_listening_session(true);
            handle_track_finished(state, library, player);
        }

        let track_finished = player.update();
        if track_finished {
            state.finish_listening_session(true);
            if state.sleep_timer_mode == crate::state::SleepTimerMode::EndOfSong {
                player.stop();
                state.sleep_timer_mode = crate::state::SleepTimerMode::Off;
                state.set_status_message("Sleep timer finished", 2.0);
            } else {
                handle_track_finished(state, library, player);
            }
        }

        if player.is_playing && state.sleep_timer_mode.seconds().is_some() {
            state.sleep_timer_remaining_sec = (state.sleep_timer_remaining_sec - dt).max(0.0);
            if state.sleep_timer_remaining_sec <= 30.0 {
                let original = *state.pre_sleep_volume.get_or_insert(player.volume);
                player.set_volume(original * (state.sleep_timer_remaining_sec / 30.0).clamp(0.0, 1.0));
            }
            if state.sleep_timer_remaining_sec <= 0.0 {
                player.pause();
                if let Some(original) = state.pre_sleep_volume.take() {
                    player.set_volume(original);
                }
                state.sleep_timer_mode = crate::state::SleepTimerMode::Off;
                state.set_status_message("Good night — playback paused", 3.0);
            }
        }

        if matches!(state.current_view(), ScreenView::MusicQuiz) {
            if state.music_quiz.revealed.is_some() {
                state.music_quiz.reveal_remaining_sec -= dt;
                if state.music_quiz.reveal_remaining_sec <= 0.0 {
                    // 3-strike rule: zero the score and start a fresh run
                    if state.music_quiz.restart_pending {
                        state.music_quiz.score = 0;
                        state.music_quiz.streak = 0;
                        state.music_quiz.question_number = 0;
                        state.music_quiz.wrong_strikes = 0;
                        state.music_quiz.restart_pending = false;
                        state.set_status_message("Quiz restarted — good luck!", 2.0);
                    }
                    start_music_quiz_question(state, library, player);
                }
            } else if !state.music_quiz.choices.is_empty() {
                state.music_quiz.remaining_sec = (state.music_quiz.remaining_sec - dt).max(0.0);
                if state.music_quiz.remaining_sec <= 0.0 {
                    register_quiz_wrong(state);
                }
            }
        }
        video_player.update();

        // Synchronize Discord Rich Presence state (only when something actually changed)
        self.discord_rpc.enabled = state.discord_enabled;
        self.discord_rpc.display_mode = state.discord_display_mode;
        self.discord_rpc.client_id = state.discord_client_id.clone();
        let snapshot = build_discord_snapshot(state, library, player);
        if self.last_discord_snapshot.as_ref() != Some(&snapshot) {
            update_discord_status(&self.discord_rpc, state, library, player);
            self.last_discord_snapshot = Some(snapshot);
        }

        // Check and handle custom edge/corner window resize dragging for borderless window
        handle_window_edge_resize(ui);

        // Epod fills the window completely with zero black borders
        let avail_rect = ui.available_rect_before_wrap();
        let body_rect = avail_rect;
        let body_w = body_rect.width();

        // Lazily generate + upload the Pixel Art chassis sprite for the active ChassisColor.
        // Includes the custom theme config in the cache key so custom color edits do not
        // regenerate the sprite every frame.
        if state.shell_style.is_pixel() {
            let needs_reload = match &self.pixel_texture {
                Some((color, custom, _)) => {
                    *color != state.chassis_color
                        || (state.chassis_color == ChassisColor::Custom && *custom != state.custom_theme)
                }
                None => true,
            };
            if needs_reload {
                let tex = load_pixel_body_texture(ui.ctx(), state.chassis_color, &state.custom_theme);
                self.pixel_texture = Some((state.chassis_color, state.custom_theme.clone(), tex));
            }
        }

        // 1. Render Epod Physical Chassis
        let pixel_tex_handle = self.pixel_texture.as_ref().map(|(_, _, tex)| tex);
        let (screen_rect, window_action) = body::BodyRenderer::render_chassis(
            ui,
            body_rect,
            state.chassis_color,
            &state.custom_theme,
            &mut state.is_hold_locked,
            pixel_tex_handle,
            state.shell_style.is_pixel(),
        );

        let screen_scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);

        // Click Wheel geometry (computed up-front so wheel-hover volume works globally)
        let wheel_avail_h = body_rect.max.y - screen_rect.max.y;
        let wheel_center_y = screen_rect.max.y + wheel_avail_h * 0.48_f32;
        let wheel_center = Pos2::new(body_rect.center().x, wheel_center_y);
        let wheel_outer_radius = (wheel_avail_h * 0.38_f32).min(body_w * 0.275_f32).max(25.0_f32);
        let wheel_center_radius = wheel_outer_radius * 0.37_f32;

        // Process Window Actions
        if let Some(act) = window_action {
            match act {
                body::WindowAction::StartDrag => {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                body::WindowAction::Minimize => {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                body::WindowAction::Maximize => {
                    state.is_enlarged = !state.is_enlarged;
                    if state.is_enlarged {
                        ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(480.0, 790.0)));
                    } else {
                        ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(370.0, 606.0)));
                    }
                }
                body::WindowAction::Close => {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                body::WindowAction::OpenSettings => {
                    state.push_view(ScreenView::SettingsMenu);
                    clicker.play_button_click(state.clicker_setting);
                }
            }
        }

        // Dedicated empty strip at the very bottom of the LCD so the mini back
        // button gets its own space and never overlaps list rows or content.
        let bottom_strip_h = (24.0_f32 * screen_scale).max(15.0_f32);
        let view_rect = Rect::from_min_max(
            screen_rect.min,
            Pos2::new(screen_rect.max.x, screen_rect.max.y - bottom_strip_h),
        );

        // 2. Render Virtual 4:3 LCD Screen (Scaled with chassis!)
        let painter = ui.painter().clone();
        let screen_painter = painter.with_clip_rect(screen_rect);
        lcd::LcdRenderer::draw_screen_background(&screen_painter, ui.ctx(), screen_rect, state, library, &mut self.art_cache);

        // Render Active Screen View with ArtCache (content ends above the back-button strip)
        render_active_view(&screen_painter, ui.ctx(), view_rect, state, library, player, video_player, &mut self.art_cache);

        // Pixel Art Theme: retro pixel-grid / scanline overlay on the LCD
        if state.shell_style.is_pixel() {
            lcd::LcdRenderer::draw_pixel_overlay(&screen_painter, screen_rect);
        }

        // Screen direct touch/mouse click & scroll interaction
        let screen_resp = ui.allocate_rect(screen_rect, Sense::click_and_drag());

        // Mouse wheel over the CENTER SELECT BUTTON adjusts volume (any screen,
        // whenever a track is loaded); scrolling anywhere else — LCD or wheel
        // ring — keeps navigating lists like rotary turns.
        if !state.is_hold_locked {
            let pointer = ui.input(|i| i.pointer.hover_pos());
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0_f32 {
                if let Some(pos) = pointer {
                    let dist = ((pos.x - wheel_center.x).powi(2) + (pos.y - wheel_center.y).powi(2)).sqrt();
                    if dist <= wheel_center_radius {
                        let vol_step = 1.0_f32 / 16.0_f32;
                        player.set_volume(player.volume + (if scroll_delta > 0.0 { -1.0 } else { 1.0 }) * vol_step);
                        state.show_volume_hud();
                        state.save_settings(player);
                        state.trigger_user_activity();
                        clicker.play_rotary_tick(state.clicker_setting);
                    } else if screen_resp.hovered() || dist <= wheel_outer_radius {
                        let ticks = if scroll_delta > 0.0_f32 { -1 } else { 1 };
                        state.trigger_user_activity();
                        clicker.play_rotary_tick(state.clicker_setting);
                        handle_wheel_action(WheelAction::Tick(ticks), state, library, player, video_player);
                    }
                }
            }
        }

        // Mini On-Screen Back Arrow Button Rect (inside its own bottom strip)
        let back_btn_w = 30.0_f32 * screen_scale;
        let back_btn_h = bottom_strip_h - 5.0_f32 * screen_scale;
        let back_btn_rect = Rect::from_min_size(
            Pos2::new(
                screen_rect.min.x + 6.0_f32 * screen_scale,
                screen_rect.max.y - bottom_strip_h + 2.5_f32 * screen_scale,
            ),
            Vec2::new(back_btn_w, back_btn_h),
        );

        // Press-based detection (far more reliable than click: works even when
        // the pointer moves slightly between press & release).
        let mut back_clicked = false;
        if state.view_stack.len() > 1 && !state.is_hold_locked {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if back_btn_rect.contains(pos) && ui.input(|i| i.pointer.primary_pressed()) {
                    back_clicked = true;
                }
            }
            if !back_clicked && screen_resp.clicked() {
                if let Some(pos) = screen_resp.interact_pointer_pos() {
                    if back_btn_rect.contains(pos) {
                        back_clicked = true;
                    }
                }
            }
        }

        let active_screen_rect = view_rect;

        if back_clicked {
            if matches!(state.current_view(), ScreenView::MusicQuiz) {
                restore_after_music_quiz(state, library, player);
            }
            if let ScreenView::VideoPlayerScreen { video_id } = state.current_view().clone() {
                if let Some(v) = library.videos.get_mut(video_id) {
                    v.resume_pos_sec = video_player.current_time_sec;
                }
                video_player.is_playing = false;
            }
            state.pop_view();
            clicker.play_button_click(state.clicker_setting);
        } else if !state.is_hold_locked {
            // View-specific interactions or generic list row clicks
            let curr_view = state.current_view().clone();
            match curr_view {
                ScreenView::ColorPicker { target } => {
                    if screen_resp.clicked() || screen_resp.dragged() {
                        if let Some(pos) = screen_resp.interact_pointer_pos() {
                            handle_color_picker_touch(pos, target, active_screen_rect, screen_scale, state, player, clicker, screen_resp.clicked());
                        }
                    }
                }
                ScreenView::PresetNameInput { ref editing_preset_id } => {
                    handle_preset_name_input_interaction(ui, &screen_resp, editing_preset_id.clone(), active_screen_rect, screen_scale, state, player, clicker);
                }
                ScreenView::PlaylistNameInput { editing_playlist_idx } => {
                    handle_playlist_name_input_interaction(ui, &screen_resp, editing_playlist_idx, active_screen_rect, screen_scale, state, library, player, clicker);
                }
                ScreenView::PlaylistAddSongs { playlist_idx, ref search_query } => {
                    handle_playlist_add_songs_interaction(ui, &screen_resp, playlist_idx, search_query.clone(), active_screen_rect, screen_scale, state, library, player, clicker);
                }
                ScreenView::PlaylistDetail { playlist_idx } if state.playlist_view_mode == PlaylistViewMode::Modern => {
                    if screen_resp.clicked() {
                        if let Some(pos) = screen_resp.interact_pointer_pos() {
                            handle_modern_playlist_detail_touch(pos, playlist_idx, active_screen_rect, screen_scale, state, library, player, &mut self.art_cache, clicker);
                        }
                    }
                }
                ScreenView::SearchScreen { .. } => {
                    handle_search_screen_interaction(ui, &screen_resp, active_screen_rect, screen_scale, state, library, player, clicker);
                }
                ScreenView::NowPlaying => {
                    if screen_resp.clicked() {
                        if let Some(pos) = screen_resp.interact_pointer_pos() {
                            let shuff_rect = views::now_playing_view::shuffle_btn_rect(active_screen_rect, screen_scale);
                            let rep_rect = views::now_playing_view::repeat_btn_rect(active_screen_rect, screen_scale);
                            if shuff_rect.contains(pos) {
                                player.shuffle = match player.shuffle {
                                    ShuffleMode::Off => ShuffleMode::Songs,
                                    _ => ShuffleMode::Off,
                                };
                                state.save_settings(player);
                                state.set_status_message(&format!("Shuffle: {}", player.shuffle.name()), 1.5);
                                clicker.play_button_click(state.clicker_setting);
                            } else if rep_rect.contains(pos) {
                                cycle_repeat_mode(player);
                                state.save_settings(player);
                                state.set_status_message(&format!("Repeat: {}", player.repeat.name()), 1.5);
                                clicker.play_button_click(state.clicker_setting);
                            } else {
                                handle_select_click(state, library, player, video_player);
                            }
                        }
                    }
                }
                _ => {
                    if screen_resp.clicked() {
                        if let Some(click_pos) = screen_resp.interact_pointer_pos() {
                            let bar_h = 20.0_f32 * screen_scale;
                            let item_h = match curr_view {
                                ScreenView::PlaylistsList if state.playlist_view_mode == PlaylistViewMode::Modern => 36.0_f32 * screen_scale,
                                ScreenView::PlaylistViewSelector => 32.0_f32 * screen_scale,
                                _ => 22.0_f32 * screen_scale,
                            };
                            if click_pos.y >= active_screen_rect.min.y + bar_h && click_pos.y <= active_screen_rect.max.y {
                                let rel_y = click_pos.y - (active_screen_rect.min.y + bar_h);
                                let visible_row = (rel_y / item_h).floor() as usize;
                                let visible_count = (((active_screen_rect.height() - bar_h) / item_h).floor() as usize).max(1);
                                let scroll_offset = match state.current_view() {
                                    ScreenView::SettingsMenu => state.get_selected_index("settings_menu").saturating_sub(visible_count - 1),
                                    ScreenView::QueueList => state.get_selected_index("queue_list").saturating_sub(visible_count - 1),
                                    ScreenView::StatsSongs => state.get_selected_index("stats_songs").saturating_sub(visible_count - 1),
                                    ScreenView::StatsArtists => state.get_selected_index("stats_artists").saturating_sub(visible_count - 1),
                                    ScreenView::StatsAlbums => state.get_selected_index("stats_albums").saturating_sub(visible_count - 1),
                                    _ => 0,
                                };
                                handle_screen_item_clicked(scroll_offset + visible_row, state, library, player, video_player, clicker);
                            }
                        }
                    }
                }
            }
        }

        // Draw Mini Back Arrow Badge
        if state.view_stack.len() > 1 {
            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
            let is_hov = pointer_pos.map(|p| back_btn_rect.contains(p)).unwrap_or(false);
            let bg_col = if is_hov {
                Color32::from_rgb(30, 110, 235)
            } else {
                Color32::from_rgba_premultiplied(40, 45, 55, 180)
            };
            painter.rect_filled(back_btn_rect, 3.0_f32 * screen_scale, bg_col);
            painter.rect_stroke(back_btn_rect, 3.0_f32 * screen_scale, Stroke::new((0.5_f32 * screen_scale).max(0.5_f32), Color32::from_white_alpha(80)));

            painter.text(
                back_btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                "◀",
                FontId::proportional(9.5_f32 * screen_scale),
                Color32::WHITE,
            );
        }

        // Render Volume HUD overlay if active
        if state.volume_hud_timer > 0.0 {
            lcd::LcdRenderer::draw_volume_hud(&screen_painter, screen_rect, player.volume);
        }

        // Render Alphabet HUD overlay if active
        if state.alphabet_hud_timer > 0.0 {
            lcd::LcdRenderer::draw_alphabet_hud(&screen_painter, screen_rect, state.alphabet_hud_char);
        }

        // Render Status Toast if active
        if let Some((ref msg, _)) = state.status_message {
            let toast_w = (220.0_f32 * screen_scale).min(screen_rect.width() - 16.0_f32 * screen_scale);
            let toast_rect = Rect::from_center_size(
                Pos2::new(screen_rect.center().x, screen_rect.max.y - 25.0_f32 * screen_scale),
                Vec2::new(toast_w, 22.0_f32 * screen_scale),
            );
            screen_painter.rect_filled(toast_rect, 4.0_f32 * screen_scale, Color32::from_black_alpha(200));
            screen_painter.text(
                toast_rect.center(),
                egui::Align2::CENTER_CENTER,
                msg,
                FontId::proportional(10.5_f32 * screen_scale),
                Color32::WHITE,
            );
        }

        // Screen Backlight & Brightness Shader Overlay
        lcd::LcdRenderer::apply_backlight_and_brightness(&screen_painter, screen_rect, state);

        // 3. Render Interactive Click Wheel (Scales proportionally — geometry precomputed)
        let actions = self.click_wheel.ui(
            ui,
            wheel_center,
            wheel_outer_radius,
            wheel_center_radius,
            state.chassis_color,
            &state.custom_theme,
            clicker,
            state.clicker_setting,
            state.is_hold_locked,
            state.shell_style.is_pixel(),
        );

        // Snapshot LCD geometry + hover position for wheel-button handlers
        // (used by hover-queueing with the NEXT button)
        state.lcd_scale = screen_scale;
        state.lcd_view_rect = (view_rect.min.x, view_rect.min.y, view_rect.width(), view_rect.height());
        state.lcd_hover_pos = ui.input(|i| i.pointer.hover_pos()).map(|p| (p.x, p.y));

        // 4. Dispatch Click Wheel Actions
        for action in actions {
            state.trigger_user_activity();
            handle_wheel_action(action, state, library, player, video_player);
        }

        // 5. Adaptive repaint scheduling — event-driven rendering for minimal CPU usage.
        //    Only requests frames when something is actively changing on screen.
        let mut interval: Option<std::time::Duration> = None;
        let mut consider = |dur_ms: u64| {
            let d = std::time::Duration::from_millis(dur_ms);
            interval = Some(match interval {
                Some(cur) => cur.min(d),
                None => d,
            });
        };

        // Video playback and the Now Playing spectrum need smooth frame updates
        if video_player.is_playing
            || (state.current_view() == &ScreenView::NowPlaying
                && state.now_playing_substate == NowPlayingSubState::Visualizer
                && player.is_playing)
        {
            consider(33);
        }
        if matches!(state.current_view(), ScreenView::MusicQuiz) {
            consider(100);
        }

        // Marquee ticker + multi-artist rotation detection
        let mut marquee_active = false;
        let is_artist_rotation_screen = matches!(
            state.current_view(),
            ScreenView::NowPlaying
                | ScreenView::MainMenu
                | ScreenView::MusicMenu
                | ScreenView::SongsList
                | ScreenView::SearchScreen { .. }
                | ScreenView::PlaylistDetail { .. }
                | ScreenView::PlaylistAddSongs { .. }
                | ScreenView::GenreDetail { .. }
                | ScreenView::ComposerDetail { .. }
                | ScreenView::ArtistAlbums { .. }
        );
        if is_artist_rotation_screen {
            consider(500);
        }

        let is_preview_screen = matches!(
            state.current_view(),
            ScreenView::NowPlaying | ScreenView::MainMenu | ScreenView::MusicMenu
        );
        if is_preview_screen {
            if let Some(&song_id) = state.current_queue.get(state.current_queue_idx) {
                if let Some(song) = library.songs.get(song_id) {
                    if LcdRenderer::split_artists(&song.artist).len() > 1 {
                        consider(500);
                    }
                    let ctx = ui.ctx();
                    let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
                    let avail_w = if state.current_view() == &ScreenView::NowPlaying {
                        screen_rect.width() * 0.42_f32
                    } else {
                        // Menu split-view preview pane (right 57% of screen)
                        screen_rect.width() * 0.50_f32
                    };
                    let overflows = |text: &str, size: f32| -> bool {
                        let font = FontId::proportional(size * scale);
                        ctx.fonts(|f| f.layout_no_wrap(text.to_string(), font, Color32::WHITE).size().x)
                            > avail_w
                    };
                    marquee_active = overflows(&song.title, 13.5_f32)
                        || overflows(&song.artist, 11.5_f32)
                        || overflows(&song.album, 10.5_f32);
                }
            }
        }

        if marquee_active {
            consider(40);
        } else if player.is_playing {
            // 10 FPS is plenty for playback progress bars and timers
            consider(100);
        }

        // Keep polling the asynchronous metadata repair worker while it is active.
        if self.metadata_repair_receiver.is_some() {
            consider(250);
        }

        // HUD overlays & toasts
        if state.volume_hud_timer > 0.0_f32
            || state.alphabet_hud_timer > 0.0_f32
            || state.status_message.is_some()
        {
            consider(100);
        }

        // Backlight countdown only needs slow updates
        if state.backlight_remaining_sec > 0.0_f32 && state.backlight_timer != BacklightDuration::AlwaysOn {
            consider(1000);
        }

        // Blinking text-input cursors
        let is_text_input_screen = matches!(
            state.current_view(),
            ScreenView::SearchScreen { .. }
                | ScreenView::PresetNameInput { .. }
                | ScreenView::PlaylistNameInput { .. }
                | ScreenView::PlaylistAddSongs { .. }
        );
        if is_text_input_screen {
            consider(400);
        }

        if let Some(dur) = interval {
            ui.ctx().request_repaint_after(dur);
        }
    }
}

/// Generate the pixel-art iPod 5G chassis sprite for the given chassis color into a GPU texture.
fn load_pixel_body_texture(ctx: &Context, chassis: ChassisColor, custom: &CustomThemeConfig) -> egui::TextureHandle {
    let color_image = pixel_body::generate_pixel_body(chassis, custom);
    let name = format!("pixel_body_{:?}", chassis);
    ctx.load_texture(name, color_image, egui::TextureOptions::NEAREST)
}

fn handle_color_picker_touch(
    pos: Pos2,
    target: crate::theme::ColorTarget,
    screen_rect: Rect,
    screen_scale: f32,
    state: &mut AppState,
    player: &AudioPlayer,
    clicker: &ClickerAudio,
    is_clicked: bool,
) {
    let bar_h = 20.0_f32 * screen_scale;
    let content_min_y = screen_rect.min.y + bar_h;
    let slider_x = screen_rect.min.x + 18.0_f32 * screen_scale;
    let slider_w = screen_rect.width() - 36.0_f32 * screen_scale;
    let slider_h = 10.0_f32 * screen_scale;

    // Mode buttons at top right: [Sat] [Hue] [Val]
    let mode_start_x = screen_rect.max.x - 110.0_f32 * screen_scale;
    let mode_y = content_min_y + 10.0_f32 * screen_scale + 2.0_f32 * screen_scale;
    for m_idx in 0..3 {
        let m_rect = Rect::from_min_size(
            Pos2::new(mode_start_x + (m_idx as f32) * 34.0_f32 * screen_scale, mode_y),
            Vec2::new(30.0_f32 * screen_scale, 18.0_f32 * screen_scale),
        );
        if m_rect.contains(pos) && is_clicked {
            state.color_picker_mode = m_idx;
            clicker.play_button_click(state.clicker_setting);
            return;
        }
    }

    let [r, g, b] = state.custom_theme.get_color(target);
    let (h, s, v) = rgb_to_hsv(r, g, b);

    // Hue spectrum bar
    let hue_y = content_min_y + 44.0_f32 * screen_scale;
    let hue_rect = Rect::from_min_size(Pos2::new(slider_x, hue_y), Vec2::new(slider_w, slider_h)).expand(4.0_f32 * screen_scale);
    if hue_rect.contains(pos) {
        let new_h = ((pos.x - slider_x) / slider_w).clamp(0.0, 1.0) * 360.0;
        let new_rgb = hsv_to_rgb(new_h, s.max(0.1), v.max(0.1));
        state.custom_theme.set_color(target, new_rgb);
        state.color_picker_mode = 1;
        state.save_settings(player);
        return;
    }

    // Saturation slider
    let sat_y = hue_y + 26.0_f32 * screen_scale;
    let sat_rect = Rect::from_min_size(Pos2::new(slider_x, sat_y), Vec2::new(slider_w, slider_h)).expand(4.0_f32 * screen_scale);
    if sat_rect.contains(pos) {
        let new_s = ((pos.x - slider_x) / slider_w).clamp(0.0, 1.0);
        let new_rgb = hsv_to_rgb(h, new_s, v.max(0.1));
        state.custom_theme.set_color(target, new_rgb);
        state.color_picker_mode = 0;
        state.save_settings(player);
        return;
    }

    // Brightness / Value slider
    let val_y = sat_y + 26.0_f32 * screen_scale;
    let val_rect = Rect::from_min_size(Pos2::new(slider_x, val_y), Vec2::new(slider_w, slider_h)).expand(4.0_f32 * screen_scale);
    if val_rect.contains(pos) {
        let new_v = ((pos.x - slider_x) / slider_w).clamp(0.0, 1.0);
        let new_rgb = hsv_to_rgb(h, s, new_v);
        state.custom_theme.set_color(target, new_rgb);
        state.color_picker_mode = 2;
        state.save_settings(player);
        return;
    }

    // Bottom quick swatches
    let swatches: [[u8; 3]; 9] = [
        [245, 246, 248], [22, 22, 24], [208, 30, 45],
        [238, 110, 25], [238, 172, 34], [88, 172, 108],
        [42, 118, 202], [145, 65, 215], [105, 68, 54],
    ];
    let swatch_spacing = slider_w / (swatches.len() as f32);
    let swatch_row_y = val_y + 24.0_f32 * screen_scale;
    for (i, swatch) in swatches.iter().enumerate() {
        let sc_center = Pos2::new(slider_x + (i as f32 + 0.5_f32) * swatch_spacing, swatch_row_y);
        let sc_rect = Rect::from_center_size(sc_center, Vec2::splat(18.0_f32 * screen_scale));
        if sc_rect.contains(pos) && is_clicked {
            state.custom_theme.set_color(target, *swatch);
            state.save_settings(player);
            clicker.play_button_click(state.clicker_setting);
            return;
        }
    }
}

fn handle_preset_name_input_interaction(
    ui: &mut Ui,
    screen_resp: &egui::Response,
    editing_preset_id: Option<String>,
    screen_rect: Rect,
    screen_scale: f32,
    state: &mut AppState,
    player: &AudioPlayer,
    clicker: &ClickerAudio,
) {
    let mut commit_save = false;
    let mut commit_cancel = false;

    // Keyboard typing
    ui.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Text(t) => {
                    for ch in t.chars() {
                        if !ch.is_control() && state.preset_name_buffer.len() < 24 {
                            state.preset_name_buffer.push(ch);
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    state.preset_name_buffer.pop();
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                    commit_save = true;
                }
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                    commit_cancel = true;
                }
                _ => {}
            }
        }
    });

    if screen_resp.clicked() {
        if let Some(pos) = screen_resp.interact_pointer_pos() {
            let bar_h = 20.0_f32 * screen_scale;
            let content_min_y = screen_rect.min.y + bar_h;

            // Suggestion chips
            let suggestions = ["Cyberpunk", "Pastel Mint", "Sunset", "Solar Flare", "Matcha", "Obsidian", "Lavender", "Cobalt"];
            let chip_h = 16.0_f32 * screen_scale;
            let mut curr_x = screen_rect.min.x + 18.0_f32 * screen_scale;
            let mut curr_y = content_min_y + 92.0_f32 * screen_scale;

            for s in suggestions {
                let chip_w = (s.len() as f32 * 6.2_f32 * screen_scale) + 12.0_f32 * screen_scale;
                if curr_x + chip_w > screen_rect.max.x - 18.0_f32 * screen_scale {
                    curr_x = screen_rect.min.x + 18.0_f32 * screen_scale;
                    curr_y += chip_h + 5.0_f32 * screen_scale;
                }
                let chip_rect = Rect::from_min_size(Pos2::new(curr_x, curr_y), Vec2::new(chip_w, chip_h));
                if chip_rect.contains(pos) {
                    state.preset_name_buffer = s.to_string();
                    clicker.play_button_click(state.clicker_setting);
                    return;
                }
                curr_x += chip_w + 5.0_f32 * screen_scale;
            }

            // Save & Cancel Buttons
            let btn_w = 88.0_f32 * screen_scale;
            let btn_h = 22.0_f32 * screen_scale;
            let btn_y = screen_rect.max.y - 24.0_f32 * screen_scale;

            let save_rect = Rect::from_center_size(
                Pos2::new(screen_rect.center().x - 52.0_f32 * screen_scale, btn_y),
                Vec2::new(btn_w, btn_h),
            );
            if save_rect.contains(pos) {
                commit_save = true;
            }

            let cancel_rect = Rect::from_center_size(
                Pos2::new(screen_rect.center().x + 52.0_f32 * screen_scale, btn_y),
                Vec2::new(btn_w, btn_h),
            );
            if cancel_rect.contains(pos) {
                commit_cancel = true;
            }
        }
    }

    if commit_save {
        let name = if state.preset_name_buffer.trim().is_empty() {
            format!("Custom Preset {}", state.custom_presets.len() + 1)
        } else {
            state.preset_name_buffer.trim().to_string()
        };

        if let Some(ref pid) = editing_preset_id {
            if let Some(preset) = state.custom_presets.iter_mut().find(|p| p.id == *pid) {
                preset.name = name.clone();
            }
            state.set_status_message(&format!("Preset renamed to '{}'", name), 1.8);
        } else {
            let new_preset = NamedColorPreset::new(name.clone(), state.custom_theme.clone());
            let new_id = new_preset.id.clone();
            state.custom_presets.push(new_preset);
            state.active_preset_id = Some(new_id);
            state.chassis_color = ChassisColor::Custom;
            state.set_status_message(&format!("Preset '{}' saved!", name), 1.8);
        }
        state.save_settings(player);
        clicker.play_button_click(state.clicker_setting);
        state.pop_view();
    } else if commit_cancel {
        clicker.play_button_click(state.clicker_setting);
        state.pop_view();
    }
}

fn handle_playlist_name_input_interaction(
    ui: &mut Ui,
    screen_resp: &egui::Response,
    editing_playlist_idx: Option<usize>,
    screen_rect: Rect,
    screen_scale: f32,
    state: &mut AppState,
    library: &mut Library,
    _player: &AudioPlayer,
    clicker: &ClickerAudio,
) {
    let mut commit_save = false;
    let mut commit_cancel = false;

    // Keyboard typing
    ui.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Text(t) => {
                    for ch in t.chars() {
                        if !ch.is_control() && state.playlist_name_buffer.len() < 30 {
                            state.playlist_name_buffer.push(ch);
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    state.playlist_name_buffer.pop();
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                    commit_save = true;
                }
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                    commit_cancel = true;
                }
                _ => {}
            }
        }
    });

    if screen_resp.clicked() {
        if let Some(pos) = screen_resp.interact_pointer_pos() {
            let bar_h = 20.0_f32 * screen_scale;
            let content_min_y = screen_rect.min.y + bar_h;

            // Suggestion chips
            let suggestions = ["My Favorites", "Workout Mix", "Chill Beats", "Night Vibes", "Road Trip", "Discover", "Party Jam"];
            let chip_h = 16.0_f32 * screen_scale;
            let mut curr_x = screen_rect.min.x + 18.0_f32 * screen_scale;
            let mut curr_y = content_min_y + 90.0_f32 * screen_scale;

            for s in suggestions {
                let chip_w = (s.len() as f32 * 6.2_f32 * screen_scale) + 12.0_f32 * screen_scale;
                if curr_x + chip_w > screen_rect.max.x - 18.0_f32 * screen_scale {
                    curr_x = screen_rect.min.x + 18.0_f32 * screen_scale;
                    curr_y += chip_h + 5.0_f32 * screen_scale;
                }
                let chip_rect = Rect::from_min_size(Pos2::new(curr_x, curr_y), Vec2::new(chip_w, chip_h));
                if chip_rect.contains(pos) {
                    state.playlist_name_buffer = s.to_string();
                    clicker.play_button_click(state.clicker_setting);
                    return;
                }
                curr_x += chip_w + 5.0_f32 * screen_scale;
            }

            // Save & Cancel Buttons
            let btn_w = 90.0_f32 * screen_scale;
            let btn_h = 22.0_f32 * screen_scale;
            let btn_y = screen_rect.max.y - 24.0_f32 * screen_scale;

            let save_rect = Rect::from_center_size(
                Pos2::new(screen_rect.center().x - 52.0_f32 * screen_scale, btn_y),
                Vec2::new(btn_w, btn_h),
            );
            if save_rect.contains(pos) {
                commit_save = true;
            }

            let cancel_rect = Rect::from_center_size(
                Pos2::new(screen_rect.center().x + 52.0_f32 * screen_scale, btn_y),
                Vec2::new(btn_w, btn_h),
            );
            if cancel_rect.contains(pos) {
                commit_cancel = true;
            }
        }
    }

    if commit_save {
        let name = if state.playlist_name_buffer.trim().is_empty() {
            format!("My Playlist {}", library.user_playlists.len() + 1)
        } else {
            state.playlist_name_buffer.trim().to_string()
        };

        if let Some(idx) = editing_playlist_idx {
            library.rename_playlist_by_idx(idx, name.clone());
            state.set_status_message(&format!("Playlist renamed to '{}'", name), 1.8);
            clicker.play_button_click(state.clicker_setting);
            state.pop_view();
        } else {
            let new_idx = library.create_user_playlist(name.clone());
            state.set_status_message(&format!("Playlist '{}' created!", name), 1.8);
            clicker.play_button_click(state.clicker_setting);
            state.pop_view();
            state.set_selected_index("playlist_detail", 0);
            state.push_view(ScreenView::PlaylistDetail { playlist_idx: new_idx });
        }
    } else if commit_cancel {
        clicker.play_button_click(state.clicker_setting);
        state.pop_view();
    }
}

fn handle_modern_playlist_detail_touch(
    pos: Pos2,
    playlist_idx: usize,
    screen_rect: Rect,
    screen_scale: f32,
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    art_cache: &mut ArtCache,
    clicker: &ClickerAudio,
) {
    let playlist = match library.playlists.get(playlist_idx) {
        Some(p) => p.clone(),
        None => return,
    };

    let bar_h = 20.0_f32 * screen_scale;
    let content_min_y = screen_rect.min.y + bar_h;
    let header_h = 68.0_f32 * screen_scale;
    let header_rect = Rect::from_min_size(
        Pos2::new(screen_rect.min.x + 6.0_f32 * screen_scale, content_min_y + 4.0_f32 * screen_scale),
        Vec2::new(screen_rect.width() - 12.0_f32 * screen_scale, header_h),
    );

    let cover_size = 56.0_f32 * screen_scale;
    let cover_rect = Rect::from_min_size(
        Pos2::new(header_rect.min.x + 6.0_f32 * screen_scale, header_rect.center().y - cover_size * 0.5),
        Vec2::splat(cover_size),
    );

    // If clicked on cover image itself -> open file picker to change cover
    if cover_rect.contains(pos) {
        clicker.play_button_click(state.clicker_setting);
        if let Some(file_path) = rfd::FileDialog::new()
            .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp"])
            .pick_file()
        {
            library.set_playlist_cover_by_idx(playlist_idx, Some(file_path));
            art_cache.invalidate_playlist_cover(&playlist.id);
            state.set_status_message("Playlist cover updated!", 2.0);
        }
        return;
    }

    let info_x = cover_rect.max.x + 8.0_f32 * screen_scale;
    let btn_y = header_rect.min.y + 46.0_f32 * screen_scale;
    let btn_h = 16.0_f32 * screen_scale;

    let play_btn_w = 42.0_f32 * screen_scale;
    let play_btn_rect = Rect::from_min_size(Pos2::new(info_x, btn_y), Vec2::new(play_btn_w, btn_h));

    let cover_btn_w = 30.0_f32 * screen_scale;
    let cover_btn_rect = Rect::from_min_size(Pos2::new(play_btn_rect.max.x + 4.0_f32 * screen_scale, btn_y), Vec2::new(cover_btn_w, btn_h));

    let add_btn_w = 26.0_f32 * screen_scale;
    let add_btn_rect = Rect::from_min_size(Pos2::new(cover_btn_rect.max.x + 4.0_f32 * screen_scale, btn_y), Vec2::new(add_btn_w, btn_h));

    let edit_btn_w = 26.0_f32 * screen_scale;
    let edit_btn_rect = Rect::from_min_size(Pos2::new(add_btn_rect.max.x + 4.0_f32 * screen_scale, btn_y), Vec2::new(edit_btn_w, btn_h));

    if play_btn_rect.contains(pos) {
        clicker.play_button_click(state.clicker_setting);
        if !playlist.song_ids.is_empty() {
            // Respect the global shuffle mode when starting playback
            if player.shuffle != ShuffleMode::Off {
                let mut ids = playlist.song_ids.clone();
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                ids.shuffle(&mut rng);
                state.current_queue = ids;
            } else {
                state.current_queue = playlist.song_ids.clone();
            }
            state.current_queue_idx = 0;
            play_current_queue_track(state, library, player);
            state.push_view(ScreenView::NowPlaying);
        }
        return;
    }

    if cover_btn_rect.contains(pos) {
        clicker.play_button_click(state.clicker_setting);
        if let Some(file_path) = rfd::FileDialog::new()
            .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp"])
            .pick_file()
        {
            library.set_playlist_cover_by_idx(playlist_idx, Some(file_path));
            art_cache.invalidate_playlist_cover(&playlist.id);
            state.set_status_message("Playlist cover updated!", 2.0);
        }
        return;
    }

    if add_btn_rect.contains(pos) {
        clicker.play_button_click(state.clicker_setting);
        state.set_selected_index("playlist_add_songs", 0);
        state.push_view(ScreenView::PlaylistAddSongs { playlist_idx, search_query: String::new() });
        return;
    }

    if edit_btn_rect.contains(pos) {
        clicker.play_button_click(state.clicker_setting);
        state.set_selected_index("playlist_options", 0);
        state.push_view(ScreenView::PlaylistOptions { playlist_idx });
        return;
    }

    // Track table click
    let table_top_y = header_rect.max.y + 4.0_f32 * screen_scale;
    let table_header_h = 14.0_f32 * screen_scale;
    let list_top_y = table_top_y + table_header_h + 2.0_f32 * screen_scale;

    if pos.y >= list_top_y && pos.y <= screen_rect.max.y {
        let row_h = 24.0_f32 * screen_scale;
        let rel_y = pos.y - list_top_y;
        let clicked_row = (rel_y / row_h).floor() as usize;

        let total_songs = playlist.song_ids.len();
        let list_h = screen_rect.max.y - list_top_y;
        let visible_count = ((list_h / row_h).floor() as usize).max(1);
        let sel = state.get_selected_index("playlist_detail").min(total_songs.saturating_sub(1));
        let scroll_offset = if sel >= visible_count {
            sel - visible_count + 1
        } else {
            0
        };

        let target_idx = scroll_offset + clicked_row;
        if target_idx < total_songs {
            let target_song_id = playlist.song_ids[target_idx];
            let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
            if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                clicker.play_button_click(state.clicker_setting);
                state.push_view(ScreenView::NowPlaying);
                return;
            }
            state.set_selected_index("playlist_detail", target_idx);
            state.current_queue = playlist.song_ids.clone();
            state.current_queue_idx = target_idx;
            play_current_queue_track(state, library, player);
            clicker.play_button_click(state.clicker_setting);
            state.push_view(ScreenView::NowPlaying);
        }
    }
}

fn handle_playlist_add_songs_interaction(
    ui: &mut Ui,
    screen_resp: &egui::Response,
    playlist_idx: usize,
    mut search_query: String,
    screen_rect: Rect,
    screen_scale: f32,
    state: &mut AppState,
    library: &mut Library,
    _player: &AudioPlayer,
    clicker: &ClickerAudio,
) {
    let mut query_changed = false;

    // Keyboard typing for filtering songs
    ui.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Text(t) => {
                    for ch in t.chars() {
                        if !ch.is_control() && search_query.len() < 30 {
                            search_query.push(ch);
                            query_changed = true;
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    search_query.pop();
                    query_changed = true;
                }
                _ => {}
            }
        }
    });

    if query_changed {
        state.set_selected_index("playlist_add_songs", 0);
        if let ScreenView::PlaylistAddSongs { search_query: ref mut sq, .. } = state.current_view_mut() {
            *sq = search_query.clone();
        }
    }

    if screen_resp.clicked() {
        if let Some(pos) = screen_resp.interact_pointer_pos() {
            let bar_h = 20.0_f32 * screen_scale;
            let content_min_y = screen_rect.min.y + bar_h;
            let search_h = 22.0_f32 * screen_scale;
            let search_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x + 6.0_f32 * screen_scale, content_min_y + 4.0_f32 * screen_scale),
                Vec2::new(screen_rect.width() - 12.0_f32 * screen_scale, search_h),
            );

            // Clear button [✕]
            if !search_query.is_empty() {
                let clear_rect = Rect::from_center_size(
                    Pos2::new(search_rect.max.x - 10.0_f32 * screen_scale, search_rect.center().y),
                    Vec2::splat(16.0_f32 * screen_scale),
                );
                if clear_rect.contains(pos) {
                    if let ScreenView::PlaylistAddSongs { search_query: ref mut sq, .. } = state.current_view_mut() {
                        sq.clear();
                    }
                    state.set_selected_index("playlist_add_songs", 0);
                    clicker.play_button_click(state.clicker_setting);
                    return;
                }
            }

            // Song list clicks
            let list_top_y = search_rect.max.y + 4.0_f32 * screen_scale;
            if pos.y >= list_top_y && pos.y <= screen_rect.max.y {
                let row_h = 22.0_f32 * screen_scale;
                let rel_y = pos.y - list_top_y;
                let clicked_row = (rel_y / row_h).floor() as usize;

                let filtered = views::playlist_view::PlaylistView::get_filtered_songs_for_add(library, &search_query);
                let total_songs = filtered.len();
                let list_h = screen_rect.max.y - list_top_y;
                let visible_count = ((list_h / row_h).floor() as usize).max(1);
                let sel = state.get_selected_index("playlist_add_songs").min(total_songs.saturating_sub(1));
                let scroll_offset = if sel >= visible_count {
                    sel - visible_count + 1
                } else {
                    0
                };

                let target_idx = scroll_offset + clicked_row;
                if target_idx < total_songs {
                    let song_id = filtered[target_idx];
                    library.toggle_song_in_playlist(playlist_idx, song_id);
                    state.set_selected_index("playlist_add_songs", target_idx);
                    clicker.play_button_click(state.clicker_setting);
                }
            }
        }
    }
}

fn handle_search_screen_interaction(
    ui: &mut Ui,
    screen_resp: &egui::Response,
    screen_rect: Rect,
    screen_scale: f32,
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    clicker: &ClickerAudio,
) {
    // Keyboard input for instant search typing
    ui.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Text(t) => {
                    for ch in t.chars() {
                        if !ch.is_control() && state.search_query.len() < 30 {
                            state.search_query.push(ch);
                            state.set_selected_index("search_screen", 0);
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    state.search_query.pop();
                    state.set_selected_index("search_screen", 0);
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                    let filtered = views::search_view::SearchView::get_filtered_songs(library, &state.search_query);
                    let sel = state.get_selected_index("search_screen").min(filtered.len().saturating_sub(1));
                    if filtered.get(sel).is_some() {
                        state.current_queue = filtered;
                        state.current_queue_idx = sel;
                        play_current_queue_track(state, library, player);
                        state.push_view(ScreenView::NowPlaying);
                        clicker.play_button_click(state.clicker_setting);
                    }
                }
                _ => {}
            }
        }
    });

    if screen_resp.clicked() {
        if let Some(pos) = screen_resp.interact_pointer_pos() {
            let bar_h = 20.0_f32 * screen_scale;
            let content_min_y = screen_rect.min.y + bar_h;
            let search_box_h = 22.0_f32 * screen_scale;
            let search_box_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x + 8.0_f32 * screen_scale, content_min_y + 4.0_f32 * screen_scale),
                Vec2::new(screen_rect.width() - 16.0_f32 * screen_scale, search_box_h),
            );

            // Clear button [✕]
            if !state.search_query.is_empty() {
                let clear_rect = Rect::from_center_size(
                    Pos2::new(search_box_rect.max.x - 12.0_f32 * screen_scale, search_box_rect.center().y),
                    Vec2::splat(18.0_f32 * screen_scale),
                );
                if clear_rect.contains(pos) {
                    state.search_query.clear();
                    state.set_selected_index("search_screen", 0);
                    clicker.play_button_click(state.clicker_setting);
                    return;
                }
            }

            // Character ribbon click
            let ribbon_y = search_box_rect.max.y + 3.0_f32 * screen_scale;
            let ribbon_h = 18.0_f32 * screen_scale;
            let ribbon_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x, ribbon_y),
                Vec2::new(screen_rect.width(), ribbon_h),
            );

            if ribbon_rect.contains(pos) {
                let char_w = 16.0_f32 * screen_scale;
                let visible_chars = ((screen_rect.width() / char_w).floor() as usize).min(views::search_view::SEARCH_CHARS.len());
                let selected_char = state.search_char_idx.min(views::search_view::SEARCH_CHARS.len() - 1);
                let char_scroll = if selected_char >= visible_chars {
                    selected_char - visible_chars + 1
                } else {
                    0
                };

                let rel_x = pos.x - screen_rect.min.x;
                let clicked_idx = char_scroll + (rel_x / char_w).floor() as usize;
                if clicked_idx < views::search_view::SEARCH_CHARS.len() {
                    state.search_char_idx = clicked_idx;
                    let ch = views::search_view::SEARCH_CHARS[clicked_idx];
                    match ch {
                        "␣" => {
                            if state.search_query.len() < 30 {
                                state.search_query.push(' ');
                            }
                        }
                        "⌫" => {
                            state.search_query.pop();
                        }
                        "Clear" => {
                            state.search_query.clear();
                        }
                        letter => {
                            if state.search_query.len() < 30 {
                                state.search_query.push_str(letter);
                            }
                        }
                    }
                    state.set_selected_index("search_screen", 0);
                    clicker.play_button_click(state.clicker_setting);
                }
                return;
            }

            // Results list click (above bottom back button area)
            let list_top_y = ribbon_rect.max.y + 2.0_f32 * screen_scale;
            let item_h = 22.0_f32 * screen_scale;
            if pos.y >= list_top_y && pos.y <= screen_rect.max.y {
                let filtered = views::search_view::SearchView::get_filtered_songs(library, &state.search_query);
                let visible_count = (((screen_rect.max.y - list_top_y) / item_h).floor() as usize).max(1);
                let selected_row = state.get_selected_index("search_screen").min(filtered.len().saturating_sub(1));
                let scroll_offset = if selected_row >= visible_count {
                    selected_row - visible_count + 1
                } else {
                    0
                };

                let rel_y = pos.y - list_top_y;
                let clicked_row = scroll_offset + (rel_y / item_h).floor() as usize;
                if clicked_row < filtered.len() {
                    state.current_queue = filtered;
                    state.current_queue_idx = clicked_row;
                    play_current_queue_track(state, library, player);
                    state.push_view(ScreenView::NowPlaying);
                    clicker.play_button_click(state.clicker_setting);
                }
            }
        }
    }
}

fn update_discord_status(
    discord: &DiscordRpc,
    state: &AppState,
    library: &Library,
    player: &AudioPlayer,
) {
    if !discord.enabled {
        discord.update_presence(None);
        return;
    }

    if let Some(&song_id) = state.current_queue.get(state.current_queue_idx) {
        if let Some(song) = library.songs.get(song_id) {
            discord.update_presence(Some(DiscordTrackInfo {
                title: song.title.clone(),
                artist: song.artist.clone(),
                album: song.album.clone(),
                lookup_title: song.title.clone(),
                lookup_artist: song.artist.clone(),
                status_display_type: 2,
                is_playing: player.is_playing,
                elapsed_sec: player.current_time_sec,
                duration_sec: song.duration_sec,
                client_id: state.discord_client_id.clone(),
            }));
            return;
        }
    }

    discord.update_presence(None);
}

// Window edge/corner drag resizing for frameless window
fn handle_window_edge_resize(ui: &mut Ui) {
    let screen_rect = ui.ctx().screen_rect();
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let margin = 10.0_f32;

    if let Some(pos) = pointer_pos {
        let on_left = pos.x <= screen_rect.min.x + margin;
        let on_right = pos.x >= screen_rect.max.x - margin;
        let on_top = pos.y <= screen_rect.min.y + margin;
        let on_bottom = pos.y >= screen_rect.max.y - margin;

        let resize_dir = match (on_left, on_right, on_top, on_bottom) {
            (true, _, true, _) => Some((ResizeDirection::NorthWest, egui::CursorIcon::ResizeNorthWest)),
            (_, true, true, _) => Some((ResizeDirection::NorthEast, egui::CursorIcon::ResizeNorthEast)),
            (true, _, _, true) => Some((ResizeDirection::SouthWest, egui::CursorIcon::ResizeSouthWest)),
            (_, true, _, true) => Some((ResizeDirection::SouthEast, egui::CursorIcon::ResizeSouthEast)),
            (true, _, _, _) => Some((ResizeDirection::West, egui::CursorIcon::ResizeWest)),
            (_, true, _, _) => Some((ResizeDirection::East, egui::CursorIcon::ResizeEast)),
            (_, _, true, _) => Some((ResizeDirection::North, egui::CursorIcon::ResizeNorth)),
            (_, _, _, true) => Some((ResizeDirection::South, egui::CursorIcon::ResizeSouth)),
            _ => None,
        };

        if let Some((dir, cursor)) = resize_dir {
            ui.output_mut(|o| o.cursor_icon = cursor);
            if ui.input(|i| i.pointer.any_pressed()) {
                ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        }
    }
}

fn handle_screen_item_clicked(
    clicked_row: usize,
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    video_player: &mut VideoPlayer,
    clicker: &ClickerAudio,
) {
    state.trigger_user_activity();
    clicker.play_button_click(state.clicker_setting);

    let curr_view = state.current_view().clone();
    match curr_view {
        ScreenView::MainMenu => {
            let items = state.get_active_main_menu_items(player);
            if clicked_row < items.len() {
                state.set_selected_index("main_menu", clicked_row);
                handle_select_click(state, library, player, video_player);
            }
        }
        ScreenView::MusicMenu => {
            state.set_selected_index("music_menu", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ExtrasMenu => {
            state.set_selected_index("extras_menu", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::QueueList => {
            state.set_selected_index("queue_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::QueueOptions { .. } => {
            state.set_selected_index("queue_options", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::SongQueueOptions { .. } => {
            state.set_selected_index("song_queue_options", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::StatsHub => {
            state.set_selected_index("stats_hub", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::StatsSongs => state.set_selected_index("stats_songs", clicked_row),
        ScreenView::StatsArtists => state.set_selected_index("stats_artists", clicked_row),
        ScreenView::StatsAlbums => state.set_selected_index("stats_albums", clicked_row),
        ScreenView::MusicQuiz => {
            state.set_selected_index("music_quiz", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::SleepTimer => {
            state.set_selected_index("sleep_timer", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::SettingsMenu => {
            state.set_selected_index("settings_menu", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::SoftwareUpdate => {
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::DiscordSettings => {
            state.set_selected_index("discord_settings", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::PlaylistsList => {
            state.set_selected_index("playlists_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::PlaylistDetail { .. } => {
            state.set_selected_index("playlist_detail", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::PlaylistOptions { .. } => {
            state.set_selected_index("playlist_options", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::PlaylistAddSongs { .. } => {
            state.set_selected_index("playlist_add_songs", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::PlaylistViewSelector => {
            state.set_selected_index("playlist_view_selector", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ArtistsList => {
            state.set_selected_index("artists_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ArtistAlbums { .. } => {
            state.set_selected_index("artist_albums", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::AlbumsList => {
            state.set_selected_index("albums_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::AlbumDetail { .. } => {
            state.set_selected_index("album_detail", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::SongsList => {
            state.set_selected_index("songs_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::GenresList => {
            state.set_selected_index("genres_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::GenreDetail { .. } => {
            state.set_selected_index("genre_detail", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ComposersList => {
            state.set_selected_index("composers_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ComposerDetail { .. } => {
            state.set_selected_index("composer_detail", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::MusicSources => {
            state.set_selected_index("music_sources", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::EqSelector => {
            state.set_selected_index("eq_selector", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::BacklightSelector => {
            state.set_selected_index("backlight_selector", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ThemeSettings => {
            state.set_selected_index("theme_settings", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ColorThemeSelector => {
            state.set_selected_index("color_theme_selector", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::CustomColorCategoryList => {
            state.set_selected_index("custom_color_category_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::CustomColorTargetList { .. } => {
            state.set_selected_index("custom_color_target_list", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ColorPresetsManager => {
            state.set_selected_index("color_presets_manager", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::PresetOptions { .. } => {
            state.set_selected_index("preset_options", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::DisplayThemeSelector => {
            state.set_selected_index("display_theme_selector", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::ShellStyleSelector => {
            state.set_selected_index("shell_style_selector", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::AlbumCoverConfig => {
            state.set_selected_index("album_cover_config", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::MainMenuCustomizer => {
            state.set_selected_index("main_menu_customizer", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::VideosMenu => {
            state.set_selected_index("videos_menu", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::VideosCategoryList { .. } => {
            state.set_selected_index("videos_category", clicked_row);
            handle_select_click(state, library, player, video_player);
        }
        ScreenView::NowPlaying => {
            handle_select_click(state, library, player, video_player);
        }
        _ => {}
    }
}

fn render_active_view(
    painter: &Painter,
    ctx: &Context,
    screen_rect: Rect,
    state: &mut AppState,
    library: &mut Library,
    player: &AudioPlayer,
    video_player: &VideoPlayer,
    art_cache: &mut ArtCache,
) {
    let curr_view = state.current_view().clone();

    match curr_view {
        ScreenView::MainMenu => {
            MenuView::render_main_menu(painter, ctx, screen_rect, state, library, player, art_cache);
        }
        ScreenView::MusicMenu => {
            MenuView::render_music_menu(painter, ctx, screen_rect, state, library, player, art_cache);
        }
        ScreenView::ExtrasMenu => {
            let items = vec![
                "Up Next / Queue".to_string(),
                "Listening Stats".to_string(),
                "Music Quiz".to_string(),
                "Sleep Timer".to_string(),
            ];
            let details = vec![
                format!("{} songs", state.current_queue.len()),
                format!("{} plays", state.listening_stats.total_plays),
                format!("High: {}", state.music_quiz.high_score),
                state.sleep_timer_mode.name().to_string(),
            ];
            let sel = state.get_selected_index("extras_menu").min(items.len() - 1);
            MenuView::render_full_list(painter, screen_rect, "Extras", &items, Some(&details), sel, true, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::QueueList => {
            let titles: Vec<String> = state.current_queue.iter().enumerate().filter_map(|(idx, &id)| {
                library.songs.get(id).map(|s| if idx == state.current_queue_idx { format!("▶ {}", s.title) } else { s.title.clone() })
            }).collect();
            let details: Vec<String> = state.current_queue.iter().filter_map(|&id| library.songs.get(id).map(|s| s.artist.clone())).collect();
            let sel = state.get_selected_index("queue_list").min(titles.len().saturating_sub(1));
            MenuView::render_full_list(painter, screen_rect, "Up Next", &titles, Some(&details), sel, true, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::QueueOptions { queue_pos } => {
            let song_name = state.current_queue.get(queue_pos).and_then(|&id| library.songs.get(id)).map(|s| s.title.as_str()).unwrap_or("Queue Song");
            let items = vec!["Play Now".to_string(), "Move Up".to_string(), "Move Down".to_string(), "Remove from Queue".to_string(), "Clear Queue".to_string()];
            let sel = state.get_selected_index("queue_options").min(items.len() - 1);
            MenuView::render_full_list(painter, screen_rect, song_name, &items, None, sel, true, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::SongQueueOptions { song_id } => {
            let song_name = library.songs.get(song_id).map(|s| s.title.as_str()).unwrap_or("Song");
            let items = vec!["Play Next".to_string(), "Add to End".to_string(), "Play Now".to_string()];
            let sel = state.get_selected_index("song_queue_options").min(items.len() - 1);
            MenuView::render_full_list(painter, screen_rect, song_name, &items, None, sel, true, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::StatsHub => {
            let hours = state.listening_stats.total_listened_seconds / 3600.0;
            let items = vec!["Top Songs".to_string(), "Top Artists".to_string(), "Top Albums".to_string(), "Total Listening".to_string()];
            let details = vec![
                state.listening_stats.top_tracks().first().map(|t| t.title.clone()).unwrap_or_else(|| "No data yet".to_string()),
                state.listening_stats.aggregate_artists().first().map(|r| r.0.clone()).unwrap_or_else(|| "No data yet".to_string()),
                state.listening_stats.aggregate_albums().first().map(|r| r.0.clone()).unwrap_or_else(|| "No data yet".to_string()),
                format!("{:.1}h • {} plays", hours, state.listening_stats.total_plays),
            ];
            let sel = state.get_selected_index("stats_hub").min(items.len() - 1);
            MenuView::render_full_list(painter, screen_rect, "Epod Wrapped", &items, Some(&details), sel, sel < 3, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::StatsSongs => {
            let tracks = state.listening_stats.top_tracks();
            let items: Vec<String> = tracks.iter().enumerate().map(|(i, t)| format!("{}. {}", i + 1, t.title)).collect();
            let details: Vec<String> = tracks.iter().map(|t| format!("{}× • {}m", t.play_count, (t.listened_seconds / 60.0).round() as u64)).collect();
            let sel = state.get_selected_index("stats_songs").min(items.len().saturating_sub(1));
            MenuView::render_full_list(painter, screen_rect, "Top Songs", &items, Some(&details), sel, false, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::StatsArtists => {
            let rows = state.listening_stats.aggregate_artists();
            let items: Vec<String> = rows.iter().enumerate().map(|(i, r)| format!("{}. {}", i + 1, r.0)).collect();
            let details: Vec<String> = rows.iter().map(|r| format!("{}× • {}m", r.1, (r.2 / 60.0).round() as u64)).collect();
            let sel = state.get_selected_index("stats_artists").min(items.len().saturating_sub(1));
            MenuView::render_full_list(painter, screen_rect, "Top Artists", &items, Some(&details), sel, false, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::StatsAlbums => {
            let rows = state.listening_stats.aggregate_albums();
            let items: Vec<String> = rows.iter().enumerate().map(|(i, r)| format!("{}. {}", i + 1, r.0)).collect();
            let details: Vec<String> = rows.iter().map(|r| format!("{}× • {}m", r.1, (r.2 / 60.0).round() as u64)).collect();
            let sel = state.get_selected_index("stats_albums").min(items.len().saturating_sub(1));
            MenuView::render_full_list(painter, screen_rect, "Top Albums", &items, Some(&details), sel, false, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::MusicQuiz => {
            render_music_quiz(painter, screen_rect, state, library, player);
        }
        ScreenView::SleepTimer => {
            let items: Vec<String> = SleepTimerMode::ALL.iter().map(|m| m.name().to_string()).collect();
            let details: Vec<String> = SleepTimerMode::ALL.iter().map(|m| if *m == state.sleep_timer_mode { "✔".to_string() } else { String::new() }).collect();
            let sel = state.get_selected_index("sleep_timer").min(items.len() - 1);
            MenuView::render_full_list(painter, screen_rect, "Sleep Timer", &items, Some(&details), sel, false, player, state.is_hold_locked, state.display_theme);
        }
        ScreenView::PlaylistsList => {
            PlaylistView::render_playlists_hub(painter, ctx, screen_rect, state, library, player, art_cache);
        }
        ScreenView::PlaylistDetail { playlist_idx } => {
            PlaylistView::render_playlist_detail(painter, ctx, screen_rect, playlist_idx, state, library, player, art_cache);
        }
        ScreenView::PlaylistNameInput { editing_playlist_idx } => {
            PlaylistView::render_playlist_name_input_screen(painter, screen_rect, editing_playlist_idx.is_some(), state, player);
        }
        ScreenView::PlaylistAddSongs { playlist_idx, ref search_query } => {
            PlaylistView::render_add_songs_screen(painter, screen_rect, playlist_idx, search_query, state, library, player);
        }
        ScreenView::PlaylistOptions { playlist_idx } => {
            PlaylistView::render_playlist_options_screen(painter, screen_rect, playlist_idx, state, library, player);
        }
        ScreenView::PlaylistViewSelector => {
            PlaylistView::render_playlist_view_selector(painter, screen_rect, state, player);
        }
        ScreenView::ArtistsList => {
            let mut artist_names: Vec<String> = library.artists.keys().cloned().collect();
            artist_names.sort();
            let sel = state.get_selected_index("artists_list").min(artist_names.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Artists",
                &artist_names,
                None,
                sel,
                true,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::ArtistAlbums { ref artist_name } => {
            let albums = library
                .artists
                .get(artist_name)
                .map(|a| a.album_names.clone())
                .unwrap_or_default();
            let sel = state.get_selected_index("artist_albums").min(albums.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                artist_name,
                &albums,
                None,
                sel,
                true,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::AlbumsList => {
            let mut album_names: Vec<String> = library.albums.values().map(|a| a.name.clone()).collect();
            album_names.sort();
            album_names.dedup();
            let sel = state.get_selected_index("albums_list").min(album_names.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Albums",
                &album_names,
                None,
                sel,
                true,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::AlbumDetail { ref album_key } => {
            let (album_name, song_titles, durations) = if let Some(album) = library.albums.get(album_key) {
                let titles: Vec<String> = album
                    .song_ids
                    .iter()
                    .filter_map(|&id| library.songs.get(id).map(|s| s.title.clone()))
                    .collect();
                let durs: Vec<String> = album
                    .song_ids
                    .iter()
                    .filter_map(|&id| {
                        library.songs.get(id).map(|s| {
                            let sec = s.duration_sec as u32;
                            format!("{}:{:02}", sec / 60, sec % 60)
                        })
                    })
                    .collect();
                (album.name.clone(), titles, durs)
            } else {
                ("Album".to_string(), Vec::new(), Vec::new())
            };
            let sel = state.get_selected_index("album_detail").min(song_titles.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                &album_name,
                &song_titles,
                Some(&durations),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::SongsList => {
            let time = ctx.input(|i| i.time);
            let titles: Vec<String> = library.sorted_song_indices.iter().filter_map(|&id| library.songs.get(id).map(|s| s.title.clone())).collect();
            let artists: Vec<String> = library.sorted_song_indices.iter().filter_map(|&id| {
                library.songs.get(id).map(|s| LcdRenderer::get_display_artist(&s.artist, time))
            }).collect();
            let sel = state.get_selected_index("songs_list").min(titles.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Songs",
                &titles,
                Some(&artists),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::GenresList => {
            let mut genres: Vec<String> = library.genres.keys().cloned().collect();
            genres.sort();
            let sel = state.get_selected_index("genres_list").min(genres.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Genres",
                &genres,
                None,
                sel,
                true,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::GenreDetail { ref genre_name } => {
            let time = ctx.input(|i| i.time);
            let song_ids = library.genres.get(genre_name).cloned().unwrap_or_default();
            let titles: Vec<String> = song_ids
                .iter()
                .filter_map(|&id| library.songs.get(id).map(|s| s.title.clone()))
                .collect();
            let artists: Vec<String> = song_ids
                .iter()
                .filter_map(|&id| library.songs.get(id).map(|s| LcdRenderer::get_display_artist(&s.artist, time)))
                .collect();
            let sel = state.get_selected_index("genre_detail").min(titles.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                genre_name,
                &titles,
                Some(&artists),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::ComposersList => {
            let mut composers: Vec<String> = library.composers.keys().cloned().collect();
            composers.sort();
            let sel = state.get_selected_index("composers_list").min(composers.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Composers",
                &composers,
                None,
                sel,
                true,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::ComposerDetail { ref composer_name } => {
            let time = ctx.input(|i| i.time);
            let song_ids = library.composers.get(composer_name).cloned().unwrap_or_default();
            let titles: Vec<String> = song_ids
                .iter()
                .filter_map(|&id| library.songs.get(id).map(|s| s.title.clone()))
                .collect();
            let artists: Vec<String> = song_ids
                .iter()
                .filter_map(|&id| library.songs.get(id).map(|s| LcdRenderer::get_display_artist(&s.artist, time)))
                .collect();
            let sel = state.get_selected_index("composer_detail").min(titles.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                composer_name,
                &titles,
                Some(&artists),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::SearchScreen { .. } => {
            views::search_view::SearchView::render(painter, screen_rect, state, library, player);
        }
        ScreenView::NowPlaying => {
            NowPlayingView::render(painter, ctx, screen_rect, state, library, player, art_cache);
        }
        ScreenView::VideosMenu => {
            VideoView::render_videos_menu(painter, screen_rect, state, library, player);
        }
        ScreenView::VideosCategoryList { category } => {
            let vids: Vec<String> = library
                .videos
                .iter()
                .filter(|v| v.category == category)
                .map(|v| v.title.clone())
                .collect();
            let durs: Vec<String> = library
                .videos
                .iter()
                .filter(|v| v.category == category)
                .map(|v| {
                    let s = v.duration_sec as u32;
                    format!("{}:{:02}", s / 60, s % 60)
                })
                .collect();
            let sel = state.get_selected_index("videos_category").min(vids.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                category.name(),
                &vids,
                Some(&durs),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::VideoPlayerScreen { video_id } => {
            if let Some(video) = library.videos.get(video_id) {
                VideoView::render_video_player(painter, screen_rect, state, video_player, video);
            }
        }
        ScreenView::SettingsMenu => {
            SettingsView::render_settings_menu(painter, screen_rect, state, library, player);
        }
        ScreenView::SoftwareUpdate => {
            SettingsView::render_software_update_screen(painter, screen_rect, state, player);
        }
        ScreenView::AboutScreen => {
            SettingsView::render_about_screen(painter, screen_rect, state, library, player);
        }
        ScreenView::MusicSources => {
            SettingsView::render_music_sources_screen(painter, screen_rect, state, library, player);
        }
        ScreenView::CustomEqEditor => {
            SettingsView::render_custom_eq_screen(painter, screen_rect, state, player);
        }
        ScreenView::DiscordSettings => {
            SettingsView::render_discord_settings_screen(painter, screen_rect, state, player);
        }
        ScreenView::MainMenuCustomizer => {
            let items = vec![
                format!("Music: {}", if state.main_menu_config.show_music { "On" } else { "Off" }),
                format!("Videos: {}", if state.main_menu_config.show_videos { "On" } else { "Off" }),
                format!("Shuffle Songs: {}", if state.main_menu_config.show_shuffle_songs { "On" } else { "Off" }),
                format!("Now Playing: {}", if state.main_menu_config.show_now_playing { "On" } else { "Off" }),
                format!("Settings: {}", if state.main_menu_config.show_settings { "On" } else { "Off" }),
            ];
            let sel = state.get_selected_index("main_menu_customizer").min(items.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Main Menu",
                &items,
                None,
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::EqSelector => {
            let eq_names: Vec<String> = EqPreset::ALL.iter().map(|e| e.name().to_string()).collect();
            let details: Vec<String> = EqPreset::ALL
                .iter()
                .map(|e| if *e == player.eq { "✔".to_string() } else { "".to_string() })
                .collect();
            let sel = state.get_selected_index("eq_selector").min(eq_names.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Equalizer",
                &eq_names,
                Some(&details),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::BacklightSelector => {
            let durations = [
                BacklightDuration::Off,
                BacklightDuration::TwoSec,
                BacklightDuration::FiveSec,
                BacklightDuration::TenSec,
                BacklightDuration::TwentySec,
                BacklightDuration::AlwaysOn,
            ];
            let names: Vec<String> = durations.iter().map(|d| d.name().to_string()).collect();
            let details: Vec<String> = durations
                .iter()
                .map(|d| if *d == state.backlight_timer { "✔".to_string() } else { "".to_string() })
                .collect();
            let sel = state.get_selected_index("backlight_selector").min(names.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Backlight Timer",
                &names,
                Some(&details),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::BrightnessAdjuster => {
            SettingsView::render_brightness_screen(painter, screen_rect, state, player);
        }
        ScreenView::ThemeSettings => {
            SettingsView::render_theme_settings_menu(painter, screen_rect, state, player);
        }
        ScreenView::ColorThemeSelector => {
            // Custom studio first, then saved presets (live in the list), then built-in colors
            let mut names: Vec<String> = vec!["Custom".to_string()];
            let mut details: Vec<String> = vec![if state.chassis_color == ChassisColor::Custom {
                "✔ >".to_string()
            } else {
                ">".to_string()
            }];

            for preset in &state.custom_presets {
                names.push(preset.name.clone());
                let is_active = state.active_preset_id.as_deref() == Some(&preset.id);
                details.push(if is_active { "✔".to_string() } else { "".to_string() });
            }

            for color in ChassisColor::ALL.iter().filter(|c| **c != ChassisColor::Custom) {
                names.push(color.name().to_string());
                details.push(if *color == state.chassis_color { "✔".to_string() } else { "".to_string() });
            }

            let sel = state.get_selected_index("color_theme_selector").min(names.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Color",
                &names,
                Some(&details),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::CustomColorCategoryList => {
            SettingsView::render_custom_color_category_list(painter, screen_rect, state, player);
        }
        ScreenView::CustomColorTargetList { category } => {
            SettingsView::render_custom_color_target_list(painter, screen_rect, category, state, player);
        }
        ScreenView::ColorPicker { target } => {
            SettingsView::render_color_picker_screen(painter, screen_rect, target, state, player);
        }
        ScreenView::ColorPresetsManager => {
            SettingsView::render_color_presets_manager(painter, screen_rect, state, player);
        }
        ScreenView::PresetOptions { ref preset_id } => {
            SettingsView::render_preset_options_screen(painter, screen_rect, preset_id, state, player);
        }
        ScreenView::PresetNameInput { ref editing_preset_id } => {
            SettingsView::render_preset_name_input_screen(painter, screen_rect, editing_preset_id.is_some(), state, player);
        }
        ScreenView::DisplayThemeSelector => {
            let names: Vec<String> = DisplayTheme::ALL.iter().map(|d| d.name().to_string()).collect();
            let details: Vec<String> = DisplayTheme::ALL
                .iter()
                .map(|d| {
                    if *d == state.display_theme {
                        if *d == DisplayTheme::AlbumCover {
                            "✔ >".to_string()
                        } else {
                            "✔".to_string()
                        }
                    } else if *d == DisplayTheme::AlbumCover {
                        ">".to_string()
                    } else {
                        "".to_string()
                    }
                })
                .collect();
            let sel = state.get_selected_index("display_theme_selector").min(names.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Display",
                &names,
                Some(&details),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::ShellStyleSelector => {
            let names: Vec<String> = ShellStyle::ALL.iter().map(|s| s.name().to_string()).collect();
            let details: Vec<String> = ShellStyle::ALL
                .iter()
                .map(|s| if *s == state.shell_style { "✔".to_string() } else { "".to_string() })
                .collect();
            let sel = state.get_selected_index("shell_style_selector").min(names.len().saturating_sub(1));
            MenuView::render_full_list(
                painter,
                screen_rect,
                "Shell",
                &names,
                Some(&details),
                sel,
                false,
                player,
                state.is_hold_locked,
                state.display_theme,
            );
        }
        ScreenView::AlbumCoverConfig => {
            SettingsView::render_album_cover_config_screen(painter, screen_rect, state, player);
        }
        ScreenView::AlbumCoverBlurAdjuster => {
            SettingsView::render_album_cover_blur_screen(painter, screen_rect, state, player);
        }
        ScreenView::AlbumCoverBrightnessAdjuster => {
            SettingsView::render_album_cover_brightness_screen(painter, screen_rect, state, player);
        }
    }
}

fn render_music_quiz(
    painter: &Painter,
    screen_rect: Rect,
    state: &AppState,
    library: &Library,
    player: &AudioPlayer,
) {
    let items: Vec<String> = state.music_quiz.choices.iter().filter_map(|&id| library.songs.get(id).map(|s| s.title.clone())).collect();
    let details: Vec<String> = state.music_quiz.choices.iter().enumerate().filter_map(|(idx, &id)| {
        library.songs.get(id).map(|song| {
            if let Some(was_correct) = state.music_quiz.revealed {
                if idx == state.music_quiz.correct_choice {
                    "✔ Correct".to_string()
                } else if idx == state.get_selected_index("music_quiz") && !was_correct {
                    "✕ Wrong".to_string()
                } else {
                    song.artist.clone()
                }
            } else {
                song.artist.clone()
            }
        })
    }).collect();
    let strike_str = if state.music_quiz.wrong_strikes > 0 {
        format!(" • ✕{}/3", state.music_quiz.wrong_strikes)
    } else {
        String::new()
    };
    let title = format!(
        "Quiz {} • {:.0}s • {} pts{}",
        state.music_quiz.question_number.max(1),
        state.music_quiz.remaining_sec.ceil(),
        state.music_quiz.score,
        strike_str
    );
    let sel = state.get_selected_index("music_quiz").min(items.len().saturating_sub(1));
    MenuView::render_full_list(painter, screen_rect, &title, &items, Some(&details), sel, false, player, state.is_hold_locked, state.display_theme);
}

fn restore_after_music_quiz(state: &mut AppState, library: &mut Library, player: &mut AudioPlayer) {
    player.stop();
    state.current_queue = std::mem::take(&mut state.quiz_saved_queue);
    state.current_queue_idx = state.quiz_saved_queue_idx.min(state.current_queue.len().saturating_sub(1));
    if !state.current_queue.is_empty() {
        play_current_queue_track(state, library, player);
        player.seek_to(state.quiz_saved_position_sec);
        if !state.quiz_saved_was_playing {
            player.pause();
        }
    } else {
        state.finish_listening_session(false);
    }
    state.music_quiz.choices.clear();
    state.music_quiz.revealed = None;
    state.music_quiz.wrong_strikes = 0;
    state.music_quiz.restart_pending = false;
}

/// Register a wrong quiz answer (bad pick or timeout): resets the streak and
/// counts a strike; three strikes queue a full quiz restart.
fn register_quiz_wrong(state: &mut AppState) {
    state.music_quiz.streak = 0;
    state.music_quiz.wrong_strikes += 1;
    if state.music_quiz.wrong_strikes >= 3 {
        state.music_quiz.restart_pending = true;
        state.set_status_message("3 strikes! Quiz restarting…", 2.5);
    }
    state.music_quiz.revealed = Some(false);
    state.music_quiz.reveal_remaining_sec = 2.0;
}

fn start_music_quiz_question(state: &mut AppState, library: &Library, player: &mut AudioPlayer) {
    use rand::seq::SliceRandom;
    if library.songs.len() < 2 {
        state.set_status_message("Add at least 2 songs for Music Quiz", 3.0);
        return;
    }

    let mut ids: Vec<usize> = library.sorted_song_indices.iter().copied().filter(|&id| {
        library.songs.get(id).is_some_and(|song| song.duration_sec >= 12.0)
    }).collect();
    ids.shuffle(&mut rand::thread_rng());
    ids.truncate(4.min(ids.len()));
    if ids.len() < 2 {
        state.set_status_message("Not enough playable songs", 3.0);
        return;
    }

    state.music_quiz.choices = ids;
    state.music_quiz.correct_choice = rand::random::<usize>() % state.music_quiz.choices.len();
    state.music_quiz.question_number += 1;
    state.music_quiz.remaining_sec = 10.0;
    state.music_quiz.revealed = None;
    state.music_quiz.reveal_remaining_sec = 0.0;
    state.set_selected_index("music_quiz", 0);

    let song_id = state.music_quiz.choices[state.music_quiz.correct_choice];
    if let Some(song) = library.songs.get(song_id) {
        if let Some(ref path) = song.file_path {
            player.play_file(path.clone(), song.duration_sec);
        } else if song.is_synthetic_demo {
            player.play_synthetic_demo(song_id, song.duration_sec);
        }
        let max_start = (song.duration_sec - 10.0).max(0.0);
        let start = if max_start > 15.0 { 8.0 + rand::random::<f32>() * (max_start - 8.0) } else { 0.0 };
        player.seek_to(start);
    }
}

pub fn handle_wheel_action(
    action: WheelAction,
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    video_player: &mut VideoPlayer,
) {
    match action {
        WheelAction::Tick(delta) => {
            handle_wheel_rotary(delta, state, library, player, video_player);
        }
        WheelAction::Click(btn) => {
            handle_button_click(btn, state, library, player, video_player);
        }
        WheelAction::Hold(btn) => {
            handle_button_hold(btn, state, library, player, video_player);
        }
    }
}

fn handle_wheel_rotary(
    delta: i32,
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    video_player: &mut VideoPlayer,
) {
    let curr_view = state.current_view().clone();

    match curr_view {
        ScreenView::MainMenu => {
            let items = state.get_active_main_menu_items(player);
            state.move_selection("main_menu", delta, items.len());
        }
        ScreenView::MusicMenu => {
            state.move_selection("music_menu", delta, 7);
        }
        ScreenView::ExtrasMenu => state.move_selection("extras_menu", delta, 4),
        ScreenView::QueueList => state.move_selection("queue_list", delta, state.current_queue.len()),
        ScreenView::QueueOptions { .. } => state.move_selection("queue_options", delta, 5),
        ScreenView::SongQueueOptions { .. } => state.move_selection("song_queue_options", delta, 3),
        ScreenView::StatsHub => state.move_selection("stats_hub", delta, 4),
        ScreenView::StatsSongs => state.move_selection("stats_songs", delta, state.listening_stats.top_tracks().len()),
        ScreenView::StatsArtists => state.move_selection("stats_artists", delta, state.listening_stats.aggregate_artists().len()),
        ScreenView::StatsAlbums => state.move_selection("stats_albums", delta, state.listening_stats.aggregate_albums().len()),
        ScreenView::MusicQuiz => state.move_selection("music_quiz", delta, state.music_quiz.choices.len()),
        ScreenView::SleepTimer => state.move_selection("sleep_timer", delta, SleepTimerMode::ALL.len()),
        ScreenView::PlaylistsList => {
            state.move_selection("playlists_list", delta, 1 + library.playlists.len());
        }
        ScreenView::PlaylistDetail { playlist_idx } => {
            let count = library.playlists.get(playlist_idx).map(|p| p.song_ids.len()).unwrap_or(0);
            let total = if state.playlist_view_mode == PlaylistViewMode::Modern {
                count
            } else {
                3 + count
            };
            state.move_selection("playlist_detail", delta, total);
        }
        ScreenView::PlaylistAddSongs { playlist_idx: _, ref search_query } => {
            let filtered = views::playlist_view::PlaylistView::get_filtered_songs_for_add(library, search_query);
            state.move_selection("playlist_add_songs", delta, filtered.len());
        }
        ScreenView::PlaylistOptions { .. } => {
            state.move_selection("playlist_options", delta, 5);
        }
        ScreenView::PlaylistNameInput { .. } => {}
        ScreenView::PlaylistViewSelector => {
            state.move_selection("playlist_view_selector", delta, PlaylistViewMode::ALL.len());
        }
        ScreenView::ArtistsList => {
            let mut artists: Vec<String> = library.artists.keys().cloned().collect();
            artists.sort();
            state.move_selection("artists_list", delta, artists.len());
            if let Some(artist) = artists.get(state.get_selected_index("artists_list")) {
                if let Some(first_char) = artist.chars().next() {
                    state.show_alphabet_hud(first_char.to_ascii_uppercase());
                }
            }
        }
        ScreenView::ArtistAlbums { ref artist_name } => {
            let count = library.artists.get(artist_name).map(|a| a.album_names.len()).unwrap_or(0);
            state.move_selection("artist_albums", delta, count);
        }
        ScreenView::AlbumsList => {
            state.move_selection("albums_list", delta, library.albums.len());
        }
        ScreenView::AlbumDetail { ref album_key } => {
            let count = library.albums.get(album_key).map(|a| a.song_ids.len()).unwrap_or(0);
            state.move_selection("album_detail", delta, count);
        }
        ScreenView::SongsList => {
            state.move_selection("songs_list", delta, library.sorted_song_indices.len());
            let sel = state.get_selected_index("songs_list");
            if let Some(&song_id) = library.sorted_song_indices.get(sel) {
                if let Some(song) = library.songs.get(song_id) {
                    if let Some(first_char) = song.title.chars().next() {
                        state.show_alphabet_hud(first_char.to_ascii_uppercase());
                    }
                }
            }
        }
        ScreenView::GenresList => {
            state.move_selection("genres_list", delta, library.genres.len());
        }
        ScreenView::GenreDetail { ref genre_name } => {
            let count = library.genres.get(genre_name).map(|g| g.len()).unwrap_or(0);
            state.move_selection("genre_detail", delta, count);
        }
        ScreenView::ComposersList => {
            state.move_selection("composers_list", delta, library.composers.len());
        }
        ScreenView::ComposerDetail { ref composer_name } => {
            let count = library.composers.get(composer_name).map(|c| c.len()).unwrap_or(0);
            state.move_selection("composer_detail", delta, count);
        }
        ScreenView::SearchScreen { .. } => {
            let filtered = views::search_view::SearchView::get_filtered_songs(library, &state.search_query);
            state.move_selection("search_screen", delta, filtered.len());
        }
        ScreenView::NowPlaying => {
            match state.now_playing_substate {
                NowPlayingSubState::Standard | NowPlayingSubState::FullArtwork | NowPlayingSubState::Visualizer => {
                    let vol_step = 1.0_f32 / 16.0_f32;
                    let new_vol = player.volume + (delta as f32) * vol_step;
                    player.set_volume(new_vol);
                    state.show_volume_hud();
                    state.save_settings(player);
                }
                NowPlayingSubState::Lyrics => {
                    state.lyrics_scroll_offset -= (delta as f32) * 24.0_f32;
                }
            }
        }
        ScreenView::VideosMenu => {
            state.move_selection("videos_menu", delta, 5);
        }
        ScreenView::VideosCategoryList { category } => {
            let count = library.videos.iter().filter(|v| v.category == category).count();
            state.move_selection("videos_category", delta, count);
        }
        ScreenView::VideoPlayerScreen { .. } => {
            video_player.seek((delta as f32) * 5.0_f32);
        }
        ScreenView::SettingsMenu => {
            state.move_selection("settings_menu", delta, 17);
        }
        ScreenView::SoftwareUpdate => {}
        ScreenView::DiscordSettings => {
            let max_idx = if state.discord_enabled { 2 } else { 1 };
            state.move_selection("discord_settings", delta, max_idx);
        }
        ScreenView::AboutScreen => {
            state.move_selection("about_screen", delta, 10);
        }
        ScreenView::MusicSources => {
            let count = 2 + library.music_sources.len();
            state.move_selection("music_sources", delta, count);
        }
        ScreenView::CustomEqEditor => {
            let band = state.selected_eq_band;
            if band < 10 {
                let curr = player.custom_eq_bands[band];
                player.custom_eq_bands[band] = (curr + (delta as f32) * 1.0_f32).clamp(-12.0_f32, 12.0_f32);
                player.eq = EqPreset::Custom;
                player.set_volume(player.volume);
                state.save_settings(player);
            }
        }
        ScreenView::MainMenuCustomizer => {
            state.move_selection("main_menu_customizer", delta, 5);
        }
        ScreenView::EqSelector => {
            state.move_selection("eq_selector", delta, EqPreset::ALL.len());
        }
        ScreenView::BacklightSelector => {
            state.move_selection("backlight_selector", delta, 6);
        }
        ScreenView::BrightnessAdjuster => {
            let new_bright = (state.brightness_percent as i32 + delta * 5).clamp(10, 100) as u8;
            state.brightness_percent = new_bright;
            state.save_settings(player);
        }
        ScreenView::ThemeSettings => {
            state.move_selection("theme_settings", delta, 3);
        }
        ScreenView::ColorThemeSelector => {
            state.move_selection("color_theme_selector", delta, 1 + state.custom_presets.len() + ChassisColor::ALL.len() - 1);
        }
        ScreenView::CustomColorCategoryList => {
            state.move_selection("custom_color_category_list", delta, 7);
        }
        ScreenView::CustomColorTargetList { category } => {
            state.move_selection("custom_color_target_list", delta, category.targets().len());
        }
        ScreenView::ColorPresetsManager => {
            state.move_selection("color_presets_manager", delta, 1 + state.custom_presets.len());
        }
        ScreenView::PresetOptions { .. } => {
            state.move_selection("preset_options", delta, 4);
        }
        ScreenView::PresetNameInput { .. } => {}
        ScreenView::ColorPicker { target } => {
            let [r, g, b] = state.custom_theme.get_color(target);
            let (h, s, v) = rgb_to_hsv(r, g, b);
            let new_rgb = match state.color_picker_mode {
                0 => { // Saturation
                    let new_s = (s + delta as f32 * 0.03).clamp(0.0, 1.0);
                    hsv_to_rgb(h, new_s, v.max(0.1))
                }
                1 => { // Hue
                    let new_h = (h + delta as f32 * 6.0).rem_euclid(360.0);
                    hsv_to_rgb(new_h, s.max(0.1), v.max(0.1))
                }
                _ => { // Value / Brightness
                    let new_v = (v + delta as f32 * 0.03).clamp(0.0, 1.0);
                    hsv_to_rgb(h, s, new_v)
                }
            };
            state.custom_theme.set_color(target, new_rgb);
            state.save_settings(player);
        }
        ScreenView::DisplayThemeSelector => {
            state.move_selection("display_theme_selector", delta, DisplayTheme::ALL.len());
        }
        ScreenView::ShellStyleSelector => {
            state.move_selection("shell_style_selector", delta, ShellStyle::ALL.len());
        }
        ScreenView::AlbumCoverConfig => {
            state.move_selection("album_cover_config", delta, 2);
        }
        ScreenView::AlbumCoverBlurAdjuster => {
            let new_blur = (state.album_art_blur as i32 + delta).clamp(0, 30) as u8;
            state.album_art_blur = new_blur;
            state.save_settings(player);
        }
        ScreenView::AlbumCoverBrightnessAdjuster => {
            let new_bright = (state.album_art_brightness as i32 + delta * 5).clamp(10, 100) as u8;
            state.album_art_brightness = new_bright;
            state.save_settings(player);
        }
    }
}

fn cycle_repeat_mode(player: &mut AudioPlayer) {
    player.repeat = match player.repeat {
        RepeatMode::Off => RepeatMode::All,
        RepeatMode::All => RepeatMode::One,
        RepeatMode::One => RepeatMode::Off,
    };
}

fn handle_button_click(
    btn: WheelButton,
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    video_player: &mut VideoPlayer,
) {
    let quiz_active = matches!(state.current_view(), ScreenView::MusicQuiz);
    match btn {
        WheelButton::Menu => {
            if quiz_active {
                restore_after_music_quiz(state, library, player);
            }
            if let ScreenView::VideoPlayerScreen { video_id } = state.current_view().clone() {
                if let Some(v) = library.videos.get_mut(video_id) {
                    v.resume_pos_sec = video_player.current_time_sec;
                }
                video_player.is_playing = false;
            }
            state.pop_view();
        }
        WheelButton::PlayPause => {
            // Transport is locked during the Music Quiz so the excerpt keeps playing
            if quiz_active {
                return;
            }
            if let ScreenView::VideoPlayerScreen { .. } = state.current_view() {
                video_player.toggle_play_pause();
            } else {
                player.toggle_play_pause();
            }
        }
        WheelButton::Previous => {
            if quiz_active {
                return;
            }
            if let ScreenView::VideoPlayerScreen { .. } = state.current_view() {
                video_player.seek(-15.0_f32);
                return;
            }
            // iPod behavior: 1st press restarts the current song; a quick
            // 2nd press goes back to the previous song.
            let now = std::time::Instant::now();
            let second_press = state
                .last_previous_press
                .is_some_and(|t| now.duration_since(t).as_millis() < 1400);
            if second_press {
                state.last_previous_press = None;
                if !state.current_queue.is_empty() {
                    state.current_queue_idx = if state.current_queue_idx == 0 {
                        state.current_queue.len() - 1
                    } else {
                        state.current_queue_idx - 1
                    };
                    play_current_queue_track(state, library, player);
                }
            } else {
                state.last_previous_press = Some(now);
                state.restart_listening_session();
                player.seek_to(0.0_f32);
            }
        }
        WheelButton::Next => {
            if let ScreenView::VideoPlayerScreen { .. } = state.current_view() {
                video_player.seek(15.0_f32);
                return;
            }
            if quiz_active {
                return;
            }
            // Hover-queueing: hovering a song row and pressing NEXT queues it
            // up next instead of skipping the current track.
            if let Some(song_id) = hovered_song_row(state, library) {
                let title = library.songs.get(song_id).map(|s| s.title.clone()).unwrap_or_default();
                if state.current_queue.is_empty() {
                    state.current_queue = vec![song_id];
                    state.current_queue_idx = 0;
                    play_current_queue_track(state, library, player);
                } else {
                    let insert_at = (state.current_queue_idx + 1).min(state.current_queue.len());
                    state.current_queue.insert(insert_at, song_id);
                }
                state.set_status_message(&format!("Queued next: {}", title), 1.8);
                return;
            }
            if !state.current_queue.is_empty() {
                state.current_queue_idx = (state.current_queue_idx + 1) % state.current_queue.len();
                play_current_queue_track(state, library, player);
            }
        }
        WheelButton::Select => {
            handle_select_click(state, library, player, video_player);
        }
    }
}

/// Resolve which song row (if any) the mouse is currently hovering over on the
/// virtual LCD across all song-listing screens.
fn hovered_song_row(state: &AppState, library: &Library) -> Option<usize> {
    let (rx, ry, rw, rh) = state.lcd_view_rect;
    let screen_rect = Rect::from_min_max(Pos2::new(rx, ry), Pos2::new(rx + rw, ry + rh));
    let scale = state.lcd_scale;
    let (px, py) = state.lcd_hover_pos?;
    if px < screen_rect.min.x || px > screen_rect.max.x {
        return None;
    }
    let pos = Pos2::new(px, py);
    let bar_h = 20.0_f32 * scale;
    if pos.y < screen_rect.min.y + bar_h || pos.y > screen_rect.max.y {
        return None;
    }

    fn scroll_offset_for(sel: usize, top_y: f32, bottom_y: f32, row_h: f32) -> usize {
        let visible = (((bottom_y - top_y) / row_h).floor() as usize).max(1);
        if sel >= visible { sel - visible + 1 } else { 0 }
    }

    match state.current_view() {
        ScreenView::SongsList => {
            let sel = state.get_selected_index("songs_list");
            let offset = scroll_offset_for(sel, screen_rect.min.y + bar_h, screen_rect.max.y, 22.0 * scale);
            let row = ((pos.y - screen_rect.min.y - bar_h) / (22.0 * scale)).floor() as usize;
            library.sorted_song_indices.get(offset + row).copied()
        }
        ScreenView::AlbumDetail { album_key } => {
            let ids = library.albums.get(album_key)?.song_ids.clone();
            let sel = state.get_selected_index("album_detail");
            let offset = scroll_offset_for(sel, screen_rect.min.y + bar_h, screen_rect.max.y, 22.0 * scale);
            let row = ((pos.y - screen_rect.min.y - bar_h) / (22.0 * scale)).floor() as usize;
            ids.get(offset + row).copied()
        }
        ScreenView::GenreDetail { genre_name } => {
            let ids = library.genres.get(genre_name)?.clone();
            let sel = state.get_selected_index("genre_detail");
            let offset = scroll_offset_for(sel, screen_rect.min.y + bar_h, screen_rect.max.y, 22.0 * scale);
            let row = ((pos.y - screen_rect.min.y - bar_h) / (22.0 * scale)).floor() as usize;
            ids.get(offset + row).copied()
        }
        ScreenView::ComposerDetail { composer_name } => {
            let ids = library.composers.get(composer_name)?.clone();
            let sel = state.get_selected_index("composer_detail");
            let offset = scroll_offset_for(sel, screen_rect.min.y + bar_h, screen_rect.max.y, 22.0 * scale);
            let row = ((pos.y - screen_rect.min.y - bar_h) / (22.0 * scale)).floor() as usize;
            ids.get(offset + row).copied()
        }
        ScreenView::SearchScreen { .. } => {
            let search_box_bottom = screen_rect.min.y + bar_h + (4.0 + 22.0) * scale;
            let ribbon_bottom = search_box_bottom + 3.0 * scale + 18.0 * scale;
            let list_top = ribbon_bottom + 2.0 * scale;
            if pos.y < list_top {
                return None;
            }
            let filtered = views::search_view::SearchView::get_filtered_songs(library, &state.search_query);
            let sel = state.get_selected_index("search_screen");
            let offset = scroll_offset_for(sel, list_top, screen_rect.max.y, 22.0 * scale);
            let row = ((pos.y - list_top) / (22.0 * scale)).floor() as usize;
            filtered.get(offset + row).copied()
        }
        ScreenView::PlaylistDetail { playlist_idx } if state.playlist_view_mode == PlaylistViewMode::Modern => {
            let playlist = library.playlists.get(*playlist_idx)?;
            let header_top = screen_rect.min.y + bar_h + 4.0 * scale;
            let table_top = header_top + 68.0 * scale + 4.0 * scale;
            let list_top = table_top + 14.0 * scale + 2.0 * scale;
            if pos.y < list_top {
                return None;
            }
            let total_songs = playlist.song_ids.len();
            let sel = state.get_selected_index("playlist_detail").min(total_songs.saturating_sub(1));
            let visible = (((screen_rect.max.y - list_top) / (24.0 * scale)).floor() as usize).max(1);
            let offset = if sel >= visible { sel - visible + 1 } else { 0 };
            let row = ((pos.y - list_top) / (24.0 * scale)).floor() as usize;
            playlist.song_ids.get(offset + row).copied()
        }
        _ => None,
    }
}

fn handle_select_click(
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    video_player: &mut VideoPlayer,
) {
    let curr_view = state.current_view().clone();

    match curr_view {
        ScreenView::MainMenu => {
            let items = state.get_active_main_menu_items(player);
            let sel = state.get_selected_index("main_menu").min(items.len().saturating_sub(1));
            if let Some(target) = items.get(sel) {
                match target {
                    MainMenuItem::Music => state.push_view(ScreenView::MusicMenu),
                    MainMenuItem::Videos => state.push_view(ScreenView::VideosMenu),
                    MainMenuItem::ShuffleSongs => {
                        player.shuffle = ShuffleMode::Songs;
                        let mut all_ids: Vec<usize> = library.sorted_song_indices.clone();
                        use rand::seq::SliceRandom;
                        let mut rng = rand::thread_rng();
                        all_ids.shuffle(&mut rng);
                        state.current_queue = all_ids;
                        state.current_queue_idx = 0;
                        play_current_queue_track(state, library, player);
                        state.push_view(ScreenView::NowPlaying);
                    }
                    MainMenuItem::NowPlaying => state.push_view(ScreenView::NowPlaying),
                    MainMenuItem::Settings => state.push_view(ScreenView::SettingsMenu),
                    MainMenuItem::Extras => {
                        state.set_selected_index("extras_menu", 0);
                        state.push_view(ScreenView::ExtrasMenu);
                    }
                }
            }
        }
        ScreenView::MusicMenu => {
            let sel = state.get_selected_index("music_menu");
            match sel {
                0 => state.push_view(ScreenView::PlaylistsList),
                1 => state.push_view(ScreenView::ArtistsList),
                2 => state.push_view(ScreenView::AlbumsList),
                3 => state.push_view(ScreenView::SongsList),
                4 => state.push_view(ScreenView::GenresList),
                5 => state.push_view(ScreenView::ComposersList),
                6 => {
                    state.search_query.clear();
                    state.set_selected_index("search_screen", 0);
                    state.push_view(ScreenView::SearchScreen { query: String::new() });
                }
                _ => {}
            }
        }
        ScreenView::ExtrasMenu => {
            match state.get_selected_index("extras_menu") {
                0 => {
                    state.set_selected_index("queue_list", state.current_queue_idx);
                    state.push_view(ScreenView::QueueList);
                }
                1 => {
                    state.set_selected_index("stats_hub", 0);
                    state.push_view(ScreenView::StatsHub);
                }
                2 => {
                    state.quiz_saved_queue = state.current_queue.clone();
                    state.quiz_saved_queue_idx = state.current_queue_idx;
                    state.quiz_saved_position_sec = player.current_time_sec;
                    state.quiz_saved_was_playing = player.is_playing;
                    state.music_quiz.score = 0;
                    state.music_quiz.streak = 0;
                    state.music_quiz.question_number = 0;
                    state.music_quiz.wrong_strikes = 0;
                    state.music_quiz.restart_pending = false;
                    start_music_quiz_question(state, library, player);
                    state.push_view(ScreenView::MusicQuiz);
                }
                3 => {
                    state.set_selected_index("sleep_timer", 0);
                    state.push_view(ScreenView::SleepTimer);
                }
                _ => {}
            }
        }
        ScreenView::QueueList => {
            let pos = state.get_selected_index("queue_list");
            if pos < state.current_queue.len() {
                state.set_selected_index("queue_options", 0);
                state.push_view(ScreenView::QueueOptions { queue_pos: pos });
            }
        }
        ScreenView::QueueOptions { queue_pos } => {
            let sel = state.get_selected_index("queue_options");
            if queue_pos < state.current_queue.len() {
                match sel {
                    0 => {
                        let target_song_id = state.current_queue.get(queue_pos).copied();
                        let is_current = state.current_queue.get(state.current_queue_idx).copied() == target_song_id;
                        if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                            state.view_stack.retain(|v| !matches!(v, ScreenView::QueueOptions { .. }));
                            state.push_view(ScreenView::NowPlaying);
                            return;
                        }
                        state.current_queue_idx = queue_pos;
                        play_current_queue_track(state, library, player);
                        state.view_stack.retain(|v| !matches!(v, ScreenView::QueueOptions { .. }));
                        state.push_view(ScreenView::NowPlaying);
                    }
                    1 if queue_pos > 0 => {
                        state.current_queue.swap(queue_pos, queue_pos - 1);
                        if state.current_queue_idx == queue_pos { state.current_queue_idx -= 1; }
                        else if state.current_queue_idx == queue_pos - 1 { state.current_queue_idx += 1; }
                        state.pop_view();
                        state.set_selected_index("queue_list", queue_pos - 1);
                    }
                    2 if queue_pos + 1 < state.current_queue.len() => {
                        state.current_queue.swap(queue_pos, queue_pos + 1);
                        if state.current_queue_idx == queue_pos { state.current_queue_idx += 1; }
                        else if state.current_queue_idx == queue_pos + 1 { state.current_queue_idx -= 1; }
                        state.pop_view();
                        state.set_selected_index("queue_list", queue_pos + 1);
                    }
                    3 => {
                        let removing_current = queue_pos == state.current_queue_idx;
                        state.current_queue.remove(queue_pos);
                        if state.current_queue.is_empty() {
                            state.current_queue_idx = 0;
                            state.finish_listening_session(false);
                            player.stop();
                        } else {
                            if queue_pos < state.current_queue_idx { state.current_queue_idx -= 1; }
                            state.current_queue_idx = state.current_queue_idx.min(state.current_queue.len() - 1);
                            if removing_current { play_current_queue_track(state, library, player); }
                        }
                        state.pop_view();
                    }
                    4 => {
                        state.current_queue.clear();
                        state.current_queue_idx = 0;
                        state.finish_listening_session(false);
                        player.stop();
                        state.pop_view();
                        state.set_status_message("Queue cleared", 2.0);
                    }
                    _ => {}
                }
            }
        }
        ScreenView::SongQueueOptions { song_id } => {
            match state.get_selected_index("song_queue_options") {
                0 => {
                    let insert_at = (state.current_queue_idx + 1).min(state.current_queue.len());
                    state.current_queue.insert(insert_at, song_id);
                    state.set_status_message("Added to Play Next", 2.0);
                    state.pop_view();
                }
                1 => {
                    state.current_queue.push(song_id);
                    state.set_status_message("Added to end of queue", 2.0);
                    state.pop_view();
                }
                2 => {
                    let insert_at = if state.current_queue.is_empty() { 0 } else { (state.current_queue_idx + 1).min(state.current_queue.len()) };
                    state.current_queue.insert(insert_at, song_id);
                    state.current_queue_idx = insert_at;
                    play_current_queue_track(state, library, player);
                    state.push_view(ScreenView::NowPlaying);
                }
                _ => {}
            }
        }
        ScreenView::StatsHub => {
            match state.get_selected_index("stats_hub") {
                0 => state.push_view(ScreenView::StatsSongs),
                1 => state.push_view(ScreenView::StatsArtists),
                2 => state.push_view(ScreenView::StatsAlbums),
                _ => {}
            }
        }
        ScreenView::StatsSongs | ScreenView::StatsArtists | ScreenView::StatsAlbums => {}
        ScreenView::MusicQuiz => {
            if state.music_quiz.revealed.is_none() && !state.music_quiz.choices.is_empty() {
                let sel = state.get_selected_index("music_quiz").min(state.music_quiz.choices.len() - 1);
                let correct = sel == state.music_quiz.correct_choice;
                if correct {
                    state.music_quiz.streak += 1;
                    let speed_points = state.music_quiz.remaining_sec.ceil().max(1.0) as u32 * 100;
                    state.music_quiz.score += speed_points + state.music_quiz.streak.saturating_sub(1) * 250;
                    state.music_quiz.high_score = state.music_quiz.high_score.max(state.music_quiz.score);
                    state.save_settings(player);
                    state.music_quiz.revealed = Some(true);
                    state.music_quiz.reveal_remaining_sec = 2.2;
                } else {
                    register_quiz_wrong(state);
                }
            }
        }
        ScreenView::SleepTimer => {
            if let Some(&mode) = SleepTimerMode::ALL.get(state.get_selected_index("sleep_timer")) {
                if let Some(original) = state.pre_sleep_volume.take() {
                    player.set_volume(original);
                }
                state.sleep_timer_mode = mode;
                state.sleep_timer_remaining_sec = mode.seconds().unwrap_or(0.0);
                state.set_status_message(if mode == SleepTimerMode::Off { "Sleep timer off" } else { "Sleep timer set" }, 2.0);
                state.pop_view();
            }
        }
        ScreenView::PlaylistsList => {
            let sel = state.get_selected_index("playlists_list");
            if sel == 0 {
                // Item 0 is "Create New Playlist"
                state.playlist_name_buffer = format!("Playlist #{}", library.user_playlists.len() + 1);
                state.push_view(ScreenView::PlaylistNameInput { editing_playlist_idx: None });
            } else {
                let playlist_idx = sel - 1;
                if playlist_idx < library.playlists.len() {
                    state.set_selected_index("playlist_detail", 0);
                    state.push_view(ScreenView::PlaylistDetail { playlist_idx });
                }
            }
        }
        ScreenView::PlaylistDetail { playlist_idx } => {
            let sel = state.get_selected_index("playlist_detail");
            if let Some(playlist) = library.playlists.get(playlist_idx) {
                if state.playlist_view_mode == PlaylistViewMode::Modern {
                    if sel < playlist.song_ids.len() {
                        let target_song_id = playlist.song_ids[sel];
                        let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
                        if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                            state.push_view(ScreenView::NowPlaying);
                            return;
                        }
                        state.current_queue = playlist.song_ids.clone();
                        state.current_queue_idx = sel;
                        play_current_queue_track(state, library, player);
                        state.push_view(ScreenView::NowPlaying);
                    }
                } else {
                    // Default mode options:
                    // 0: [+] Add Songs...
                    // 1: [✎] Rename Playlist...
                    // 2: [🗑] Delete Playlist
                    // 3+: Songs
                    match sel {
                        0 => {
                            state.set_selected_index("playlist_add_songs", 0);
                            state.push_view(ScreenView::PlaylistAddSongs { playlist_idx, search_query: String::new() });
                        }
                        1 => {
                            state.playlist_name_buffer = playlist.name.clone();
                            state.push_view(ScreenView::PlaylistNameInput { editing_playlist_idx: Some(playlist_idx) });
                        }
                        2 => {
                            let name = playlist.name.clone();
                            library.delete_playlist_by_idx(playlist_idx);
                            state.set_status_message(&format!("Deleted '{}'", name), 2.0);
                            state.pop_view();
                        }
                        _ => {
                            let song_offset = sel - 3;
                            if song_offset < playlist.song_ids.len() {
                                let target_song_id = playlist.song_ids[song_offset];
                                let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
                                if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                                    state.push_view(ScreenView::NowPlaying);
                                    return;
                                }
                                state.current_queue = playlist.song_ids.clone();
                                state.current_queue_idx = song_offset;
                                play_current_queue_track(state, library, player);
                                state.push_view(ScreenView::NowPlaying);
                            }
                        }
                    }
                }
            }
        }
        ScreenView::PlaylistOptions { playlist_idx } => {
            let sel = state.get_selected_index("playlist_options");
            if let Some(playlist) = library.playlists.get(playlist_idx) {
                match sel {
                    0 => {
                        // Rename
                        state.playlist_name_buffer = playlist.name.clone();
                        state.push_view(ScreenView::PlaylistNameInput { editing_playlist_idx: Some(playlist_idx) });
                    }
                    1 => {
                        // Change Cover
                        if let Some(file_path) = rfd::FileDialog::new()
                            .set_title("Select Playlist Cover Image")
                            .add_filter("Images", &["jpg", "jpeg", "png", "webp", "bmp"])
                            .pick_file()
                        {
                            library.set_playlist_cover_by_idx(playlist_idx, Some(file_path));
                            state.set_status_message("Cover image updated!", 2.0);
                            state.pop_view();
                        }
                    }
                    2 => {
                        // Clear Cover
                        library.set_playlist_cover_by_idx(playlist_idx, None);
                        state.set_status_message("Cover image cleared!", 2.0);
                        state.pop_view();
                    }
                    3 => {
                        // Add Songs
                        state.set_selected_index("playlist_add_songs", 0);
                        state.push_view(ScreenView::PlaylistAddSongs { playlist_idx, search_query: String::new() });
                    }
                    4 => {
                        // Delete Playlist
                        let name = playlist.name.clone();
                        library.delete_playlist_by_idx(playlist_idx);
                        state.set_status_message(&format!("Deleted '{}'", name), 2.0);
                        state.pop_view(); // Leave options
                        state.pop_view(); // Leave playlist detail back to hub
                    }
                    _ => {}
                }
            }
        }
        ScreenView::PlaylistAddSongs { playlist_idx, ref search_query } => {
            let sel = state.get_selected_index("playlist_add_songs");
            let filtered = views::playlist_view::PlaylistView::get_filtered_songs_for_add(library, search_query);
            if let Some(&song_id) = filtered.get(sel) {
                library.toggle_song_in_playlist(playlist_idx, song_id);
            }
        }
        ScreenView::PlaylistViewSelector => {
            let sel = state.get_selected_index("playlist_view_selector");
            if let Some(&mode) = PlaylistViewMode::ALL.get(sel) {
                state.playlist_view_mode = mode;
                state.save_settings(player);
                state.pop_view();
            }
        }
        ScreenView::ArtistsList => {
            let mut artists: Vec<String> = library.artists.keys().cloned().collect();
            artists.sort();
            let sel = state.get_selected_index("artists_list");
            if let Some(artist_name) = artists.get(sel).cloned() {
                state.push_view(ScreenView::ArtistAlbums { artist_name });
            }
        }
        ScreenView::ArtistAlbums { ref artist_name } => {
            if let Some(artist) = library.artists.get(artist_name) {
                let sel = state.get_selected_index("artist_albums");
                if let Some(album_name) = artist.album_names.get(sel) {
                    let album_key = format!("{} - {}", album_name, artist_name);
                    state.push_view(ScreenView::AlbumDetail { album_key });
                }
            }
        }
        ScreenView::AlbumsList => {
            let mut album_keys: Vec<String> = library.albums.keys().cloned().collect();
            album_keys.sort();
            let sel = state.get_selected_index("albums_list");
            if let Some(album_key) = album_keys.get(sel).cloned() {
                state.push_view(ScreenView::AlbumDetail { album_key });
            }
        }
        ScreenView::AlbumDetail { ref album_key } => {
            if let Some(album) = library.albums.get(album_key) {
                let sel = state.get_selected_index("album_detail");
                if sel < album.song_ids.len() {
                    let target_song_id = album.song_ids[sel];
                    let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
                    if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                        state.push_view(ScreenView::NowPlaying);
                        return;
                    }
                    state.current_queue = album.song_ids.clone();
                    state.current_queue_idx = sel;
                    play_current_queue_track(state, library, player);
                    state.push_view(ScreenView::NowPlaying);
                }
            }
        }
        ScreenView::SongsList => {
            let sel = state.get_selected_index("songs_list");
            if sel < library.sorted_song_indices.len() {
                let target_song_id = library.sorted_song_indices[sel];
                let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
                if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                    state.push_view(ScreenView::NowPlaying);
                    return;
                }
                state.current_queue = library.sorted_song_indices.clone();
                state.current_queue_idx = sel;
                play_current_queue_track(state, library, player);
                state.push_view(ScreenView::NowPlaying);
            }
        }
        ScreenView::GenresList => {
            let mut genres: Vec<String> = library.genres.keys().cloned().collect();
            genres.sort();
            let sel = state.get_selected_index("genres_list");
            if let Some(genre_name) = genres.get(sel).cloned() {
                state.push_view(ScreenView::GenreDetail { genre_name });
            }
        }
        ScreenView::GenreDetail { ref genre_name } => {
            if let Some(song_ids) = library.genres.get(genre_name).cloned() {
                let sel = state.get_selected_index("genre_detail");
                if sel < song_ids.len() {
                    let target_song_id = song_ids[sel];
                    let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
                    if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                        state.push_view(ScreenView::NowPlaying);
                        return;
                    }
                    state.current_queue = song_ids;
                    state.current_queue_idx = sel;
                    play_current_queue_track(state, library, player);
                    state.push_view(ScreenView::NowPlaying);
                }
            }
        }
        ScreenView::ComposersList => {
            let mut composers: Vec<String> = library.composers.keys().cloned().collect();
            composers.sort();
            let sel = state.get_selected_index("composers_list");
            if let Some(composer_name) = composers.get(sel).cloned() {
                state.push_view(ScreenView::ComposerDetail { composer_name });
            }
        }
        ScreenView::ComposerDetail { ref composer_name } => {
            if let Some(song_ids) = library.composers.get(composer_name).cloned() {
                let sel = state.get_selected_index("composer_detail");
                if sel < song_ids.len() {
                    let target_song_id = song_ids[sel];
                    let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
                    if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                        state.push_view(ScreenView::NowPlaying);
                        return;
                    }
                    state.current_queue = song_ids;
                    state.current_queue_idx = sel;
                    play_current_queue_track(state, library, player);
                    state.push_view(ScreenView::NowPlaying);
                }
            }
        }
        ScreenView::SearchScreen { .. } => {
            let filtered = views::search_view::SearchView::get_filtered_songs(library, &state.search_query);
            let sel = state.get_selected_index("search_screen");
            if let Some(&target_song_id) = filtered.get(sel) {
                let is_current = state.current_queue.get(state.current_queue_idx).copied() == Some(target_song_id);
                if is_current && (player.is_playing || player.current_time_sec > 0.0) {
                    state.push_view(ScreenView::NowPlaying);
                    return;
                }
                state.current_queue = filtered;
                state.current_queue_idx = sel;
                play_current_queue_track(state, library, player);
                state.push_view(ScreenView::NowPlaying);
            }
        }
        ScreenView::NowPlaying => {
            state.now_playing_substate = state.now_playing_substate.next();
        }
        ScreenView::VideosMenu => {
            let sel = state.get_selected_index("videos_menu");
            let categories = [
                VideoCategory::Movie,
                VideoCategory::Movie,
                VideoCategory::MusicVideo,
                VideoCategory::TVShow,
                VideoCategory::VideoPodcast,
            ];
            if let Some(&cat) = categories.get(sel) {
                state.push_view(ScreenView::VideosCategoryList { category: cat });
            }
        }
        ScreenView::VideosCategoryList { category } => {
            let matching_vids: Vec<usize> = library
                .videos
                .iter()
                .filter(|v| v.category == category)
                .map(|v| v.id)
                .collect();
            let sel = state.get_selected_index("videos_category");
            if let Some(&vid_id) = matching_vids.get(sel) {
                let dur = library.videos.get(vid_id).map(|v| v.duration_sec).unwrap_or(180.0_f32);
                let resume = library.videos.get(vid_id).map(|v| v.resume_pos_sec).unwrap_or(0.0_f32);
                video_player.load_video(vid_id, dur, resume);
                player.pause();
                state.push_view(ScreenView::VideoPlayerScreen { video_id: vid_id });
            }
        }
        ScreenView::VideoPlayerScreen { .. } => {
            video_player.toggle_aspect_ratio();
        }
        ScreenView::SettingsMenu => {
            let sel = state.get_selected_index("settings_menu");
            match sel {
                0 => state.push_view(ScreenView::SoftwareUpdate),
                1 => state.push_view(ScreenView::AboutScreen),
                2 => state.push_view(ScreenView::MusicSources),
                3 => state.push_view(ScreenView::PlaylistViewSelector),
                4 => state.push_view(ScreenView::EqSelector),
                5 => state.push_view(ScreenView::DiscordSettings),
                6 => state.push_view(ScreenView::MainMenuCustomizer),
                7 => {
                    player.shuffle = match player.shuffle {
                        ShuffleMode::Off => ShuffleMode::Songs,
                        ShuffleMode::Songs => ShuffleMode::Albums,
                        ShuffleMode::Albums => ShuffleMode::Off,
                    };
                    state.save_settings(player);
                }
                8 => {
                    cycle_repeat_mode(player);
                    state.save_settings(player);
                }
                9 => {
                    player.sound_check = !player.sound_check;
                    state.save_settings(player);
                }
                10 => {
                    state.clicker_setting = match state.clicker_setting {
                        crate::audio::clicker::ClickerSetting::Speaker => crate::audio::clicker::ClickerSetting::Headphones,
                        crate::audio::clicker::ClickerSetting::Headphones => crate::audio::clicker::ClickerSetting::Both,
                        crate::audio::clicker::ClickerSetting::Both => crate::audio::clicker::ClickerSetting::Off,
                        crate::audio::clicker::ClickerSetting::Off => crate::audio::clicker::ClickerSetting::Speaker,
                    };
                    state.save_settings(player);
                }
                11 => state.push_view(ScreenView::BacklightSelector),
                12 => state.push_view(ScreenView::BrightnessAdjuster),
                13 => state.push_view(ScreenView::ThemeSettings),
                14 => {
                    player.crossfade_seconds = match player.crossfade_seconds {
                        0 => 2,
                        2 => 4,
                        4 => 6,
                        6 => 8,
                        8 => 10,
                        _ => 0,
                    };
                    state.save_settings(player);
                }
                15 => {
                    state.metadata_repair_requested = true;
                    state.set_status_message("Starting metadata repair…", 2.0);
                }
                16 => {
                    player.shuffle = ShuffleMode::Off;
                    player.repeat = RepeatMode::Off;
                    player.sound_check = false;
                    player.crossfade_seconds = 0;
                    player.eq = EqPreset::Off;
                    player.custom_eq_bands = [0.0; 10];
                    state.chassis_color = ChassisColor::White;
                    state.display_theme = DisplayTheme::Default;
                    state.shell_style = ShellStyle::Default;
                    state.playlist_view_mode = PlaylistViewMode::Default;
                    state.brightness_percent = 85;
                    state.discord_enabled = true;
                    state.discord_display_mode = DiscordDisplayMode::SongAndArtist;
                    state.save_settings(player);
                    state.set_status_message("Settings Reset", 2.0_f32);
                }
                _ => {}
            }
        }
        ScreenView::SoftwareUpdate => {
            match &state.update_status {
                crate::updater::UpdateStatus::UpdateAvailable { download_url, .. } => {
                    let url = download_url.clone();
                    if let Some(ref tx) = state.update_tx {
                        crate::updater::spawn_download_and_apply(url, tx.clone());
                    }
                }
                crate::updater::UpdateStatus::InstalledRestartRequired => {
                    if let Ok(current_exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(current_exe).spawn();
                        std::process::exit(0);
                    }
                }
                crate::updater::UpdateStatus::UpToDate | crate::updater::UpdateStatus::Error(_) | crate::updater::UpdateStatus::Idle => {
                    if let Some(ref tx) = state.update_tx {
                        crate::updater::spawn_update_check(tx.clone());
                    }
                }
                _ => {}
            }
        }
        ScreenView::DiscordSettings => {
            let sel = state.get_selected_index("discord_settings");
            if sel == 0 {
                // Toggle Discord On/Off
                state.discord_enabled = !state.discord_enabled;
                state.save_settings(player);
            } else if sel == 1 && state.discord_enabled {
                // Cycle Display Mode
                state.discord_display_mode = match state.discord_display_mode {
                    DiscordDisplayMode::AppOnly => DiscordDisplayMode::SongTitle,
                    DiscordDisplayMode::SongTitle => DiscordDisplayMode::Artist,
                    DiscordDisplayMode::Artist => DiscordDisplayMode::SongAndArtist,
                    DiscordDisplayMode::SongAndArtist => DiscordDisplayMode::AppOnly,
                };
                state.save_settings(player);
            }
        }
        ScreenView::MusicSources => {
            let sel = state.get_selected_index("music_sources");
            if sel == 0 {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title("Select Music Folder")
                    .pick_folder()
                {
                    library.add_source_folder(folder.clone());
                    let (songs, _) = LibraryScanner::scan_directory(library, &folder);
                    state.save_settings(player);
                    state.set_status_message(&format!("Added! Loaded {} songs", songs), 2.5_f32);
                }
            } else if sel == 1 {
                let (songs, vids) = LibraryScanner::rescan_all_sources(library);
                state.set_status_message(&format!("Rescanned: {} songs, {} vids", songs, vids), 2.5_f32);
            } else {
                let folder_idx = sel - 2;
                if folder_idx < library.music_sources.len() {
                    library.remove_source_folder(folder_idx);
                    state.save_settings(player);
                    state.set_status_message("Folder removed", 2.0_f32);
                }
            }
        }
        ScreenView::CustomEqEditor => {
            state.selected_eq_band = (state.selected_eq_band + 1) % 10;
        }
        ScreenView::MainMenuCustomizer => {
            let sel = state.get_selected_index("main_menu_customizer");
            match sel {
                0 => state.main_menu_config.show_music = !state.main_menu_config.show_music,
                1 => state.main_menu_config.show_videos = !state.main_menu_config.show_videos,
                2 => state.main_menu_config.show_shuffle_songs = !state.main_menu_config.show_shuffle_songs,
                3 => state.main_menu_config.show_now_playing = !state.main_menu_config.show_now_playing,
                4 => state.main_menu_config.show_settings = !state.main_menu_config.show_settings,
                _ => {}
            }
            state.save_settings(player);
        }
        ScreenView::EqSelector => {
            let sel = state.get_selected_index("eq_selector");
            if let Some(&eq) = EqPreset::ALL.get(sel) {
                if eq == EqPreset::Custom {
                    player.eq = EqPreset::Custom;
                    state.save_settings(player);
                    state.push_view(ScreenView::CustomEqEditor);
                } else {
                    player.eq = eq;
                    player.set_volume(player.volume);
                    state.save_settings(player);
                    state.pop_view();
                }
            }
        }
        ScreenView::BacklightSelector => {
            let durations = [
                BacklightDuration::Off,
                BacklightDuration::TwoSec,
                BacklightDuration::FiveSec,
                BacklightDuration::TenSec,
                BacklightDuration::TwentySec,
                BacklightDuration::AlwaysOn,
            ];
            let sel = state.get_selected_index("backlight_selector");
            if let Some(&dur) = durations.get(sel) {
                state.backlight_timer = dur;
                state.save_settings(player);
                state.pop_view();
            }
        }
        ScreenView::ThemeSettings => {
            let sel = state.get_selected_index("theme_settings");
            match sel {
                0 => state.push_view(ScreenView::ColorThemeSelector),
                1 => state.push_view(ScreenView::DisplayThemeSelector),
                2 => state.push_view(ScreenView::ShellStyleSelector),
                _ => {}
            }
        }
        ScreenView::ColorThemeSelector => {
            let sel = state.get_selected_index("color_theme_selector");
            if sel == 0 {
                // Custom studio (make & edit presets inside)
                state.chassis_color = ChassisColor::Custom;
                state.save_settings(player);
                state.push_view(ScreenView::CustomColorCategoryList);
            } else if sel <= state.custom_presets.len() {
                // Apply saved preset directly from the color list
                let preset_idx = sel - 1;
                if let Some(preset) = state.custom_presets.get(preset_idx).cloned() {
                    state.custom_theme = preset.config.clone();
                    state.chassis_color = ChassisColor::Custom;
                    state.active_preset_id = Some(preset.id);
                    state.save_settings(player);
                    state.set_status_message(&format!("Applied preset '{}'", preset.name), 1.8);
                    state.pop_view();
                }
            } else {
                let builtin_idx = sel - 1 - state.custom_presets.len();
                if let Some(&color) = ChassisColor::ALL.iter().filter(|c| **c != ChassisColor::Custom).nth(builtin_idx) {
                    state.chassis_color = color;
                    state.active_preset_id = None;
                    state.save_settings(player);
                    state.pop_view();
                }
            }
        }
        ScreenView::CustomColorCategoryList => {
            let sel = state.get_selected_index("custom_color_category_list");
            match sel {
                0 => {
                    // Save as New Preset
                    state.preset_name_buffer = format!("Preset {}", state.custom_presets.len() + 1);
                    state.push_view(ScreenView::PresetNameInput { editing_preset_id: None });
                }
                1 => {
                    // Saved Presets
                    state.push_view(ScreenView::ColorPresetsManager);
                }
                2 => {
                    // Reset to Defaults
                    state.custom_theme = CustomThemeConfig::default();
                    state.active_preset_id = None;
                    state.save_settings(player);
                    state.set_status_message("Theme reset to defaults", 1.8);
                }
                3..=6 => {
                    let category = CustomThemeCategory::ALL[sel - 3];
                    state.push_view(ScreenView::CustomColorTargetList { category });
                }
                _ => {}
            }
        }
        ScreenView::CustomColorTargetList { category } => {
            let sel = state.get_selected_index("custom_color_target_list");
            let targets = category.targets();
            if let Some(&target) = targets.get(sel) {
                state.push_view(ScreenView::ColorPicker { target });
            }
        }
        ScreenView::ColorPresetsManager => {
            let sel = state.get_selected_index("color_presets_manager");
            if sel == 0 {
                state.preset_name_buffer = format!("Preset {}", state.custom_presets.len() + 1);
                state.push_view(ScreenView::PresetNameInput { editing_preset_id: None });
            } else {
                let preset_idx = sel - 1;
                if let Some(preset) = state.custom_presets.get(preset_idx) {
                    let pid = preset.id.clone();
                    state.push_view(ScreenView::PresetOptions { preset_id: pid });
                }
            }
        }
        ScreenView::PresetOptions { ref preset_id } => {
            let pid = preset_id.clone();
            let sel = state.get_selected_index("preset_options");
            match sel {
                0 => {
                    // Apply Preset
                    if let Some(preset) = state.custom_presets.iter().find(|p| p.id == pid) {
                        state.custom_theme = preset.config.clone();
                        state.chassis_color = ChassisColor::Custom;
                        state.active_preset_id = Some(pid);
                        state.save_settings(player);
                        let name = preset.name.clone();
                        state.set_status_message(&format!("Applied preset '{}'", name), 1.8);
                        state.pop_view();
                    }
                }
                1 => {
                    // Overwrite with Current Colors
                    if let Some(preset) = state.custom_presets.iter_mut().find(|p| p.id == pid) {
                        preset.config = state.custom_theme.clone();
                        let name = preset.name.clone();
                        state.active_preset_id = Some(pid);
                        state.save_settings(player);
                        state.set_status_message(&format!("Updated preset '{}'", name), 1.8);
                        state.pop_view();
                    }
                }
                2 => {
                    // Rename Preset
                    if let Some(preset) = state.custom_presets.iter().find(|p| p.id == pid) {
                        state.preset_name_buffer = preset.name.clone();
                        state.push_view(ScreenView::PresetNameInput { editing_preset_id: Some(pid) });
                    }
                }
                3 => {
                    // Delete Preset
                    if let Some(pos) = state.custom_presets.iter().position(|p| p.id == pid) {
                        let deleted = state.custom_presets.remove(pos);
                        if state.active_preset_id.as_deref() == Some(&pid) {
                            state.active_preset_id = None;
                        }
                        state.save_settings(player);
                        state.set_status_message(&format!("Deleted preset '{}'", deleted.name), 1.8);
                        state.pop_view();
                    }
                }
                _ => {}
            }
        }
        ScreenView::ColorPicker { .. } => {
            state.color_picker_mode = (state.color_picker_mode + 1) % 3;
        }
        ScreenView::DisplayThemeSelector => {
            let sel = state.get_selected_index("display_theme_selector");
            if let Some(&disp) = DisplayTheme::ALL.get(sel) {
                state.display_theme = disp;
                state.save_settings(player);
                if disp == DisplayTheme::AlbumCover {
                    state.push_view(ScreenView::AlbumCoverConfig);
                } else {
                    state.pop_view();
                }
            }
        }
        ScreenView::ShellStyleSelector => {
            let sel = state.get_selected_index("shell_style_selector");
            if let Some(&shell) = ShellStyle::ALL.get(sel) {
                state.shell_style = shell;
                state.save_settings(player);
                state.pop_view();
            }
        }
        ScreenView::AlbumCoverConfig => {
            let sel = state.get_selected_index("album_cover_config");
            match sel {
                0 => state.push_view(ScreenView::AlbumCoverBlurAdjuster),
                1 => state.push_view(ScreenView::AlbumCoverBrightnessAdjuster),
                _ => {}
            }
        }
        ScreenView::AlbumCoverBlurAdjuster | ScreenView::AlbumCoverBrightnessAdjuster => {
            state.pop_view();
        }
        _ => {}
    }
}

fn handle_button_hold(
    btn: WheelButton,
    state: &mut AppState,
    library: &mut Library,
    player: &mut AudioPlayer,
    video_player: &mut VideoPlayer,
) {
    // Transport holds are locked during the Music Quiz
    if matches!(state.current_view(), ScreenView::MusicQuiz)
        && matches!(btn, WheelButton::Previous | WheelButton::Next | WheelButton::PlayPause)
    {
        return;
    }
    match btn {
        WheelButton::Previous => {
            player.seek_to(player.current_time_sec - 1.5_f32);
            video_player.seek(-1.5_f32);
        }
        WheelButton::Next => {
            player.seek_to(player.current_time_sec + 1.5_f32);
            video_player.seek(1.5_f32);
        }
        WheelButton::Select => {
            let song_id = match state.current_view().clone() {
                ScreenView::SongsList => library.sorted_song_indices.get(state.get_selected_index("songs_list")).copied(),
                ScreenView::AlbumDetail { album_key } => library.albums.get(&album_key).and_then(|a| a.song_ids.get(state.get_selected_index("album_detail"))).copied(),
                ScreenView::GenreDetail { genre_name } => library.genres.get(&genre_name).and_then(|ids| ids.get(state.get_selected_index("genre_detail"))).copied(),
                ScreenView::ComposerDetail { composer_name } => library.composers.get(&composer_name).and_then(|ids| ids.get(state.get_selected_index("composer_detail"))).copied(),
                _ => None,
            };
            if let Some(song_id) = song_id {
                state.set_selected_index("song_queue_options", 0);
                state.push_view(ScreenView::SongQueueOptions { song_id });
            }
        }
        _ => {}
    }
}

fn play_current_queue_track(state: &mut AppState, library: &mut Library, player: &mut AudioPlayer) {
    state.finish_listening_session(false);
    if let Some(&song_id) = state.current_queue.get(state.current_queue_idx) {
        if let Some(song) = library.songs.get(song_id) {
            let duration = song.duration_sec;
            let is_synth = song.is_synthetic_demo;
            let file_path = song.file_path.clone();
            state.begin_listening_session(song);

            if let Some(path) = file_path {
                player.play_file(path, duration);
            } else if is_synth {
                player.play_synthetic_demo(song_id, duration);
            }
        }
    }
}

fn handle_track_finished(state: &mut AppState, library: &mut Library, player: &mut AudioPlayer) {
    match player.repeat {
        RepeatMode::One => {
            play_current_queue_track(state, library, player);
        }
        RepeatMode::All => {
            if !state.current_queue.is_empty() {
                state.current_queue_idx = (state.current_queue_idx + 1) % state.current_queue.len();
                play_current_queue_track(state, library, player);
            }
        }
        RepeatMode::Off => {
            if state.current_queue_idx + 1 < state.current_queue.len() {
                state.current_queue_idx += 1;
                play_current_queue_track(state, library, player);
            } else {
                player.stop();
            }
        }
    }
}
