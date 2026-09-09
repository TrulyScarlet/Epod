use crate::audio::player::{AudioPlayer, RepeatMode};
use crate::library::model::Library;
use crate::state::AppState;
use crate::theme::{ChassisColor, ColorTarget, CustomThemeConfig, DisplayTheme, LcdPalette};
use crate::ui::art_cache::ArtCache;
use egui::{Color32, Context, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub struct LcdRenderer;

impl LcdRenderer {
    pub fn draw_screen_background(
        painter: &Painter,
        ctx: &Context,
        rect: Rect,
        state: &AppState,
        library: &Library,
        art_cache: &mut ArtCache,
    ) {
        if state.display_theme.is_album_cover() {
            // Check if active song exists
            let song_info = state
                .current_queue
                .get(state.current_queue_idx)
                .and_then(|&id| library.songs.get(id))
                .filter(|s| !s.is_synthetic_demo);

            // Base backdrop: dark grey retro matrix for idle, deep tone for active cover
            let base_bg = if song_info.is_some() {
                Color32::from_rgb(12, 14, 20)
            } else {
                Color32::from_rgb(56, 60, 67)
            };
            painter.rect_filled(rect, 0.0_f32, base_bg);

            let (song_id, art_bytes, file_path, seed) = if let Some(s) = song_info {
                (s.id, s.artwork_bytes.as_deref(), s.file_path.as_deref(), s.id)
            } else {
                (usize::MAX, None, None, 0)
            };

            if let Some(texture) = art_cache.get_or_load_background(
                ctx,
                song_id,
                art_bytes,
                file_path,
                state.album_art_blur,
                seed,
            ) {
                let tint = if song_info.is_some() {
                    let b_factor = (state.album_art_brightness as f32 / 100.0_f32).clamp(0.05, 1.0);
                    let b_byte = (255.0 * b_factor).round() as u8;
                    Color32::from_rgba_unmultiplied(b_byte, b_byte, b_byte, 255)
                } else {
                    Color32::WHITE
                };

                painter.image(
                    texture.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0_f32, 1.0_f32)),
                    tint,
                );

                // Subtle dark overlay only when showing actual song album art
                if song_info.is_some() {
                    painter.rect_filled(rect, 0.0_f32, Color32::from_black_alpha(20));
                }
            }
        } else if state.chassis_color == ChassisColor::Custom {
            let bg = state.custom_theme.get_color32(ColorTarget::ScreenBg);
            painter.rect_filled(rect, 0.0_f32, bg);
        } else {
            painter.rect_filled(rect, 0.0_f32, state.display_theme.bg_color());
        }

        // 4-Sided crisp screen perimeter border
        let border_scale = (rect.width() / 333.0_f32).max(0.2_f32);
        painter.rect_stroke(
            rect,
            0.0_f32,
            Stroke::new((1.0_f32 * border_scale).max(0.75_f32), Color32::from_black_alpha(80)),
        );
    }

    pub fn draw_status_bar(
        painter: &Painter,
        screen_rect: Rect,
        title: &str,
        player: &AudioPlayer,
        is_locked: bool,
        display_theme: DisplayTheme,
    ) {
        Self::draw_status_bar_custom(painter, screen_rect, title, player, is_locked, display_theme, None, false);
    }

    pub fn draw_status_bar_with_update(
        painter: &Painter,
        screen_rect: Rect,
        title: &str,
        player: &AudioPlayer,
        is_locked: bool,
        display_theme: DisplayTheme,
        update_available: bool,
    ) {
        Self::draw_status_bar_custom(painter, screen_rect, title, player, is_locked, display_theme, None, update_available);
    }

    pub fn draw_status_bar_custom(
        painter: &Painter,
        screen_rect: Rect,
        title: &str,
        player: &AudioPlayer,
        is_locked: bool,
        display_theme: DisplayTheme,
        custom: Option<&CustomThemeConfig>,
        update_available: bool,
    ) {
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let bar_h = 20.0_f32 * scale;
        let bar_rect = Rect::from_min_size(screen_rect.min, Vec2::new(screen_rect.width(), bar_h));
        let mid_y = bar_rect.min.y + bar_h * 0.5_f32;

        let is_status_dark = if custom.is_some() {
            false
        } else {
            display_theme.is_dark()
        };

        if let Some(cfg) = custom {
            let bg = cfg.get_color32(ColorTarget::StatusBarBg);
            painter.rect_filled(bar_rect, 0.0_f32, bg);
            painter.line_segment(
                [
                    Pos2::new(bar_rect.min.x, bar_rect.max.y),
                    Pos2::new(bar_rect.max.x, bar_rect.max.y),
                ],
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_black_alpha(40)),
            );
        } else {
            match display_theme {
                DisplayTheme::AlbumCover => {
                    // Frosted dark glass gradient (idle grey matrix + active artwork both dark)
                    painter.rect_filled(
                        Rect::from_min_max(bar_rect.min, Pos2::new(bar_rect.max.x, mid_y)),
                        0.0_f32,
                        Color32::from_black_alpha(170),
                    );
                    painter.rect_filled(
                        Rect::from_min_max(Pos2::new(bar_rect.min.x, mid_y), bar_rect.max),
                        0.0_f32,
                        Color32::from_black_alpha(210),
                    );
                    painter.line_segment(
                        [
                            Pos2::new(bar_rect.min.x, bar_rect.max.y),
                            Pos2::new(bar_rect.max.x, bar_rect.max.y),
                        ],
                        Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_white_alpha(35)),
                    );
                }
                DisplayTheme::NightmodeBlack => {
                    // OLED pitch dark status bar
                    painter.rect_filled(
                        Rect::from_min_max(bar_rect.min, Pos2::new(bar_rect.max.x, mid_y)),
                        0.0_f32,
                        Color32::from_rgb(22, 22, 26),
                    );
                    painter.rect_filled(
                        Rect::from_min_max(Pos2::new(bar_rect.min.x, mid_y), bar_rect.max),
                        0.0_f32,
                        Color32::from_rgb(14, 14, 17),
                    );
                    painter.line_segment(
                        [
                            Pos2::new(bar_rect.min.x, bar_rect.max.y),
                            Pos2::new(bar_rect.max.x, bar_rect.max.y),
                        ],
                        Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_rgb(45, 45, 52)),
                    );
                }
                DisplayTheme::DarkGrey => {
                    // Dark slate charcoal status bar
                    painter.rect_filled(
                        Rect::from_min_max(bar_rect.min, Pos2::new(bar_rect.max.x, mid_y)),
                        0.0_f32,
                        Color32::from_rgb(48, 51, 58),
                    );
                    painter.rect_filled(
                        Rect::from_min_max(Pos2::new(bar_rect.min.x, mid_y), bar_rect.max),
                        0.0_f32,
                        Color32::from_rgb(36, 38, 44),
                    );
                    painter.line_segment(
                        [
                            Pos2::new(bar_rect.min.x, bar_rect.max.y),
                            Pos2::new(bar_rect.max.x, bar_rect.max.y),
                        ],
                        Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_rgb(60, 64, 72)),
                    );
                }
                DisplayTheme::Default | DisplayTheme::Unknown => {
                    // Status bar gradient
                    painter.rect_filled(
                        Rect::from_min_max(bar_rect.min, Pos2::new(bar_rect.max.x, mid_y)),
                        0.0_f32,
                        LcdPalette::STATUS_TOP,
                    );
                    painter.rect_filled(
                        Rect::from_min_max(Pos2::new(bar_rect.min.x, mid_y), bar_rect.max),
                        0.0_f32,
                        LcdPalette::STATUS_BOTTOM,
                    );
                    painter.line_segment(
                        [
                            Pos2::new(bar_rect.min.x, bar_rect.max.y),
                            Pos2::new(bar_rect.max.x, bar_rect.max.y),
                        ],
                        Stroke::new((1.0_f32 * scale).max(0.5_f32), LcdPalette::STATUS_BORDER),
                    );
                }
            }
        }

        let text_color = if let Some(cfg) = custom {
            cfg.get_color32(ColorTarget::StatusBarText)
        } else if is_status_dark {
            LcdPalette::ALBUM_COVER_TEXT_PRIMARY
        } else {
            LcdPalette::STATUS_TEXT
        };

        // Left: Play / Pause / Hold Lock Icon
        let icon_pos = Pos2::new(bar_rect.min.x + 8.0_f32 * scale, bar_rect.center().y);
        if is_locked {
            painter.text(
                icon_pos,
                egui::Align2::LEFT_CENTER,
                "🔒",
                FontId::proportional(10.0_f32 * scale),
                text_color,
            );
        } else if player.is_playing {
            painter.text(
                icon_pos,
                egui::Align2::LEFT_CENTER,
                "▶",
                FontId::proportional(9.0_f32 * scale),
                text_color,
            );
        } else if player.current_time_sec > 0.0_f32 {
            painter.text(
                icon_pos,
                egui::Align2::LEFT_CENTER,
                "❙❙",
                FontId::proportional(8.0_f32 * scale),
                text_color,
            );
        }

        // Center: Title
        painter.text(
            bar_rect.center(),
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(11.5_f32 * scale),
            text_color,
        );

        // Right: Battery Icon
        let bat_w = 20.0_f32 * scale;
        let bat_h = 9.0_f32 * scale;
        let bat_x = bar_rect.max.x - bat_w - 8.0_f32 * scale;
        let bat_y = bar_rect.center().y - bat_h * 0.5_f32;
        let bat_rect = Rect::from_min_size(Pos2::new(bat_x, bat_y), Vec2::new(bat_w, bat_h));

        let shell_color = if let Some(cfg) = custom {
            cfg.get_color32(ColorTarget::BatteryFill)
        } else if is_status_dark {
            LcdPalette::ALBUM_COVER_TEXT_SECONDARY
        } else {
            LcdPalette::BATTERY_SHELL
        };

        // Battery outer shell
        painter.rect_stroke(bat_rect, 1.5_f32 * scale, Stroke::new((1.0_f32 * scale).max(0.5_f32), shell_color));
        // Battery terminal pip
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(bat_rect.max.x + 1.0_f32 * scale, bat_rect.center().y - 2.5_f32 * scale),
                Vec2::new(1.5_f32 * scale, 5.0_f32 * scale),
            ),
            0.5_f32 * scale,
            shell_color,
        );
        // Shuffle status badge: 🔀 appears left of the repeat badge when shuffle is on.
        if player.shuffle != crate::audio::player::ShuffleMode::Off {
            let shuffle_x = bat_rect.min.x - (if player.repeat != RepeatMode::Off { 26.0_f32 } else { 13.0_f32 }) * scale;
            painter.text(
                Pos2::new(shuffle_x, bar_rect.center().y),
                egui::Align2::CENTER_CENTER,
                "🔀",
                FontId::proportional(8.5_f32 * scale),
                text_color,
            );
        }

        // Repeat status badge: ↻ for queue/playlist, with a superscript 1 for current song.
        if player.repeat != RepeatMode::Off {
            let repeat_x = bat_rect.min.x - 13.0_f32 * scale;
            painter.text(
                Pos2::new(repeat_x, bar_rect.center().y),
                egui::Align2::CENTER_CENTER,
                "↻",
                FontId::proportional(10.5_f32 * scale),
                text_color,
            );
            if player.repeat == RepeatMode::One {
                painter.text(
                    Pos2::new(repeat_x + 5.0_f32 * scale, bar_rect.center().y - 4.5_f32 * scale),
                    egui::Align2::LEFT_CENTER,
                    "1",
                    FontId::proportional(6.5_f32 * scale),
                    text_color,
                );
            }
        }

        // Battery green charge fill (approx 85%)
        let fill_w = (bat_w - 3.0_f32 * scale) * 0.85_f32;
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(bat_rect.min.x + 1.5_f32 * scale, bat_rect.min.y + 1.5_f32 * scale),
                Vec2::new(fill_w, bat_h - 3.0_f32 * scale),
            ),
            1.0_f32 * scale,
            LcdPalette::BATTERY_FILL,
        );

        // Subtle glowing ★ Update pill badge when a new release is available
        if update_available {
            let badge_w = 46.0_f32 * scale;
            let badge_h = 13.0_f32 * scale;
            let mut right_x = bat_rect.min.x - 6.0_f32 * scale;
            if player.shuffle != crate::audio::player::ShuffleMode::Off {
                right_x -= (if player.repeat != RepeatMode::Off { 26.0_f32 } else { 13.0_f32 }) * scale;
            } else if player.repeat != RepeatMode::Off {
                right_x -= 13.0_f32 * scale;
            }
            let badge_rect = Rect::from_min_size(
                Pos2::new(right_x - badge_w, bar_rect.center().y - badge_h * 0.5_f32),
                Vec2::new(badge_w, badge_h),
            );
            painter.rect_filled(badge_rect, 3.0_f32 * scale, Color32::from_rgb(40, 130, 240));
            painter.rect_stroke(badge_rect, 3.0_f32 * scale, Stroke::new(0.6_f32 * scale, Color32::from_white_alpha(140)));
            painter.text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                "★ Update",
                FontId::proportional(7.5_f32 * scale),
                Color32::WHITE,
            );
        }
    }

    pub fn draw_list_item(
        painter: &Painter,
        rect: Rect,
        title: &str,
        detail: Option<&str>,
        has_arrow: bool,
        is_selected: bool,
        is_dark: bool,
    ) {
        Self::draw_list_item_custom(painter, rect, title, detail, has_arrow, is_selected, is_dark, None);
    }

    pub fn draw_list_item_custom(
        painter: &Painter,
        rect: Rect,
        title: &str,
        detail: Option<&str>,
        has_arrow: bool,
        is_selected: bool,
        is_dark: bool,
        custom: Option<&CustomThemeConfig>,
    ) {
        let scale = (rect.height() / 22.0_f32).max(0.2_f32);

        if is_selected {
            let sel_col = custom
                .map(|c| c.get_color32(ColorTarget::SelectionHighlight))
                .unwrap_or(LcdPalette::SEL_TOP);
            let mid_y = rect.min.y + rect.height() * 0.48_f32;
            painter.rect_filled(
                Rect::from_min_max(rect.min, Pos2::new(rect.max.x, mid_y)),
                0.0_f32,
                sel_col,
            );
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(rect.min.x, mid_y), rect.max),
                0.0_f32,
                custom.map(|_| sel_col).unwrap_or(LcdPalette::SEL_BOTTOM),
            );
            // Top/Bottom highlight lines
            painter.line_segment(
                [rect.min, Pos2::new(rect.max.x, rect.min.y)],
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_white_alpha(70)),
            );
            painter.line_segment(
                [Pos2::new(rect.min.x, rect.max.y), rect.max],
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_black_alpha(60)),
            );
        } else {
            let sep_color = if is_dark {
                Color32::from_white_alpha(30)
            } else {
                Color32::from_rgb(235, 238, 242)
            };
            painter.line_segment(
                [Pos2::new(rect.min.x, rect.max.y), rect.max],
                Stroke::new((1.0_f32 * scale).max(0.5_f32), sep_color),
            );
        }

        let text_color = if is_selected {
            custom
                .map(|c| c.get_color32(ColorTarget::SelectedText))
                .unwrap_or(LcdPalette::TEXT_SELECTED)
        } else if let Some(cfg) = custom {
            cfg.get_color32(ColorTarget::TitleText)
        } else if is_dark {
            LcdPalette::ALBUM_COVER_TEXT_PRIMARY
        } else {
            LcdPalette::TEXT_BLACK
        };

        let title_pos = Pos2::new(rect.min.x + 8.0_f32 * scale, rect.center().y);
        let right_margin = if detail.is_some() && has_arrow {
            (rect.width() * 0.45_f32).max(70.0_f32 * scale)
        } else if detail.is_some() {
            (rect.width() * 0.45_f32).max(60.0_f32 * scale)
        } else if has_arrow {
            20.0_f32 * scale
        } else {
            10.0_f32 * scale
        };

        let title_clip_rect = Rect::from_min_max(
            Pos2::new(rect.min.x, rect.min.y),
            Pos2::new(rect.max.x - right_margin, rect.max.y),
        );
        let title_painter = painter.with_clip_rect(title_clip_rect);
        title_painter.text(
            title_pos,
            egui::Align2::LEFT_CENTER,
            title,
            FontId::proportional(12.0_f32 * scale),
            text_color,
        );

        if let Some(det) = detail {
            let det_pos = Pos2::new(rect.max.x - if has_arrow { 22.0_f32 * scale } else { 8.0_f32 * scale }, rect.center().y);
            let det_color = if is_selected {
                custom
                    .map(|c| c.get_color32(ColorTarget::SelectedText))
                    .unwrap_or(Color32::from_rgb(220, 235, 255))
            } else if let Some(cfg) = custom {
                cfg.get_color32(ColorTarget::ArtistText)
            } else if is_dark {
                LcdPalette::ALBUM_COVER_TEXT_SECONDARY
            } else {
                LcdPalette::TEXT_GRAY
            };
            let det_clip_rect = Rect::from_min_max(
                Pos2::new(rect.max.x - right_margin + 6.0_f32 * scale, rect.min.y),
                Pos2::new(rect.max.x - if has_arrow { 18.0_f32 * scale } else { 4.0_f32 * scale }, rect.max.y),
            );
            let det_painter = painter.with_clip_rect(det_clip_rect);
            det_painter.text(
                det_pos,
                egui::Align2::RIGHT_CENTER,
                det,
                FontId::proportional(11.0_f32 * scale),
                det_color,
            );
        }

        if has_arrow {
            let arrow_pos = Pos2::new(rect.max.x - 8.0_f32 * scale, rect.center().y);
            let arrow_color = if is_selected {
                Color32::WHITE
            } else if let Some(cfg) = custom {
                cfg.get_color32(ColorTarget::ArrowIndicator)
            } else if is_dark {
                LcdPalette::ALBUM_COVER_TEXT_SECONDARY
            } else {
                Color32::from_rgb(140, 145, 155)
            };
            painter.text(
                arrow_pos,
                egui::Align2::RIGHT_CENTER,
                ">",
                FontId::proportional(12.5_f32 * scale),
                arrow_color,
            );
        }
    }

    pub fn draw_split_view_divider(painter: &Painter, split_x: f32, top_y: f32, bottom_y: f32, is_dark: bool) {
        let div_color = if is_dark {
            Color32::from_white_alpha(45)
        } else {
            LcdPalette::BORDER_GRAY
        };
        painter.line_segment(
            [Pos2::new(split_x, top_y), Pos2::new(split_x, bottom_y)],
            Stroke::new(1.0_f32, div_color),
        );
    }

    pub fn draw_volume_hud(painter: &Painter, screen_rect: Rect, volume: f32) {
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let hud_w = 175.0_f32 * scale;
        let hud_h = 56.0_f32 * scale;
        let hud_rect = Rect::from_center_size(screen_rect.center(), Vec2::new(hud_w, hud_h));

        // Drop shadow
        painter.rect_filled(hud_rect.expand(2.0_f32 * scale), 6.0_f32 * scale, Color32::from_black_alpha(40));
        // White dialog background
        painter.rect_filled(hud_rect, 6.0_f32 * scale, Color32::from_rgb(248, 250, 252));
        painter.rect_stroke(hud_rect, 6.0_f32 * scale, Stroke::new(1.5_f32 * scale, Color32::from_rgb(180, 185, 195)));

        // Speaker icon
        let icon = if volume <= 0.001 { "🔇" } else { "🔊" };
        painter.text(
            Pos2::new(hud_rect.min.x + 18.0_f32 * scale, hud_rect.center().y),
            egui::Align2::CENTER_CENTER,
            icon,
            FontId::proportional(16.0_f32 * scale),
            Color32::from_rgb(60, 65, 75),
        );

        // 16-Segment Volume Bar
        let bar_x = hud_rect.min.x + 36.0_f32 * scale;
        let bar_y = hud_rect.center().y - 6.0_f32 * scale;
        let num_segments = 16;
        let active_segments = (volume * num_segments as f32).round() as usize;

        for i in 0..num_segments {
            let seg_rect = Rect::from_min_size(
                Pos2::new(bar_x + (i as f32) * 8.3_f32 * scale, bar_y),
                Vec2::new(7.0_f32 * scale, 12.0_f32 * scale),
            );
            if i < active_segments {
                painter.rect_filled(seg_rect, 1.5_f32 * scale, Color32::from_rgb(30, 110, 230));
            } else {
                painter.rect_filled(seg_rect, 1.5_f32 * scale, Color32::from_rgb(215, 220, 228));
            }
        }
    }

    pub fn draw_alphabet_hud(painter: &Painter, screen_rect: Rect, letter: char) {
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let hud_size = 54.0_f32 * scale;
        let hud_rect = Rect::from_center_size(screen_rect.center(), Vec2::splat(hud_size));

        // Semi-transparent dark rounded badge
        painter.rect_filled(hud_rect, 10.0_f32 * scale, Color32::from_black_alpha(190));
        painter.rect_stroke(hud_rect, 10.0_f32 * scale, Stroke::new(1.5_f32 * scale, Color32::from_white_alpha(90)));

        painter.text(
            hud_rect.center(),
            egui::Align2::CENTER_CENTER,
            letter.to_string(),
            FontId::proportional(28.0_f32 * scale),
            Color32::WHITE,
        );
    }

    /// Pixel Art Theme: retro LCD pixel grid / scanline overlay drawn on top
    /// of whatever view is active. Chunky 3px cells with soft darkening so it
    /// reads as a low-res matrix without hurting legibility.
    pub fn draw_pixel_overlay(painter: &Painter, rect: Rect) {
        let scan_col = Color32::from_black_alpha(13);
        let grid_col = Color32::from_black_alpha(7);

        let mut y = rect.min.y + 3.0_f32;
        while y < rect.max.y {
            let yy = y.round();
            painter.line_segment(
                [Pos2::new(rect.min.x, yy), Pos2::new(rect.max.x, yy)],
                Stroke::new(1.0_f32, scan_col),
            );
            y += 3.0_f32;
        }

        let mut x = rect.min.x + 3.0_f32;
        while x < rect.max.x {
            let xx = x.round();
            painter.line_segment(
                [Pos2::new(xx, rect.min.y), Pos2::new(xx, rect.max.y)],
                Stroke::new(1.0_f32, grid_col),
            );
            x += 3.0_f32;
        }
    }

    pub fn apply_backlight_and_brightness(
        painter: &Painter,
        rect: Rect,
        state: &AppState,
    ) {
        // Brightness adjustment
        let bright_factor = state.brightness_percent as f32 / 100.0_f32;
        if bright_factor < 1.0_f32 {
            let dim_alpha = ((1.0_f32 - bright_factor) * 200.0_f32) as u8;
            painter.rect_filled(rect, 0.0_f32, Color32::from_black_alpha(dim_alpha));
        }

        // Backlight timeout dimming
        if state.backlight_remaining_sec <= 0.0_f32 && state.backlight_timer != crate::state::BacklightDuration::AlwaysOn {
            painter.rect_filled(rect, 0.0_f32, Color32::from_black_alpha(160));
        }
    }

    pub fn render_scrolling_text(
        painter: &Painter,
        ctx: &Context,
        text: &str,
        font: FontId,
        color: Color32,
        bounds: Rect,
        time: f64,
        align: egui::Align2,
    ) {
        let text_w = ctx.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), color).size().x);
        let avail_w = bounds.width();

        if text_w <= avail_w {
            let anchor = match align {
                egui::Align2::LEFT_CENTER | egui::Align2::LEFT_TOP | egui::Align2::LEFT_BOTTOM => {
                    Pos2::new(bounds.min.x, bounds.center().y)
                }
                egui::Align2::CENTER_CENTER | egui::Align2::CENTER_TOP | egui::Align2::CENTER_BOTTOM => {
                    bounds.center()
                }
                egui::Align2::RIGHT_CENTER | egui::Align2::RIGHT_TOP | egui::Align2::RIGHT_BOTTOM => {
                    Pos2::new(bounds.max.x, bounds.center().y)
                }
            };
            painter.text(anchor, align, text, font, color);
        } else {
            // NOTE: Repainting is driven by the adaptive scheduler in ui/mod.rs
            // (25 FPS while a marquee is active) — no per-frame request here.
            let overflow = text_w - avail_w;
            let cycle_duration = 7.0_f64;
            let t = (time % cycle_duration) as f32;

            let scroll_offset = if t < 1.5 {
                0.0
            } else if t < 5.0 {
                let p = (t - 1.5) / 3.5;
                p * overflow
            } else {
                overflow
            };

            let clipped_painter = painter.with_clip_rect(bounds);
            clipped_painter.text(
                Pos2::new(bounds.min.x - scroll_offset, bounds.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                font,
                color,
            );
        }
    }

    /// Parse and split artist string into individual artist names.
    /// Handles multiple delimiters: comma, semicolon, slash, feat., ft., featuring, &, with, x, vs.
    pub fn split_artists(raw: &str) -> Vec<String> {
        let mut cleaned = raw.to_string();

        let placeholder_tyler = "__TYLER_THE_CREATOR__";
        let placeholder_ewf = "__EARTH_WIND_FIRE__";
        let placeholder_csny = "__CROSBY_STILLS_NASH_YOUNG__";
        let placeholder_csn = "__CROSBY_STILLS_NASH__";
        let placeholder_elp = "__EMERSON_LAKE_PALMER__";
        let placeholder_bst = "__BLOOD_SWEAT_TEARS__";

        cleaned = cleaned.replace("Tyler, The Creator", placeholder_tyler);
        cleaned = cleaned.replace("tyler, the creator", placeholder_tyler);
        cleaned = cleaned.replace("Tyler, the Creator", placeholder_tyler);
        cleaned = cleaned.replace("Earth, Wind & Fire", placeholder_ewf);
        cleaned = cleaned.replace("earth, wind & fire", placeholder_ewf);
        cleaned = cleaned.replace("Earth, Wind and Fire", placeholder_ewf);
        cleaned = cleaned.replace("Crosby, Stills, Nash & Young", placeholder_csny);
        cleaned = cleaned.replace("Crosby, Stills & Nash", placeholder_csn);
        cleaned = cleaned.replace("Emerson, Lake & Palmer", placeholder_elp);
        cleaned = cleaned.replace("Blood, Sweat & Tears", placeholder_bst);

        let patterns = [
            " (feat. ", " (feat ", " (ft. ", " (ft ", " (featuring ", " (with ", " (prod. ", " (Prod. ",
            " [feat. ", " [feat ", " [ft. ", " [ft ", " [featuring ", " [with ", " [prod. ", " [Prod. ",
            " feat. ", " feat ", " ft. ", " ft ", " featuring ", " with ", " prod. ",
            " Feat. ", " Feat ", " Ft. ", " Ft ", " Featuring ", " With ", " Prod. ",
            " / ", " // ", " \\ ", "; ", ";", " x ", " X ", " vs. ", " vs ", " VS. ", " VS "
        ];

        for pat in patterns {
            cleaned = cleaned.replace(pat, "|");
        }

        cleaned = cleaned.replace(')', "").replace(']', "");

        let mut parts: Vec<String> = Vec::new();
        for section in cleaned.split('|') {
            for sub in section.split(',') {
                for sub_and in sub.split(" & ") {
                    for sub_plus in sub_and.split(" + ") {
                        let trimmed = sub_plus.trim();
                        if !trimmed.is_empty() {
                            let restored = trimmed
                                .replace(placeholder_tyler, "Tyler, The Creator")
                                .replace(placeholder_ewf, "Earth, Wind & Fire")
                                .replace(placeholder_csny, "Crosby, Stills, Nash & Young")
                                .replace(placeholder_csn, "Crosby, Stills & Nash")
                                .replace(placeholder_elp, "Emerson, Lake & Palmer")
                                .replace(placeholder_bst, "Blood, Sweat & Tears");
                            parts.push(restored);
                        }
                    }
                }
            }
        }

        // Deduplicate while preserving order
        let mut unique: Vec<String> = Vec::new();
        for p in parts {
            if !unique.iter().any(|x| x.eq_ignore_ascii_case(&p)) {
                unique.push(p);
            }
        }

        if unique.is_empty() && !raw.trim().is_empty() {
            unique.push(raw.trim().to_string());
        }

        unique
    }

    /// If a song has multiple artists (2 or more), rotate through the artists every 3.0 seconds
    /// to keep UI compact, clean, readable, and prevent cluttered multi-artist text.
    pub fn get_display_artist(raw: &str, time: f64) -> String {
        let artists = Self::split_artists(raw);
        if artists.len() > 1 {
            let cycle_duration = 3.0_f64;
            let idx = ((time / cycle_duration).floor() as usize) % artists.len();
            artists[idx].clone()
        } else {
            raw.to_string()
        }
    }
}
