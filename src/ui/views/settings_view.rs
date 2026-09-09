use crate::audio::player::AudioPlayer;
use crate::library::model::Library;
use crate::state::AppState;
use crate::theme::{ColorTarget, CustomThemeCategory, LcdPalette, hsv_to_rgb, rgb_to_hex, rgb_to_hsv};
use crate::ui::lcd::LcdRenderer;
use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub struct SettingsView;

impl SettingsView {
    pub fn render_settings_menu(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        _library: &Library,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let update_available = matches!(state.update_status, crate::updater::UpdateStatus::UpdateAvailable { .. });
        LcdRenderer::draw_status_bar_with_update(painter, screen_rect, "Settings", player, state.is_hold_locked, state.display_theme, update_available);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let update_badge_text = match &state.update_status {
            crate::updater::UpdateStatus::UpdateAvailable { .. } => "Update Available!",
            crate::updater::UpdateStatus::Checking => "Checking...",
            crate::updater::UpdateStatus::Downloading(_) => "Downloading...",
            crate::updater::UpdateStatus::InstalledRestartRequired => "Restart Required",
            crate::updater::UpdateStatus::Error(_) => "Check Failed",
            _ => "Up to date",
        };

        let sources_detail = format!("{} folders", _library.music_sources.len());
        let items: Vec<(&str, Option<String>, bool)> = vec![
            ("Software Update", Some(update_badge_text.to_string()), true),
            ("About", None, true),
            ("Music Sources", Some(sources_detail), true),
            ("Playlist View", Some(state.playlist_view_mode.name().to_string()), true),
            ("Equalizer", Some(player.eq.name().to_string()), true),
            ("Discord", Some(if state.discord_enabled { "On" } else { "Off" }.to_string()), true),
            ("Main Menu", None, true),
            ("Shuffle", Some(player.shuffle.name().to_string()), false),
            ("Repeat", Some(player.repeat.name().to_string()), false),
            ("Sound Check", Some(if player.sound_check { "On" } else { "Off" }.to_string()), false),
            ("Clicker", Some(state.clicker_setting.name().to_string()), false),
            ("Backlight Timer", Some(state.backlight_timer.name().to_string()), true),
            ("Brightness", None, true),
            ("Theme", None, true),
            ("Crossfade", Some(if player.crossfade_seconds == 0 { "Off".to_string() } else { format!("{} Seconds", player.crossfade_seconds) }), false),
            ("Repair Missing Metadata", Some("Art • Artist • Lyrics".to_string()), false),
            ("Reset All Settings", None, false),
        ];

        let selected_idx = state.get_selected_index("settings_menu").min(items.len() - 1);
        let item_h = 22.0_f32 * scale;
        let visible_count = ((content_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if selected_idx >= visible_count {
            selected_idx - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let item_idx = scroll_offset + i;
            if item_idx >= items.len() {
                break;
            }

            let (name, detail, has_arrow) = &items[item_idx];
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                name,
                detail.as_deref(),
                *has_arrow,
                item_idx == selected_idx,
                is_dark,
            );
        }

        if items.len() > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (content_rect.height() * (visible_count as f32 / items.len() as f32)).max(12.0_f32 * scale);
            let thumb_y = content_rect.min.y + (content_rect.height() - thumb_h) * (scroll_offset as f32 / (items.len() - visible_count) as f32);
            painter.rect_filled(Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)), 2.0_f32 * scale, Color32::from_rgb(180, 185, 195));
        }
    }

    pub fn render_theme_settings_menu(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Theme", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let color_detail = if state.chassis_color == crate::theme::ChassisColor::Custom {
            state
                .custom_presets
                .iter()
                .find(|p| state.active_preset_id.as_deref() == Some(p.id.as_str()))
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Custom".to_string())
        } else {
            state.chassis_color.name().to_string()
        };

        let items = [
            ("Color", Some(color_detail), true),
            ("Display", Some(state.display_theme.name().to_string()), true),
            ("Shell", Some(state.shell_style.name().to_string()), true),
        ];

        let selected_idx = state.get_selected_index("theme_settings").min(items.len() - 1);
        let item_h = 24.0_f32 * scale;

        for (i, (name, detail, has_arrow)) in items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );
            let is_sel = i == selected_idx;

            // Selection gradient / row separator
            if is_sel {
                let mid_y = item_rect.min.y + item_rect.height() * 0.48_f32;
                painter.rect_filled(
                    Rect::from_min_max(item_rect.min, Pos2::new(item_rect.max.x, mid_y)),
                    0.0_f32,
                    LcdPalette::SEL_TOP,
                );
                painter.rect_filled(
                    Rect::from_min_max(Pos2::new(item_rect.min.x, mid_y), item_rect.max),
                    0.0_f32,
                    LcdPalette::SEL_BOTTOM,
                );
            } else {
                painter.line_segment(
                    [Pos2::new(item_rect.min.x, item_rect.max.y), item_rect.max],
                    Stroke::new(
                        1.0_f32,
                        if is_dark { Color32::from_white_alpha(30) } else { Color32::from_rgb(235, 238, 242) },
                    ),
                );
            }

            // Live color swatch preview per theme axis
            let swatch_r = 5.5_f32 * scale;
            let swatch_center = Pos2::new(item_rect.min.x + 16.0_f32 * scale, item_rect.center().y);
            let swatch_col = match i {
                0 => crate::theme::ChassisColor::body_color_with_custom(&state.chassis_color, &state.custom_theme),
                1 => state.display_theme.bg_color(),
                _ => if state.shell_style.is_pixel() {
                    Color32::from_rgb(120, 140, 200)
                } else {
                    Color32::from_rgb(210, 215, 222)
                },
            };
            painter.circle_filled(swatch_center, swatch_r, swatch_col);
            painter.circle_stroke(
                swatch_center,
                swatch_r,
                Stroke::new(
                    1.0_f32 * scale,
                    if is_dark { Color32::from_white_alpha(140) } else { Color32::from_black_alpha(110) },
                ),
            );

            // Title (indented past the swatch)
            let text_col = if is_sel {
                Color32::WHITE
            } else if is_dark {
                Color32::from_rgb(250, 250, 255)
            } else {
                LcdPalette::TEXT_BLACK
            };
            painter.text(
                Pos2::new(item_rect.min.x + 28.0_f32 * scale, item_rect.center().y),
                egui::Align2::LEFT_CENTER,
                name,
                FontId::proportional(12.0_f32 * scale),
                text_col,
            );

            // Detail (right side)
            if let Some(det) = detail {
                painter.text(
                    Pos2::new(item_rect.max.x - if *has_arrow { 22.0_f32 * scale } else { 8.0_f32 * scale }, item_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    det,
                    FontId::proportional(11.0_f32 * scale),
                    if is_sel {
                        Color32::from_rgb(220, 235, 255)
                    } else if is_dark {
                        Color32::from_rgb(195, 205, 220)
                    } else {
                        LcdPalette::TEXT_GRAY
                    },
                );
            }

            // Arrow indicator
            if *has_arrow {
                painter.text(
                    Pos2::new(item_rect.max.x - 8.0_f32 * scale, item_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    ">",
                    FontId::proportional(12.5_f32 * scale),
                    if is_sel {
                        Color32::WHITE
                    } else if is_dark {
                        Color32::from_white_alpha(170)
                    } else {
                        Color32::from_rgb(140, 145, 155)
                    },
                );
            }
        }

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 120.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "Customize Color, Display, and Shell styles",
            FontId::proportional(10.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_discord_settings_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Discord", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let rpc_status = if state.discord_enabled { "On" } else { "Off" };
        let mode_detail = if state.discord_enabled {
            state.discord_display_mode.name()
        } else {
            "Disabled"
        };

        let items = [
            ("Discord Presence", rpc_status, false),
            ("Display Mode", mode_detail, state.discord_enabled),
        ];

        let selected_idx = state.get_selected_index("discord_settings").min(items.len() - 1);
        let item_h = 24.0_f32 * scale;

        for (i, (name, detail, has_arrow)) in items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            let is_disabled = i == 1 && !state.discord_enabled;
            if is_disabled {
                // Render greyed-out disabled row
                let text_col = if is_dark { Color32::from_rgb(130, 135, 145) } else { Color32::from_rgb(170, 175, 185) };
                painter.text(
                    Pos2::new(item_rect.min.x + 8.0_f32 * scale, item_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    name,
                    FontId::proportional(12.0_f32 * scale),
                    text_col,
                );
                painter.text(
                    Pos2::new(item_rect.max.x - 8.0_f32 * scale, item_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    detail,
                    FontId::proportional(11.0_f32 * scale),
                    text_col,
                );
            } else {
                LcdRenderer::draw_list_item(
                    painter,
                    item_rect,
                    name,
                    Some(detail),
                    *has_arrow,
                    i == selected_idx,
                    is_dark,
                );
            }
        }

        // Informative note below options
        let desc = if state.discord_enabled {
            match state.discord_display_mode {
                crate::discord::DiscordDisplayMode::AppOnly => "Shows 'Listening on Epod' on your Discord profile",
                crate::discord::DiscordDisplayMode::SongTitle => "Shows current song title on your Discord profile",
                crate::discord::DiscordDisplayMode::Artist => "Shows current artist name on your Discord profile",
                crate::discord::DiscordDisplayMode::SongAndArtist => "Shows '(Song - Artist)' on your Discord profile",
            }
        } else {
            "Discord Rich Presence is currently turned off"
        };

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 110.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            desc,
            FontId::proportional(10.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_music_sources_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Music Sources", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let mut items = vec![
            "[+] Add Music Folder...".to_string(),
            "[↺] Rescan All Sources".to_string(),
        ];

        for src in &library.music_sources {
            let display_path = if let Some(name) = src.file_name().and_then(|n| n.to_str()) {
                format!("📁 {}", name)
            } else {
                format!("📁 {:?}", src)
            };
            items.push(display_path);
        }

        let selected_idx = state.get_selected_index("music_sources").min(items.len().saturating_sub(1));
        let item_h = 22.0_f32 * scale;
        let visible_count = ((content_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if selected_idx >= visible_count {
            selected_idx - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let item_idx = scroll_offset + i;
            if item_idx >= items.len() {
                break;
            }

            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            let detail = if item_idx >= 2 {
                Some("Remove >")
            } else {
                None
            };

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                &items[item_idx],
                detail,
                item_idx < 2,
                item_idx == selected_idx,
                is_dark,
            );
        }

        if items.len() > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (content_rect.height() * (visible_count as f32 / items.len() as f32)).max(12.0_f32 * scale);
            let thumb_y = content_rect.min.y + (content_rect.height() - thumb_h) * (scroll_offset as f32 / (items.len() - visible_count) as f32);
            painter.rect_filled(Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)), 2.0_f32 * scale, Color32::from_rgb(180, 185, 195));
        }
    }

    pub fn render_custom_eq_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "10-Band Graphic EQ", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let bands = [
            ("32Hz", "Sub"),
            ("64Hz", "Low"),
            ("125", "Bass"),
            ("250", "MBass"),
            ("500", "LMid"),
            ("1k", "Mid"),
            ("2k", "HMid"),
            ("4k", "Pres"),
            ("8k", "Treb"),
            ("16k", "Air"),
        ];

        let num_bands = 10;
        let slider_w = (content_rect.width() / (num_bands as f32) - 4.0_f32 * scale).clamp(10.0_f32 * scale, 28.0_f32 * scale);
        let slider_h = (content_rect.height() * 0.52_f32).max(40.0_f32);
        let spacing = content_rect.width() / (num_bands as f32);

        let center_y = content_rect.min.y + (content_rect.height() * 0.40_f32);
        let line_col = if is_dark { Color32::from_white_alpha(45) } else { Color32::from_rgb(220, 225, 235) };
        painter.line_segment(
            [Pos2::new(content_rect.min.x + 6.0_f32 * scale, center_y), Pos2::new(content_rect.max.x - 6.0_f32 * scale, center_y)],
            Stroke::new(1.0_f32 * scale, line_col),
        );

        for i in 0..num_bands {
            let x = content_rect.min.x + spacing * (i as f32 + 0.5_f32);
            let is_selected = i == state.selected_eq_band;

            let track_rect = Rect::from_center_size(Pos2::new(x, center_y), Vec2::new(3.0_f32 * scale, slider_h));
            let track_col = if is_dark { Color32::from_white_alpha(60) } else { Color32::from_rgb(215, 220, 230) };
            painter.rect_filled(track_rect, 1.5_f32 * scale, track_col);

            let gain_db = player.custom_eq_bands[i].clamp(-12.0_f32, 12.0_f32);
            let thumb_y = center_y - (gain_db / 12.0_f32) * (slider_h * 0.45_f32);

            let thumb_rect = Rect::from_center_size(
                Pos2::new(x, thumb_y),
                Vec2::new(slider_w, 10.0_f32 * scale),
            );

            let thumb_color = if is_selected {
                Color32::from_rgb(30, 110, 235)
            } else if is_dark {
                Color32::from_rgb(150, 155, 165)
            } else {
                Color32::from_rgb(120, 125, 135)
            };

            painter.rect_filled(thumb_rect, 2.5_f32 * scale, thumb_color);
            painter.rect_stroke(thumb_rect, 2.5_f32 * scale, Stroke::new(1.0_f32 * scale, Color32::WHITE));

            let val_str = format!("{:+.0}", gain_db);
            painter.text(
                Pos2::new(x, content_rect.min.y + 10.0_f32 * scale),
                egui::Align2::CENTER_CENTER,
                val_str,
                FontId::proportional(8.5_f32 * scale),
                if is_selected { Color32::from_rgb(70, 150, 255) } else { LcdPalette::text_secondary(is_dark) },
            );

            painter.text(
                Pos2::new(x, content_rect.max.y - 36.0_f32 * scale),
                egui::Align2::CENTER_CENTER,
                bands[i].0,
                FontId::proportional(9.0_f32 * scale),
                if is_selected { LcdPalette::text_primary(is_dark) } else { LcdPalette::text_secondary(is_dark) },
            );

            let sub_color = if i < 4 {
                Color32::from_rgb(240, 70, 50)
            } else if i < 7 {
                Color32::from_rgb(50, 170, 80)
            } else {
                Color32::from_rgb(50, 130, 245)
            };

            painter.text(
                Pos2::new(x, content_rect.max.y - 24.0_f32 * scale),
                egui::Align2::CENTER_CENTER,
                bands[i].1,
                FontId::proportional(8.0_f32 * scale),
                if is_selected { sub_color } else { if is_dark { Color32::from_rgb(170, 175, 185) } else { Color32::from_rgb(150, 155, 165) } },
            );
        }

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.max.y - 9.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "Wheel: Adjust dB  |  Select: Next Band",
            FontId::proportional(9.5_f32 * scale),
            if is_dark { Color32::from_rgb(190, 195, 205) } else { Color32::from_rgb(100, 105, 115) },
        );
    }

    pub fn update_action_btn_rect(content_rect: Rect, scale: f32) -> Rect {
        let btn_w = (content_rect.width() - 48.0_f32 * scale).min(200.0_f32 * scale);
        let btn_h = 24.0_f32 * scale;
        let btn_y = content_rect.min.y + 130.0_f32 * scale;
        Rect::from_center_size(Pos2::new(content_rect.center().x, btn_y), Vec2::new(btn_w, btn_h))
    }

    pub fn render_software_update_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let update_available = matches!(state.update_status, crate::updater::UpdateStatus::UpdateAvailable { .. });
        LcdRenderer::draw_status_bar_with_update(painter, screen_rect, "Software Update", player, state.is_hold_locked, state.display_theme, update_available);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let commit_prefix = if crate::updater::CURRENT_COMMIT_SHA.len() >= 7 {
            &crate::updater::CURRENT_COMMIT_SHA[..7]
        } else {
            crate::updater::CURRENT_COMMIT_SHA
        };
        let current_ver_str = format!("v{} ({})", env!("CARGO_PKG_VERSION"), commit_prefix);

        // App Title & Current Version Header
        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 18.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "Project Epod Nightly",
            FontId::proportional(13.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 34.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            &format!("Installed: {}", current_ver_str),
            FontId::proportional(10.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        // Card frame
        let card_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x, content_rect.min.y + 110.0_f32 * scale),
            Vec2::new(content_rect.width() - 24.0_f32 * scale, 120.0_f32 * scale),
        );
        let card_bg = if is_dark {
            Color32::from_white_alpha(15)
        } else {
            Color32::from_black_alpha(10)
        };
        painter.rect_filled(card_rect, 6.0_f32 * scale, card_bg);
        painter.rect_stroke(card_rect, 6.0_f32 * scale, Stroke::new(0.5_f32 * scale, Color32::from_black_alpha(30)));

        let btn_rect = Self::update_action_btn_rect(content_rect, scale);
        let accent_blue = Color32::from_rgb(40, 130, 240);

        match &state.update_status {
            crate::updater::UpdateStatus::Idle | crate::updater::UpdateStatus::Checking => {
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 35.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "⟳ Checking for updates...",
                    FontId::proportional(11.5_f32 * scale),
                    LcdPalette::text_primary(is_dark),
                );
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 55.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "Connecting to GitHub Releases...",
                    FontId::proportional(9.0_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );
            }
            crate::updater::UpdateStatus::UpToDate => {
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 35.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "✔ You are up to date",
                    FontId::proportional(12.0_f32 * scale),
                    Color32::from_rgb(46, 204, 113),
                );
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 55.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "Epod is running the latest build.",
                    FontId::proportional(9.5_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );
                // Check Again Button
                painter.rect_filled(btn_rect, 4.0_f32 * scale, if is_dark { Color32::from_white_alpha(25) } else { Color32::from_black_alpha(20) });
                painter.text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Check for Updates",
                    FontId::proportional(10.0_f32 * scale),
                    LcdPalette::text_primary(is_dark),
                );
            }
            crate::updater::UpdateStatus::UpdateAvailable { remote_sha, .. } => {
                let target_sha_prefix = if remote_sha.len() >= 7 { &remote_sha[..7] } else { remote_sha.as_str() };
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 25.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "★ New Version Available!",
                    FontId::proportional(12.0_f32 * scale),
                    Color32::from_rgb(255, 185, 30),
                );
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 44.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    &format!("Latest Nightly: {}", target_sha_prefix),
                    FontId::proportional(9.5_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );

                // Highlighted Action Button
                painter.rect_filled(btn_rect, 4.0_f32 * scale, accent_blue);
                painter.rect_stroke(btn_rect, 4.0_f32 * scale, Stroke::new(0.6_f32 * scale, Color32::from_white_alpha(150)));
                painter.text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Download & Install Update",
                    FontId::proportional(10.0_f32 * scale),
                    Color32::WHITE,
                );
            }
            crate::updater::UpdateStatus::Downloading(progress) => {
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 25.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    &format!("Downloading Update... {:.0}%", progress * 100.0),
                    FontId::proportional(11.0_f32 * scale),
                    LcdPalette::text_primary(is_dark),
                );

                // Progress Bar
                let pb_w = (card_rect.width() - 30.0_f32 * scale).max(50.0_f32);
                let pb_h = 7.0_f32 * scale;
                let pb_rect = Rect::from_center_size(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 50.0_f32 * scale),
                    Vec2::new(pb_w, pb_h),
                );
                painter.rect_filled(pb_rect, 3.5_f32 * scale, Color32::from_black_alpha(40));
                let fill_rect = Rect::from_min_size(pb_rect.min, Vec2::new(pb_w * progress.clamp(0.0, 1.0), pb_h));
                painter.rect_filled(fill_rect, 3.5_f32 * scale, accent_blue);
            }
            crate::updater::UpdateStatus::InstalledRestartRequired => {
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 25.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "✔ Update Installed!",
                    FontId::proportional(12.0_f32 * scale),
                    Color32::from_rgb(46, 204, 113),
                );
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 44.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "Restart Epod to finish updating",
                    FontId::proportional(9.5_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );

                // Restart Button
                let green_accent = Color32::from_rgb(39, 174, 96);
                painter.rect_filled(btn_rect, 4.0_f32 * scale, green_accent);
                painter.rect_stroke(btn_rect, 4.0_f32 * scale, Stroke::new(0.6_f32 * scale, Color32::from_white_alpha(150)));
                painter.text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Restart Epod Now",
                    FontId::proportional(10.5_f32 * scale),
                    Color32::WHITE,
                );
            }
            crate::updater::UpdateStatus::Error(err) => {
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 22.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    "⚠ Update Check Failed",
                    FontId::proportional(11.0_f32 * scale),
                    Color32::from_rgb(231, 76, 60),
                );
                let err_display = if err.len() > 36 { format!("{}...", &err[..33]) } else { err.clone() };
                painter.text(
                    Pos2::new(card_rect.center().x, card_rect.min.y + 42.0_f32 * scale),
                    egui::Align2::CENTER_CENTER,
                    &err_display,
                    FontId::proportional(8.5_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );

                // Retry Button
                painter.rect_filled(btn_rect, 4.0_f32 * scale, if is_dark { Color32::from_white_alpha(25) } else { Color32::from_black_alpha(20) });
                painter.text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Retry Check",
                    FontId::proportional(10.0_f32 * scale),
                    LcdPalette::text_primary(is_dark),
                );
            }
        }
    }

    pub fn render_about_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let update_available = matches!(state.update_status, crate::updater::UpdateStatus::UpdateAvailable { .. });
        LcdRenderer::draw_status_bar_with_update(painter, screen_rect, "About", player, state.is_hold_locked, state.display_theme, update_available);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let song_count = library.songs.len().to_string();
        let video_count = library.videos.len().to_string();
        let sources_count = library.music_sources.len().to_string();
        let commit_prefix = if crate::updater::CURRENT_COMMIT_SHA.len() >= 7 {
            &crate::updater::CURRENT_COMMIT_SHA[..7]
        } else {
            crate::updater::CURRENT_COMMIT_SHA
        };
        let version_str = format!("v{} ({})", env!("CARGO_PKG_VERSION"), commit_prefix);
        let info = [
            ("Name", "Jaep's Epod"),
            ("Songs", song_count.as_str()),
            ("Videos", video_count.as_str()),
            ("Music Folders", sources_count.as_str()),
            ("Capacity", "30.0 GB"),
            ("Available", "27.8 GB"),
            ("Version", version_str.as_str()),
            ("Model", "MA002LL"),
            ("Format", "Windows (FAT32)"),
            ("S/N", "8K54271UV9R"),
        ];

        let selected_idx = state.get_selected_index("about_screen").min(info.len() - 1);
        let item_h = 22.0_f32 * scale;

        for (i, (label, val)) in info.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                label,
                Some(val),
                false,
                i == selected_idx,
                is_dark,
            );
        }
    }

    pub fn render_brightness_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Brightness", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 50.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "☀️",
            FontId::proportional(28.0_f32 * scale),
            Color32::from_rgb(240, 160, 20),
        );

        let bar_w = 200.0_f32 * scale;
        let bar_h_val = 16.0_f32 * scale;
        let bar_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x, content_rect.min.y + 110.0_f32 * scale),
            Vec2::new(bar_w, bar_h_val),
        );

        let bg_col = if is_dark { Color32::from_black_alpha(150) } else { Color32::from_rgb(220, 225, 235) };
        let border_col = if is_dark { Color32::from_white_alpha(70) } else { Color32::from_rgb(180, 185, 195) };

        painter.rect_filled(bar_rect, 4.0_f32 * scale, bg_col);
        painter.rect_stroke(bar_rect, 4.0_f32 * scale, Stroke::new(1.0_f32 * scale, border_col));

        let fill_w = (bar_w - 4.0_f32 * scale) * (state.brightness_percent as f32 / 100.0_f32);
        let fill_rect = Rect::from_min_size(
            Pos2::new(bar_rect.min.x + 2.0_f32 * scale, bar_rect.min.y + 2.0_f32 * scale),
            Vec2::new(fill_w, bar_h_val - 4.0_f32 * scale),
        );
        painter.rect_filled(fill_rect, 2.0_f32 * scale, Color32::from_rgb(40, 120, 235));

        let percent_str = format!("{}%", state.brightness_percent);
        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 150.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            percent_str,
            FontId::proportional(14.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 180.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "Turn Click Wheel to adjust",
            FontId::proportional(11.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_album_cover_config_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Album Cover Theme", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let blur_str = format!("{} px", state.album_art_blur);
        let bright_str = format!("{}%", state.album_art_brightness);

        let items = [
            ("Blur", blur_str.as_str()),
            ("Brightness", bright_str.as_str()),
        ];

        let selected_idx = state.get_selected_index("album_cover_config").min(items.len() - 1);
        let item_h = 24.0_f32 * scale;

        for (i, (name, detail)) in items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                name,
                Some(detail),
                true,
                i == selected_idx,
                is_dark,
            );
        }

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 120.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "Customize the screen backdrop appearance",
            FontId::proportional(10.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_album_cover_blur_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Cover Blur", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 50.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "🌫️",
            FontId::proportional(28.0_f32 * scale),
            Color32::from_rgb(140, 185, 245),
        );

        let bar_w = 200.0_f32 * scale;
        let bar_h_val = 16.0_f32 * scale;
        let bar_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x, content_rect.min.y + 110.0_f32 * scale),
            Vec2::new(bar_w, bar_h_val),
        );

        let bg_col = if is_dark { Color32::from_black_alpha(150) } else { Color32::from_rgb(220, 225, 235) };
        let border_col = if is_dark { Color32::from_white_alpha(70) } else { Color32::from_rgb(180, 185, 195) };

        painter.rect_filled(bar_rect, 4.0_f32 * scale, bg_col);
        painter.rect_stroke(bar_rect, 4.0_f32 * scale, Stroke::new(1.0_f32 * scale, border_col));

        let max_blur = 30.0_f32;
        let fill_w = (bar_w - 4.0_f32 * scale) * ((state.album_art_blur as f32 / max_blur).clamp(0.0_f32, 1.0_f32));
        let fill_rect = Rect::from_min_size(
            Pos2::new(bar_rect.min.x + 2.0_f32 * scale, bar_rect.min.y + 2.0_f32 * scale),
            Vec2::new(fill_w, bar_h_val - 4.0_f32 * scale),
        );
        painter.rect_filled(fill_rect, 2.0_f32 * scale, Color32::from_rgb(60, 150, 245));

        let blur_str = if state.album_art_blur == 0 {
            "Sharp (0 px)".to_string()
        } else {
            format!("{} px", state.album_art_blur)
        };

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 150.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            blur_str,
            FontId::proportional(14.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 180.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "Turn Click Wheel to adjust blur",
            FontId::proportional(11.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_album_cover_brightness_screen(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Cover Brightness", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 50.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "💡",
            FontId::proportional(28.0_f32 * scale),
            Color32::from_rgb(250, 200, 50),
        );

        let bar_w = 200.0_f32 * scale;
        let bar_h_val = 16.0_f32 * scale;
        let bar_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x, content_rect.min.y + 110.0_f32 * scale),
            Vec2::new(bar_w, bar_h_val),
        );

        let bg_col = if is_dark { Color32::from_black_alpha(150) } else { Color32::from_rgb(220, 225, 235) };
        let border_col = if is_dark { Color32::from_white_alpha(70) } else { Color32::from_rgb(180, 185, 195) };

        painter.rect_filled(bar_rect, 4.0_f32 * scale, bg_col);
        painter.rect_stroke(bar_rect, 4.0_f32 * scale, Stroke::new(1.0_f32 * scale, border_col));

        let fill_w = (bar_w - 4.0_f32 * scale) * ((state.album_art_brightness as f32 / 100.0_f32).clamp(0.0_f32, 1.0_f32));
        let fill_rect = Rect::from_min_size(
            Pos2::new(bar_rect.min.x + 2.0_f32 * scale, bar_rect.min.y + 2.0_f32 * scale),
            Vec2::new(fill_w, bar_h_val - 4.0_f32 * scale),
        );
        painter.rect_filled(fill_rect, 2.0_f32 * scale, Color32::from_rgb(245, 180, 40));

        let percent_str = format!("{}%", state.album_art_brightness);
        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 150.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            percent_str,
            FontId::proportional(14.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 180.0_f32 * scale),
            egui::Align2::CENTER_CENTER,
            "Turn Click Wheel to adjust brightness",
            FontId::proportional(11.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_custom_color_category_list(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Custom Theme", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let categories = CustomThemeCategory::ALL;
        let items = [
            ("[★] Save as New Preset...", None, true),
            ("Saved Presets", Some(format!("{} saved", state.custom_presets.len())), true),
            ("Reset to Defaults", None, false),
            (categories[0].name(), Some(format!("{} items", categories[0].targets().len())), true),
            (categories[1].name(), Some(format!("{} items", categories[1].targets().len())), true),
            (categories[2].name(), Some(format!("{} items", categories[2].targets().len())), true),
            (categories[3].name(), Some(format!("{} items", categories[3].targets().len())), true),
        ];

        let selected_idx = state.get_selected_index("custom_color_category_list").min(items.len() - 1);
        let item_h = 22.0_f32 * scale;

        for (i, (name, detail, has_arrow)) in items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                name,
                detail.as_deref(),
                *has_arrow,
                i == selected_idx,
                is_dark,
            );
        }

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.max.y - 8.0_f32 * scale),
            egui::Align2::CENTER_BOTTOM,
            "Customize colors • Save & manage custom presets",
            FontId::proportional(9.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_custom_color_target_list(
        painter: &Painter,
        screen_rect: Rect,
        category: CustomThemeCategory,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, category.name(), player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let targets = category.targets();
        let selected_idx = state.get_selected_index("custom_color_target_list").min(targets.len().saturating_sub(1));
        let item_h = 24.0_f32 * scale;

        for (i, target) in targets.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            let hex = state.custom_theme.get_hex(*target);
            let color32 = state.custom_theme.get_color32(*target);

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                target.name(),
                Some(&hex),
                true,
                i == selected_idx,
                is_dark,
            );

            // Draw small color preview square next to the hex code
            let preview_size = 11.0_f32 * scale;
            let preview_rect = Rect::from_center_size(
                Pos2::new(item_rect.max.x - 72.0_f32 * scale, item_rect.center().y),
                Vec2::splat(preview_size),
            );
            painter.rect_filled(preview_rect, 2.0_f32 * scale, color32);
            painter.rect_stroke(
                preview_rect,
                2.0_f32 * scale,
                Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(100) } else { Color32::from_black_alpha(100) }),
            );
        }

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.max.y - 12.0_f32 * scale),
            egui::Align2::CENTER_BOTTOM,
            "Select an element to customize with Color Picker",
            FontId::proportional(9.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_color_picker_screen(
        painter: &Painter,
        screen_rect: Rect,
        target: ColorTarget,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, target.name(), player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let [r, g, b] = state.custom_theme.get_color(target);
        let hex = rgb_to_hex(r, g, b);
        let (h, s, v) = rgb_to_hsv(r, g, b);
        let curr_color = Color32::from_rgb(r, g, b);

        // 1. Top Section: Live Color Swatch + Hex Code + Active Mode Badges
        let swatch_w = 42.0_f32 * scale;
        let swatch_h = 22.0_f32 * scale;
        let top_y = content_rect.min.y + 10.0_f32 * scale;
        let swatch_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x + 18.0_f32 * scale, top_y),
            Vec2::new(swatch_w, swatch_h),
        );
        // Drop shadow & swatch fill
        painter.rect_filled(swatch_rect.expand(1.5_f32 * scale), 4.0_f32 * scale, Color32::from_black_alpha(40));
        painter.rect_filled(swatch_rect, 4.0_f32 * scale, curr_color);
        painter.rect_stroke(
            swatch_rect,
            4.0_f32 * scale,
            Stroke::new(1.5_f32 * scale, if is_dark { Color32::from_white_alpha(120) } else { Color32::from_black_alpha(80) }),
        );

        // Hex Code
        painter.text(
            Pos2::new(swatch_rect.max.x + 10.0_f32 * scale, swatch_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &hex,
            FontId::proportional(14.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        // Mode Selector Badges on the right: [Sat] [Hue] [Val]
        let modes = ["Sat", "Hue", "Val"];
        let mode_start_x = content_rect.max.x - 110.0_f32 * scale;
        for (m_idx, m_name) in modes.iter().enumerate() {
            let m_rect = Rect::from_min_size(
                Pos2::new(mode_start_x + (m_idx as f32) * 34.0_f32 * scale, top_y + 2.0_f32 * scale),
                Vec2::new(30.0_f32 * scale, 18.0_f32 * scale),
            );
            let is_active_mode = state.color_picker_mode == m_idx;
            let badge_bg = if is_active_mode {
                Color32::from_rgb(30, 110, 235)
            } else if is_dark {
                Color32::from_white_alpha(35)
            } else {
                Color32::from_rgb(220, 225, 235)
            };
            painter.rect_filled(m_rect, 3.0_f32 * scale, badge_bg);
            painter.text(
                m_rect.center(),
                egui::Align2::CENTER_CENTER,
                *m_name,
                FontId::proportional(9.5_f32 * scale),
                if is_active_mode { Color32::WHITE } else { LcdPalette::text_secondary(is_dark) },
            );
        }

        let slider_w = content_rect.width() - 36.0_f32 * scale;
        let slider_h = 10.0_f32 * scale;
        let slider_x = content_rect.min.x + 18.0_f32 * scale;

        // 2. Middle Section:
        // A) Spectrum / Hue Bar
        let hue_y = content_rect.min.y + 44.0_f32 * scale;
        let hue_rect = Rect::from_min_size(Pos2::new(slider_x, hue_y), Vec2::new(slider_w, slider_h));
        
        let num_segments = 24;
        let seg_w = slider_w / (num_segments as f32);
        for seg in 0..num_segments {
            let seg_h_val = (seg as f32 / num_segments as f32) * 360.0_f32;
            let seg_col = hsv_to_rgb(seg_h_val, 1.0, 1.0);
            let seg_rect = Rect::from_min_size(
                Pos2::new(slider_x + (seg as f32) * seg_w, hue_y),
                Vec2::new(seg_w + 0.5_f32, slider_h),
            );
            painter.rect_filled(seg_rect, 0.0_f32, Color32::from_rgb(seg_col[0], seg_col[1], seg_col[2]));
        }
        painter.rect_stroke(
            hue_rect,
            2.0_f32 * scale,
            Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(80) } else { Color32::from_black_alpha(60) }),
        );

        // Hue Indicator Cursor
        let hue_cursor_x = slider_x + (h / 360.0_f32) * slider_w;
        let hue_cursor_rect = Rect::from_center_size(
            Pos2::new(hue_cursor_x, hue_rect.center().y),
            Vec2::new(5.0_f32 * scale, slider_h + 4.0_f32 * scale),
        );
        painter.rect_filled(hue_cursor_rect, 2.0_f32 * scale, Color32::WHITE);
        painter.rect_stroke(hue_cursor_rect, 2.0_f32 * scale, Stroke::new(1.0_f32 * scale, Color32::BLACK));

        painter.text(
            Pos2::new(slider_x, hue_y - 2.0_f32 * scale),
            egui::Align2::LEFT_BOTTOM,
            format!("Hue: {:.0}°", h),
            FontId::proportional(9.0_f32 * scale),
            if state.color_picker_mode == 1 { Color32::from_rgb(40, 130, 245) } else { LcdPalette::text_secondary(is_dark) },
        );

        // B) Saturation Slider (Under Hue spectrum)
        let sat_y = hue_y + 26.0_f32 * scale;
        let sat_rect = Rect::from_min_size(Pos2::new(slider_x, sat_y), Vec2::new(slider_w, slider_h));

        for seg in 0..num_segments {
            let seg_s = seg as f32 / num_segments as f32;
            let seg_col = hsv_to_rgb(h, seg_s, v.max(0.2));
            let seg_rect = Rect::from_min_size(
                Pos2::new(slider_x + (seg as f32) * seg_w, sat_y),
                Vec2::new(seg_w + 0.5_f32, slider_h),
            );
            painter.rect_filled(seg_rect, 0.0_f32, Color32::from_rgb(seg_col[0], seg_col[1], seg_col[2]));
        }
        painter.rect_stroke(
            sat_rect,
            2.0_f32 * scale,
            Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(80) } else { Color32::from_black_alpha(60) }),
        );

        // Saturation Cursor
        let sat_cursor_x = slider_x + s.clamp(0.0, 1.0) * slider_w;
        let sat_cursor_rect = Rect::from_center_size(
            Pos2::new(sat_cursor_x, sat_rect.center().y),
            Vec2::new(5.0_f32 * scale, slider_h + 4.0_f32 * scale),
        );
        painter.rect_filled(sat_cursor_rect, 2.0_f32 * scale, Color32::WHITE);
        painter.rect_stroke(sat_cursor_rect, 2.0_f32 * scale, Stroke::new(1.0_f32 * scale, Color32::BLACK));

        painter.text(
            Pos2::new(slider_x, sat_y - 2.0_f32 * scale),
            egui::Align2::LEFT_BOTTOM,
            format!("Saturation: {:.0}%", s * 100.0),
            FontId::proportional(9.0_f32 * scale),
            if state.color_picker_mode == 0 { Color32::from_rgb(40, 130, 245) } else { LcdPalette::text_secondary(is_dark) },
        );

        // C) Brightness / Value Slider (Under Saturation)
        let val_y = sat_y + 26.0_f32 * scale;
        let val_rect = Rect::from_min_size(Pos2::new(slider_x, val_y), Vec2::new(slider_w, slider_h));

        for seg in 0..num_segments {
            let seg_v = seg as f32 / num_segments as f32;
            let seg_col = hsv_to_rgb(h, s, seg_v);
            let seg_rect = Rect::from_min_size(
                Pos2::new(slider_x + (seg as f32) * seg_w, val_y),
                Vec2::new(seg_w + 0.5_f32, slider_h),
            );
            painter.rect_filled(seg_rect, 0.0_f32, Color32::from_rgb(seg_col[0], seg_col[1], seg_col[2]));
        }
        painter.rect_stroke(
            val_rect,
            2.0_f32 * scale,
            Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(80) } else { Color32::from_black_alpha(60) }),
        );

        // Value Cursor
        let val_cursor_x = slider_x + v.clamp(0.0, 1.0) * slider_w;
        let val_cursor_rect = Rect::from_center_size(
            Pos2::new(val_cursor_x, val_rect.center().y),
            Vec2::new(5.0_f32 * scale, slider_h + 4.0_f32 * scale),
        );
        painter.rect_filled(val_cursor_rect, 2.0_f32 * scale, Color32::WHITE);
        painter.rect_stroke(val_cursor_rect, 2.0_f32 * scale, Stroke::new(1.0_f32 * scale, Color32::BLACK));

        painter.text(
            Pos2::new(slider_x, val_y - 2.0_f32 * scale),
            egui::Align2::LEFT_BOTTOM,
            format!("Brightness: {:.0}%", v * 100.0),
            FontId::proportional(9.0_f32 * scale),
            if state.color_picker_mode == 2 { Color32::from_rgb(40, 130, 245) } else { LcdPalette::text_secondary(is_dark) },
        );

        // 3. Quick Palette Swatches at the bottom
        let swatches: [[u8; 3]; 9] = [
            [245, 246, 248], // White
            [22, 22, 24],    // Black
            [208, 30, 45],   // Red
            [238, 110, 25],  // Orange
            [238, 172, 34],  // Yellow
            [88, 172, 108],  // Green
            [42, 118, 202],  // Blue
            [145, 65, 215],  // Purple
            [105, 68, 54],   // Brown
        ];

        let swatch_circle_r = 7.0_f32 * scale;
        let swatch_spacing = slider_w / (swatches.len() as f32);
        let swatch_row_y = val_y + 24.0_f32 * scale;

        for (i, [sr, sg, sb]) in swatches.iter().enumerate() {
            let sc_center = Pos2::new(slider_x + (i as f32 + 0.5_f32) * swatch_spacing, swatch_row_y);
            painter.circle_filled(sc_center, swatch_circle_r, Color32::from_rgb(*sr, *sg, *sb));
            painter.circle_stroke(
                sc_center,
                swatch_circle_r,
                Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(80) } else { Color32::from_black_alpha(70) }),
            );
        }

        // Hint text
        painter.text(
            Pos2::new(content_rect.center().x, content_rect.max.y - 8.0_f32 * scale),
            egui::Align2::CENTER_BOTTOM,
            "Turn wheel • Click center to switch slider • Touch sliders to pick",
            FontId::proportional(8.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
    }

    pub fn render_color_presets_manager(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Color Presets", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let total_items = 1 + state.custom_presets.len();
        let selected_idx = state.get_selected_index("color_presets_manager").min(total_items.saturating_sub(1));
        let item_h = 24.0_f32 * scale;
        let visible_count = ((content_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if selected_idx >= visible_count {
            selected_idx - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let item_idx = scroll_offset + i;
            if item_idx >= total_items {
                break;
            }

            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            if item_idx == 0 {
                // Top Action: [+] Save Current as New Preset...
                LcdRenderer::draw_list_item(
                    painter,
                    item_rect,
                    "[+] Save Current as Preset...",
                    None,
                    true,
                    item_idx == selected_idx,
                    is_dark,
                );
            } else {
                let preset_idx = item_idx - 1;
                let preset = &state.custom_presets[preset_idx];
                let is_active = state.active_preset_id.as_deref() == Some(&preset.id);
                let detail_text = if is_active { Some("✔ >") } else { Some(">") };

                LcdRenderer::draw_list_item(
                    painter,
                    item_rect,
                    &preset.name,
                    detail_text,
                    false,
                    item_idx == selected_idx,
                    is_dark,
                );

                // Two small color swatch preview dots for shell and wheel
                let dot_r = 4.5_f32 * scale;
                let shell_dot_center = Pos2::new(item_rect.max.x - 42.0_f32 * scale, item_rect.center().y);
                let wheel_dot_center = Pos2::new(item_rect.max.x - 30.0_f32 * scale, item_rect.center().y);

                let [sr, sg, sb] = preset.config.shell_color;
                let [wr, wg, wb] = preset.config.wheel_color;

                painter.circle_filled(shell_dot_center, dot_r, Color32::from_rgb(sr, sg, sb));
                painter.circle_stroke(
                    shell_dot_center,
                    dot_r,
                    Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(100) } else { Color32::from_black_alpha(100) }),
                );

                painter.circle_filled(wheel_dot_center, dot_r, Color32::from_rgb(wr, wg, wb));
                painter.circle_stroke(
                    wheel_dot_center,
                    dot_r,
                    Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(100) } else { Color32::from_black_alpha(100) }),
                );
            }
        }

        if total_items > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (content_rect.height() * (visible_count as f32 / total_items as f32)).max(12.0_f32 * scale);
            let thumb_y = content_rect.min.y + (content_rect.height() - thumb_h) * (scroll_offset as f32 / (total_items - visible_count) as f32);
            painter.rect_filled(Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)), 2.0_f32 * scale, Color32::from_rgb(180, 185, 195));
        }
    }

    pub fn render_preset_options_screen(
        painter: &Painter,
        screen_rect: Rect,
        preset_id: &str,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        
        let preset_opt = state.custom_presets.iter().find(|p| p.id == preset_id);
        let title_name = preset_opt.map(|p| p.name.as_str()).unwrap_or("Preset Options");
        LcdRenderer::draw_status_bar(painter, screen_rect, title_name, player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let options = [
            ("Apply Preset", false),
            ("Overwrite with Current Colors", false),
            ("Rename Preset", true),
            ("Delete Preset", false),
        ];

        let selected_idx = state.get_selected_index("preset_options").min(options.len() - 1);
        let item_h = 24.0_f32 * scale;

        for (i, (name, has_arrow)) in options.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                name,
                None,
                *has_arrow,
                i == selected_idx,
                is_dark,
            );
        }

        // Color Palette Preview Card at the bottom
        if let Some(preset) = preset_opt {
            let card_y = content_rect.min.y + (options.len() as f32) * item_h + 10.0_f32 * scale;
            let card_h = 50.0_f32 * scale;
            let card_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x + 16.0_f32 * scale, card_y),
                Vec2::new(content_rect.width() - 32.0_f32 * scale, card_h),
            );

            let [sr, sg, sb] = preset.config.shell_color;
            let [wr, wg, wb] = preset.config.wheel_color;
            let [cr, cg, cb] = preset.config.center_button_color;

            painter.rect_filled(card_rect, 6.0_f32 * scale, if is_dark { Color32::from_black_alpha(100) } else { Color32::from_rgb(240, 243, 248) });
            painter.rect_stroke(card_rect, 6.0_f32 * scale, Stroke::new(1.0_f32 * scale, if is_dark { Color32::from_white_alpha(50) } else { Color32::from_rgb(210, 215, 225) }));

            // Swatch pills
            let swatch_w = 28.0_f32 * scale;
            let swatch_h = 20.0_f32 * scale;
            let start_x = card_rect.min.x + 16.0_f32 * scale;
            let pill_y = card_rect.min.y + 15.0_f32 * scale;

            painter.rect_filled(Rect::from_min_size(Pos2::new(start_x, pill_y), Vec2::new(swatch_w, swatch_h)), 3.0_f32 * scale, Color32::from_rgb(sr, sg, sb));
            painter.rect_filled(Rect::from_min_size(Pos2::new(start_x + swatch_w + 10.0_f32 * scale, pill_y), Vec2::new(swatch_w, swatch_h)), 3.0_f32 * scale, Color32::from_rgb(wr, wg, wb));
            painter.rect_filled(Rect::from_min_size(Pos2::new(start_x + (swatch_w + 10.0_f32 * scale) * 2.0_f32, pill_y), Vec2::new(swatch_w, swatch_h)), 3.0_f32 * scale, Color32::from_rgb(cr, cg, cb));

            painter.text(
                Pos2::new(card_rect.center().x, card_rect.max.y - 6.0_f32 * scale),
                egui::Align2::CENTER_BOTTOM,
                "Shell • Wheel • Button preview",
                FontId::proportional(8.5_f32 * scale),
                LcdPalette::text_secondary(is_dark),
            );
        }
    }

    pub fn render_preset_name_input_screen(
        painter: &Painter,
        screen_rect: Rect,
        is_editing: bool,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let header_title = if is_editing { "Rename Preset" } else { "Name Preset" };
        LcdRenderer::draw_status_bar(painter, screen_rect, header_title, player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 14.0_f32 * scale),
            egui::Align2::CENTER_TOP,
            "Enter a name for this color preset:",
            FontId::proportional(10.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        // Text input display box
        let box_w = content_rect.width() - 36.0_f32 * scale;
        let box_h = 26.0_f32 * scale;
        let box_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x, content_rect.min.y + 50.0_f32 * scale),
            Vec2::new(box_w, box_h),
        );

        let box_bg = if is_dark { Color32::from_rgb(20, 22, 28) } else { Color32::from_rgb(255, 255, 255) };
        painter.rect_filled(box_rect, 4.0_f32 * scale, box_bg);
        painter.rect_stroke(box_rect, 4.0_f32 * scale, Stroke::new(1.5_f32 * scale, Color32::from_rgb(40, 120, 235)));

        let display_text = if state.preset_name_buffer.is_empty() {
            "Type preset name..."
        } else {
            &state.preset_name_buffer
        };
        let text_col = if state.preset_name_buffer.is_empty() {
            Color32::from_rgb(150, 155, 165)
        } else {
            LcdPalette::text_primary(is_dark)
        };

        painter.text(
            Pos2::new(box_rect.min.x + 10.0_f32 * scale, box_rect.center().y),
            egui::Align2::LEFT_CENTER,
            display_text,
            FontId::proportional(12.0_f32 * scale),
            text_col,
        );

        // Quick suggestions header
        painter.text(
            Pos2::new(content_rect.min.x + 18.0_f32 * scale, content_rect.min.y + 74.0_f32 * scale),
            egui::Align2::LEFT_TOP,
            "Quick suggestions (click to use):",
            FontId::proportional(9.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        let suggestions = ["Cyberpunk", "Pastel Mint", "Sunset", "Solar Flare", "Matcha", "Obsidian", "Lavender", "Cobalt"];
        let chip_h = 16.0_f32 * scale;
        let mut curr_x = content_rect.min.x + 18.0_f32 * scale;
        let mut curr_y = content_rect.min.y + 92.0_f32 * scale;

        for s in suggestions {
            let chip_w = (s.len() as f32 * 6.2_f32 * scale) + 12.0_f32 * scale;
            if curr_x + chip_w > content_rect.max.x - 18.0_f32 * scale {
                curr_x = content_rect.min.x + 18.0_f32 * scale;
                curr_y += chip_h + 5.0_f32 * scale;
            }
            let chip_rect = Rect::from_min_size(Pos2::new(curr_x, curr_y), Vec2::new(chip_w, chip_h));
            let chip_bg = if is_dark { Color32::from_white_alpha(35) } else { Color32::from_rgb(230, 235, 245) };
            painter.rect_filled(chip_rect, 3.0_f32 * scale, chip_bg);
            painter.text(
                chip_rect.center(),
                egui::Align2::CENTER_CENTER,
                s,
                FontId::proportional(8.5_f32 * scale),
                LcdPalette::text_primary(is_dark),
            );
            curr_x += chip_w + 5.0_f32 * scale;
        }

        // Save & Cancel Buttons at bottom
        let btn_w = 88.0_f32 * scale;
        let btn_h = 22.0_f32 * scale;
        let btn_y = content_rect.max.y - 24.0_f32 * scale;

        // Save Button
        let save_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x - 52.0_f32 * scale, btn_y),
            Vec2::new(btn_w, btn_h),
        );
        painter.rect_filled(save_rect, 4.0_f32 * scale, Color32::from_rgb(30, 110, 235));
        painter.text(
            save_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Save Preset",
            FontId::proportional(11.0_f32 * scale),
            Color32::WHITE,
        );

        // Cancel Button
        let cancel_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x + 52.0_f32 * scale, btn_y),
            Vec2::new(btn_w, btn_h),
        );
        painter.rect_filled(cancel_rect, 4.0_f32 * scale, if is_dark { Color32::from_white_alpha(40) } else { Color32::from_rgb(220, 225, 235) });
        painter.text(
            cancel_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Cancel",
            FontId::proportional(11.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );
    }
}
