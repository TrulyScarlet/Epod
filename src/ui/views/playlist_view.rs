use crate::audio::player::AudioPlayer;
use crate::library::model::Library;
use crate::state::{AppState, PlaylistViewMode};
use crate::theme::LcdPalette;
use crate::ui::art_cache::ArtCache;
use crate::ui::lcd::LcdRenderer;
use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub struct PlaylistView;

impl PlaylistView {
    /// Format seconds to mm:ss or "X min" / "X hr Y min"
    pub fn format_duration(sec: f32) -> String {
        let total_sec = sec.max(0.0) as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        format!("{}:{:02}", mins, secs)
    }

    pub fn format_total_duration(sec: f32) -> String {
        let total_sec = sec.max(0.0) as u32;
        let mins = total_sec / 60;
        let hrs = mins / 60;
        let rem_mins = mins % 60;
        if hrs > 0 {
            format!("{} hr {} min", hrs, rem_mins)
        } else {
            format!("{} min", mins)
        }
    }

    /// Filter library songs by title or artist for the Add Songs screen
    pub fn get_filtered_songs_for_add(library: &Library, query: &str) -> Vec<usize> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            library.sorted_song_indices.clone()
        } else {
            library
                .sorted_song_indices
                .iter()
                .cloned()
                .filter(|&song_id| {
                    if let Some(s) = library.songs.get(song_id) {
                        s.title.to_lowercase().contains(&q) || s.artist.to_lowercase().contains(&q) || s.album.to_lowercase().contains(&q)
                    } else {
                        false
                    }
                })
                .collect()
        }
    }

    /// Render Playlist List Hub
    pub fn render_playlists_hub(
        painter: &Painter,
        ctx: &egui::Context,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
        art_cache: &mut ArtCache,
    ) {
        match state.playlist_view_mode {
            PlaylistViewMode::Default => {
                Self::render_playlists_hub_default(painter, screen_rect, state, library, player);
            }
            PlaylistViewMode::Modern => {
                Self::render_playlists_hub_modern(painter, ctx, screen_rect, state, library, player, art_cache);
            }
        }
    }

    /// Classic iPod Playlists List View
    fn render_playlists_hub_default(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Playlists", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let total_items = 1 + library.playlists.len();
        let sel = state.get_selected_index("playlists_list").min(total_items.saturating_sub(1));
        let item_h = 22.0_f32 * scale;
        let visible_count = ((content_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if sel >= visible_count {
            sel - visible_count + 1
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
            let is_selected = item_idx == sel;

            if item_idx == 0 {
                LcdRenderer::draw_list_item(
                    painter,
                    item_rect,
                    "[+] Create Playlist...",
                    None,
                    true,
                    is_selected,
                    is_dark,
                );
            } else {
                let pl_idx = item_idx - 1;
                if let Some(pl) = library.playlists.get(pl_idx) {
                    let count_str = format!("{} songs", pl.song_ids.len().max(pl.song_entries.len()));
                    LcdRenderer::draw_list_item(
                        painter,
                        item_rect,
                        &pl.name,
                        Some(&count_str),
                        true,
                        is_selected,
                        is_dark,
                    );
                }
            }
        }

        if total_items > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (content_rect.height() * (visible_count as f32 / total_items as f32)).max(12.0_f32 * scale);
            let thumb_y = content_rect.min.y + (content_rect.height() - thumb_h) * (scroll_offset as f32 / (total_items - visible_count) as f32);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)),
                2.0_f32 * scale,
                Color32::from_rgb(180, 185, 195),
            );
        }
    }

    /// Modern Spotify-Style Playlists List View
    fn render_playlists_hub_modern(
        painter: &Painter,
        ctx: &egui::Context,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
        art_cache: &mut ArtCache,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Playlists", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let total_items = 1 + library.playlists.len();
        let sel = state.get_selected_index("playlists_list").min(total_items.saturating_sub(1));
        let item_h = 36.0_f32 * scale;
        let visible_count = ((content_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if sel >= visible_count {
            sel - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let item_idx = scroll_offset + i;
            if item_idx >= total_items {
                break;
            }

            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x + 4.0_f32 * scale, content_rect.min.y + (i as f32) * item_h + 2.0_f32 * scale),
                Vec2::new(content_rect.width() - 8.0_f32 * scale, item_h - 4.0_f32 * scale),
            );
            let is_selected = item_idx == sel;

            if is_selected {
                let border_col = Color32::from_rgb(40, 130, 240);
                painter.rect_stroke(item_rect, 5.0_f32 * scale, Stroke::new(1.2_f32 * scale, border_col));
            }

            if item_idx == 0 {
                // Modern "Create New Playlist" clean text item
                let text_x = item_rect.min.x + 10.0_f32 * scale;
                painter.text(
                    Pos2::new(text_x, item_rect.center().y - 6.0_f32 * scale),
                    egui::Align2::LEFT_CENTER,
                    "[+] Create New Playlist",
                    FontId::proportional(11.5_f32 * scale),
                    if is_selected { Color32::from_rgb(40, 130, 240) } else { LcdPalette::text_primary(is_dark) },
                );

                painter.text(
                    Pos2::new(text_x, item_rect.center().y + 6.0_f32 * scale),
                    egui::Align2::LEFT_CENTER,
                    "Tap to make your custom playlist",
                    FontId::proportional(9.0_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );

                painter.text(
                    Pos2::new(item_rect.max.x - 8.0_f32 * scale, item_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    ">",
                    FontId::proportional(12.0_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );
            } else {
                let pl_idx = item_idx - 1;
                if let Some(pl) = library.playlists.get(pl_idx) {
                    let icon_size = 28.0_f32 * scale;
                    let icon_rect = Rect::from_min_size(
                        Pos2::new(item_rect.min.x + 4.0_f32 * scale, item_rect.center().y - icon_size * 0.5),
                        Vec2::splat(icon_size),
                    );

                    // Fetch playlist cover or first song cover
                    let first_song = pl.song_ids.first().and_then(|&id| library.songs.get(id));
                    let first_song_bytes = first_song.and_then(|s| s.artwork_bytes.as_deref());
                    let first_song_path = first_song.and_then(|s| s.file_path.as_deref());
                    let cover_tex = art_cache.get_or_load_playlist_cover(
                        ctx,
                        &pl.id,
                        pl.custom_cover_path.as_deref(),
                        first_song_bytes,
                        first_song_path,
                    );

                    if let Some(tex) = cover_tex {
                        painter.image(
                            tex.id(),
                            icon_rect,
                            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        painter.rect_stroke(icon_rect, 3.0_f32 * scale, Stroke::new(0.5_f32 * scale, Color32::from_black_alpha(60)));
                    } else {
                        painter.rect_filled(icon_rect, 3.0_f32 * scale, Color32::from_rgb(30, 80, 140));
                        painter.text(
                            icon_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "🎵",
                            FontId::proportional(11.0_f32 * scale),
                            Color32::WHITE,
                        );
                    }

                    let text_x = icon_rect.max.x + 8.0_f32 * scale;
                    painter.text(
                        Pos2::new(text_x, item_rect.center().y - 6.0_f32 * scale),
                        egui::Align2::LEFT_CENTER,
                        &pl.name,
                        FontId::proportional(11.0_f32 * scale),
                        LcdPalette::text_primary(is_dark),
                    );

                    let subtitle = format!("Playlist • {} songs", pl.song_ids.len().max(pl.song_entries.len()));

                    painter.text(
                        Pos2::new(text_x, item_rect.center().y + 6.0_f32 * scale),
                        egui::Align2::LEFT_CENTER,
                        &subtitle,
                        FontId::proportional(9.0_f32 * scale),
                        LcdPalette::text_secondary(is_dark),
                    );

                    painter.text(
                        Pos2::new(item_rect.max.x - 8.0_f32 * scale, item_rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        ">",
                        FontId::proportional(12.0_f32 * scale),
                        LcdPalette::text_secondary(is_dark),
                    );
                }
            }
        }

        if total_items > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (content_rect.height() * (visible_count as f32 / total_items as f32)).max(12.0_f32 * scale);
            let thumb_y = content_rect.min.y + (content_rect.height() - thumb_h) * (scroll_offset as f32 / (total_items - visible_count) as f32);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)),
                2.0_f32 * scale,
                Color32::from_rgb(180, 185, 195),
            );
        }
    }

    /// Render Playlist Detail (Classic iPod list or Modern Spotify layout)
    pub fn render_playlist_detail(
        painter: &Painter,
        ctx: &egui::Context,
        screen_rect: Rect,
        playlist_idx: usize,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
        art_cache: &mut ArtCache,
    ) {
        let playlist = match library.playlists.get(playlist_idx) {
            Some(pl) => pl,
            None => return,
        };

        match state.playlist_view_mode {
            PlaylistViewMode::Default => {
                Self::render_playlist_detail_default(painter, screen_rect, playlist_idx, playlist, state, library, player);
            }
            PlaylistViewMode::Modern => {
                Self::render_playlist_detail_modern(painter, ctx, screen_rect, playlist_idx, playlist, state, library, player, art_cache);
            }
        }
    }

    /// Default iPod-style playlist detail view
    fn render_playlist_detail_default(
        painter: &Painter,
        screen_rect: Rect,
        _playlist_idx: usize,
        playlist: &crate::library::model::Playlist,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, &playlist.name, player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        // Header Actions:
        // Index 0: [+] Add Songs...
        // Index 1: [✎] Rename Playlist...
        // Index 2: [🗑] Delete Playlist...
        // Index 3..: Songs in playlist
        let header_actions_count = 3;

        let total_items = header_actions_count + playlist.song_ids.len();
        let sel = state.get_selected_index("playlist_detail").min(total_items.saturating_sub(1));
        let item_h = 22.0_f32 * scale;
        let visible_count = ((content_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if sel >= visible_count {
            sel - visible_count + 1
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
            let is_selected = item_idx == sel;

            if item_idx == 0 {
                LcdRenderer::draw_list_item(
                    painter,
                    item_rect,
                    "[+] Add Songs...",
                    None,
                    true,
                    is_selected,
                    is_dark,
                );
            } else if item_idx == 1 {
                LcdRenderer::draw_list_item(
                    painter,
                    item_rect,
                    "[✎] Rename Playlist...",
                    None,
                    true,
                    is_selected,
                    is_dark,
                );
            } else if item_idx == 2 {
                LcdRenderer::draw_list_item(
                    painter,
                    item_rect,
                    "[🗑] Delete Playlist",
                    None,
                    false,
                    is_selected,
                    is_dark,
                );
            } else {
                let song_pos = item_idx - header_actions_count;
                if let Some(&song_id) = playlist.song_ids.get(song_pos) {
                    if let Some(song) = library.songs.get(song_id) {
                        let dur = Self::format_duration(song.duration_sec);
                        LcdRenderer::draw_list_item(
                            painter,
                            item_rect,
                            &song.title,
                            Some(&dur),
                            false,
                            is_selected,
                            is_dark,
                        );
                    }
                }
            }
        }

        if total_items > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (content_rect.height() * (visible_count as f32 / total_items as f32)).max(12.0_f32 * scale);
            let thumb_y = content_rect.min.y + (content_rect.height() - thumb_h) * (scroll_offset as f32 / (total_items - visible_count) as f32);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)),
                2.0_f32 * scale,
                Color32::from_rgb(180, 185, 195),
            );
        }
    }

    /// Modern Spotify-Style Playlist Detail View
    fn render_playlist_detail_modern(
        painter: &Painter,
        ctx: &egui::Context,
        screen_rect: Rect,
        _playlist_idx: usize,
        playlist: &crate::library::model::Playlist,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
        art_cache: &mut ArtCache,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, &playlist.name, player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        // 1. Spotify Header Banner Card
        let header_h = 68.0_f32 * scale;
        let header_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x + 6.0_f32 * scale, content_rect.min.y + 4.0_f32 * scale),
            Vec2::new(content_rect.width() - 12.0_f32 * scale, header_h),
        );

        // Large Playlist Cover Art (Square 56x56)
        let cover_size = 56.0_f32 * scale;
        let cover_rect = Rect::from_min_size(
            Pos2::new(header_rect.min.x + 6.0_f32 * scale, header_rect.center().y - cover_size * 0.5),
            Vec2::splat(cover_size),
        );

        let first_song = playlist.song_ids.first().and_then(|&id| library.songs.get(id));
        let first_song_bytes = first_song.and_then(|s| s.artwork_bytes.as_deref());
        let first_song_path = first_song.and_then(|s| s.file_path.as_deref());
        let cover_tex = art_cache.get_or_load_playlist_cover(
            ctx,
            &playlist.id,
            playlist.custom_cover_path.as_deref(),
            first_song_bytes,
            first_song_path,
        );

        if let Some(tex) = cover_tex {
            painter.image(
                tex.id(),
                cover_rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            painter.rect_stroke(cover_rect, 4.0_f32 * scale, Stroke::new(0.6_f32 * scale, Color32::from_white_alpha(50)));
        } else {
            let grad_bg = Color32::from_rgb(30, 215, 96);
            painter.rect_filled(cover_rect, 4.0_f32 * scale, grad_bg);
            painter.text(
                cover_rect.center(),
                egui::Align2::CENTER_CENTER,
                "🎵",
                FontId::proportional(22.0_f32 * scale),
                Color32::BLACK,
            );
        }

        // Header text & action buttons to the right of cover
        let info_x = cover_rect.max.x + 8.0_f32 * scale;
        
        // Small "PLAYLIST" badge
        painter.text(
            Pos2::new(info_x, header_rect.min.y + 7.0_f32 * scale),
            egui::Align2::LEFT_TOP,
            "PLAYLIST",
            FontId::proportional(8.0_f32 * scale),
            Color32::from_rgb(50, 140, 250),
        );

        // Bold Title
        painter.text(
            Pos2::new(info_x, header_rect.min.y + 17.0_f32 * scale),
            egui::Align2::LEFT_TOP,
            &playlist.name,
            FontId::proportional(12.5_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        // Subtitle: "X songs • Y min"
        let total_secs: f32 = playlist.song_ids.iter().filter_map(|&id| library.songs.get(id)).map(|s| s.duration_sec).sum();
        let meta_str = format!("{} songs • {}", playlist.song_ids.len(), Self::format_total_duration(total_secs));
        painter.text(
            Pos2::new(info_x, header_rect.min.y + 32.0_f32 * scale),
            egui::Align2::LEFT_TOP,
            &meta_str,
            FontId::proportional(8.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        // Header Action Pills Row: [▶ Play] [📷] [+] [✎]
        let btn_y = header_rect.min.y + 46.0_f32 * scale;
        let btn_h = 16.0_f32 * scale;
        let pill_idle_bg = if is_dark { Color32::from_white_alpha(28) } else { Color32::from_rgb(222, 228, 238) };
        let accent_blue = Color32::from_rgb(40, 130, 240);

        // 1. Play Button (Blue accent pill)
        let play_btn_w = 42.0_f32 * scale;
        let play_btn_rect = Rect::from_min_size(Pos2::new(info_x, btn_y), Vec2::new(play_btn_w, btn_h));
        painter.rect_filled(play_btn_rect, 3.0_f32 * scale, accent_blue);
        painter.text(
            play_btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "▶ Play",
            FontId::proportional(8.0_f32 * scale),
            Color32::WHITE,
        );

        // 2. Cover Button
        let cover_btn_w = 30.0_f32 * scale;
        let cover_btn_rect = Rect::from_min_size(Pos2::new(play_btn_rect.max.x + 4.0_f32 * scale, btn_y), Vec2::new(cover_btn_w, btn_h));
        painter.rect_filled(cover_btn_rect, 3.0_f32 * scale, pill_idle_bg);
        painter.text(
            cover_btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "📷",
            FontId::proportional(7.5_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        // 3. Add Songs Button
        let add_btn_w = 26.0_f32 * scale;
        let add_btn_rect = Rect::from_min_size(Pos2::new(cover_btn_rect.max.x + 4.0_f32 * scale, btn_y), Vec2::new(add_btn_w, btn_h));
        painter.rect_filled(add_btn_rect, 3.0_f32 * scale, pill_idle_bg);
        painter.text(
            add_btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            FontId::proportional(9.5_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        // 4. Rename / Edit Button
        let edit_btn_w = 26.0_f32 * scale;
        let edit_btn_rect = Rect::from_min_size(Pos2::new(add_btn_rect.max.x + 4.0_f32 * scale, btn_y), Vec2::new(edit_btn_w, btn_h));
        painter.rect_filled(edit_btn_rect, 3.0_f32 * scale, pill_idle_bg);
        painter.text(
            edit_btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✎",
            FontId::proportional(9.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );

        // 2. Spotify Track Table
        let table_top_y = header_rect.max.y + 4.0_f32 * scale;
        let table_header_h = 14.0_f32 * scale;
        let table_header_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x + 8.0_f32 * scale, table_top_y),
            Vec2::new(content_rect.width() - 16.0_f32 * scale, table_header_h),
        );

        painter.text(
            Pos2::new(table_header_rect.min.x + 4.0_f32 * scale, table_header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "#   TITLE",
            FontId::proportional(8.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        painter.text(
            Pos2::new(table_header_rect.max.x - 4.0_f32 * scale, table_header_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "DURATION",
            FontId::proportional(8.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        // Divider hairline
        painter.line_segment(
            [
                Pos2::new(table_header_rect.min.x, table_header_rect.max.y),
                Pos2::new(table_header_rect.max.x, table_header_rect.max.y),
            ],
            Stroke::new(0.5_f32 * scale, Color32::from_white_alpha(25)),
        );

        // Track List Rows
        let list_rect = Rect::from_min_max(
            Pos2::new(content_rect.min.x, table_header_rect.max.y + 2.0_f32 * scale),
            screen_rect.max,
        );

        let total_songs = playlist.song_ids.len();
        if total_songs == 0 {
            let empty_rect = Rect::from_center_size(
                Pos2::new(list_rect.center().x, list_rect.center().y),
                Vec2::new(list_rect.width() - 32.0_f32 * scale, 50.0_f32 * scale),
            );
            painter.text(
                Pos2::new(empty_rect.center().x, empty_rect.center().y - 8.0_f32 * scale),
                egui::Align2::CENTER_CENTER,
                "This playlist is empty.",
                FontId::proportional(11.0_f32 * scale),
                LcdPalette::text_primary(is_dark),
            );
            painter.text(
                Pos2::new(empty_rect.center().x, empty_rect.center().y + 8.0_f32 * scale),
                egui::Align2::CENTER_CENTER,
                "Click '+ Songs' above to add music!",
                FontId::proportional(9.5_f32 * scale),
                Color32::from_rgb(50, 140, 250),
            );
            return;
        }

        let sel = state.get_selected_index("playlist_detail").min(total_songs.saturating_sub(1));
        let row_h = 24.0_f32 * scale;
        let visible_count = ((list_rect.height() / row_h).floor() as usize).max(1);

        let scroll_offset = if sel >= visible_count {
            sel - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let item_idx = scroll_offset + i;
            if item_idx >= total_songs {
                break;
            }

            let song_id = playlist.song_ids[item_idx];
            let song = match library.songs.get(song_id) {
                Some(s) => s,
                None => continue,
            };

            let row_rect = Rect::from_min_size(
                Pos2::new(list_rect.min.x + 4.0_f32 * scale, list_rect.min.y + (i as f32) * row_h),
                Vec2::new(list_rect.width() - 8.0_f32 * scale, row_h),
            );
            let is_selected = item_idx == sel;
            let is_hovered = ctx
                .input(|i| i.pointer.hover_pos())
                .map(|p| row_rect.contains(p))
                .unwrap_or(false);

            if is_selected {
                let border_col = Color32::from_rgb(40, 130, 240);
                painter.rect_stroke(row_rect, 4.0_f32 * scale, Stroke::new(1.2_f32 * scale, border_col));
            } else if is_hovered {
                // Sleek transparent hover backdrop with a soft accent outline
                painter.rect_filled(row_rect, 4.0_f32 * scale, if is_dark { Color32::from_white_alpha(12) } else { Color32::from_black_alpha(8) });
                painter.rect_stroke(row_rect, 4.0_f32 * scale, Stroke::new((0.8_f32 * scale).max(0.5), Color32::from_rgba_premultiplied(40, 130, 240, 110)));
            }

            let is_current_playing = player.is_playing && state.current_queue.get(state.current_queue_idx) == Some(&song_id);
            let num_rect = Rect::from_min_size(
                Pos2::new(row_rect.min.x + 4.0_f32 * scale, row_rect.center().y - 8.0_f32 * scale),
                Vec2::new(14.0_f32 * scale, 16.0_f32 * scale),
            );

            if is_current_playing {
                painter.text(
                    num_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "▶",
                    FontId::proportional(8.5_f32 * scale),
                    Color32::from_rgb(40, 130, 240),
                );
            } else {
                painter.text(
                    num_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &format!("{}", item_idx + 1),
                    FontId::proportional(8.5_f32 * scale),
                    LcdPalette::text_secondary(is_dark),
                );
            }

            // Album art thumbnail
            let thumb_size = 18.0_f32 * scale;
            let thumb_rect = Rect::from_min_size(
                Pos2::new(num_rect.max.x + 4.0_f32 * scale, row_rect.center().y - thumb_size * 0.5),
                Vec2::splat(thumb_size),
            );

            if let Some(tex) = art_cache.get_or_load(ctx, song.id, song.artwork_bytes.as_deref(), song.file_path.as_deref()) {
                painter.image(
                    tex.id(),
                    thumb_rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                painter.rect_stroke(thumb_rect, 2.0_f32 * scale, Stroke::new(0.5_f32 * scale, Color32::from_black_alpha(60)));
            } else {
                painter.rect_filled(thumb_rect, 2.0_f32 * scale, Color32::from_rgb(60, 65, 75));
                painter.text(
                    thumb_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "♪",
                    FontId::proportional(9.0_f32 * scale),
                    Color32::WHITE,
                );
            }

            // Song Title & Artist
            let title_x = thumb_rect.max.x + 6.0_f32 * scale;
            // Saturated peach-gold when hovering the currently-playing track,
            // classic blue while it plays, warm white when selected.
            let title_color = if is_current_playing && is_hovered {
                Color32::from_rgb(255, 158, 56)
            } else if is_current_playing {
                Color32::from_rgb(50, 140, 250)
            } else if is_selected {
                if is_dark { Color32::WHITE } else { Color32::from_rgb(20, 30, 45) }
            } else {
                LcdPalette::text_primary(is_dark)
            };

            let max_text_x = row_rect.max.x - 38.0_f32 * scale;
            let text_clip_rect = Rect::from_min_max(
                Pos2::new(title_x, row_rect.min.y),
                Pos2::new(max_text_x, row_rect.max.y),
            );
            let text_painter = painter.with_clip_rect(text_clip_rect);

            text_painter.text(
                Pos2::new(title_x, row_rect.center().y - 4.5_f32 * scale),
                egui::Align2::LEFT_CENTER,
                &song.title,
                FontId::proportional(9.5_f32 * scale),
                title_color,
            );

            let time = ctx.input(|i| i.time);
            let display_artist = LcdRenderer::get_display_artist(&song.artist, time);
            text_painter.text(
                Pos2::new(title_x, row_rect.center().y + 5.0_f32 * scale),
                egui::Align2::LEFT_CENTER,
                &display_artist,
                FontId::proportional(7.5_f32 * scale),
                LcdPalette::text_secondary(is_dark),
            );

            // Duration on Right
            painter.text(
                Pos2::new(row_rect.max.x - 8.0_f32 * scale, row_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                &Self::format_duration(song.duration_sec),
                FontId::proportional(8.5_f32 * scale),
                LcdPalette::text_secondary(is_dark),
            );
        }

        if total_songs > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = list_rect.max.x - bar_w;
            let thumb_h = (list_rect.height() * (visible_count as f32 / total_songs as f32)).max(12.0_f32 * scale);
            let thumb_y = list_rect.min.y + (list_rect.height() - thumb_h) * (scroll_offset as f32 / (total_songs - visible_count) as f32);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)),
                2.0_f32 * scale,
                Color32::from_rgb(180, 185, 195),
            );
        }
    }

    /// Render Name Input / Rename dialog for Playlists with animated white cursor underscore
    pub fn render_playlist_name_input_screen(
        painter: &Painter,
        screen_rect: Rect,
        is_editing: bool,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let header_title = if is_editing { "Rename Playlist" } else { "New Playlist" };
        LcdRenderer::draw_status_bar(painter, screen_rect, header_title, player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        painter.text(
            Pos2::new(content_rect.center().x, content_rect.min.y + 14.0_f32 * scale),
            egui::Align2::CENTER_TOP,
            "Enter a name for this playlist:",
            FontId::proportional(10.5_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        // Text input display box
        let box_w = content_rect.width() - 36.0_f32 * scale;
        let box_h = 26.0_f32 * scale;
        let box_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x, content_rect.min.y + 48.0_f32 * scale),
            Vec2::new(box_w, box_h),
        );

        let box_bg = if is_dark { Color32::from_rgb(20, 22, 28) } else { Color32::from_rgb(255, 255, 255) };
        painter.rect_filled(box_rect, 4.0_f32 * scale, box_bg);
        painter.rect_stroke(box_rect, 4.0_f32 * scale, Stroke::new(1.5_f32 * scale, Color32::from_rgb(30, 215, 96)));

        let text_col = LcdPalette::text_primary(is_dark);
        let font = FontId::proportional(12.0_f32 * scale);

        let start_x = box_rect.min.x + 10.0_f32 * scale;
        let text_y = box_rect.center().y;

        if state.playlist_name_buffer.is_empty() {
            painter.text(
                Pos2::new(start_x, text_y),
                egui::Align2::LEFT_CENTER,
                "Type playlist name...",
                font.clone(),
                Color32::from_rgb(150, 155, 165),
            );
        } else {
            painter.text(
                Pos2::new(start_x, text_y),
                egui::Align2::LEFT_CENTER,
                &state.playlist_name_buffer,
                font.clone(),
                text_col,
            );
        }

        // Measure text width to position the blinking cursor underscore
        let approx_char_w = 7.0_f32 * scale;
        let text_w = if state.playlist_name_buffer.is_empty() {
            0.0_f32
        } else {
            state.playlist_name_buffer.len() as f32 * approx_char_w
        };
        let cursor_x = start_x + text_w;
        let cursor_y = text_y + 7.0_f32 * scale;

        // Draw solid/blinking white cursor underscore
        painter.line_segment(
            [
                Pos2::new(cursor_x + 1.0_f32 * scale, cursor_y),
                Pos2::new(cursor_x + 9.0_f32 * scale, cursor_y),
            ],
            Stroke::new(2.0_f32 * scale, if is_dark { Color32::WHITE } else { Color32::BLACK }),
        );

        // Quick suggestions
        painter.text(
            Pos2::new(content_rect.min.x + 18.0_f32 * scale, content_rect.min.y + 72.0_f32 * scale),
            egui::Align2::LEFT_TOP,
            "Quick suggestions (click to use):",
            FontId::proportional(9.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );

        let suggestions = ["My Favorites", "Workout Mix", "Chill Beats", "Night Vibes", "Road Trip", "Discover", "Party Jam"];
        let chip_h = 16.0_f32 * scale;
        let mut curr_x = content_rect.min.x + 18.0_f32 * scale;
        let mut curr_y = content_rect.min.y + 90.0_f32 * scale;

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

        // Action Buttons: [ Save / Create ] [ Cancel ]
        let btn_w = 90.0_f32 * scale;
        let btn_h = 22.0_f32 * scale;
        let btn_y = content_rect.max.y - 24.0_f32 * scale;

        let save_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x - 52.0_f32 * scale, btn_y),
            Vec2::new(btn_w, btn_h),
        );
        painter.rect_filled(save_rect, 4.0_f32 * scale, Color32::from_rgb(30, 215, 96));
        painter.text(
            save_rect.center(),
            egui::Align2::CENTER_CENTER,
            if is_editing { "Save Name" } else { "Create Playlist" },
            FontId::proportional(10.0_f32 * scale),
            Color32::BLACK,
        );

        let cancel_rect = Rect::from_center_size(
            Pos2::new(content_rect.center().x + 52.0_f32 * scale, btn_y),
            Vec2::new(btn_w, btn_h),
        );
        let cancel_bg = if is_dark { Color32::from_white_alpha(40) } else { Color32::from_rgb(220, 225, 235) };
        painter.rect_filled(cancel_rect, 4.0_f32 * scale, cancel_bg);
        painter.text(
            cancel_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Cancel",
            FontId::proportional(10.0_f32 * scale),
            LcdPalette::text_primary(is_dark),
        );
    }

    /// Render Add Songs screen with real-time Search Box to easily find song titles or artists
    pub fn render_add_songs_screen(
        painter: &Painter,
        screen_rect: Rect,
        playlist_idx: usize,
        search_query: &str,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
    ) {
        let playlist = match library.playlists.get(playlist_idx) {
            Some(pl) => pl,
            None => return,
        };

        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let status_title = format!("Add to '{}' ({})", playlist.name, playlist.song_ids.len());
        LcdRenderer::draw_status_bar(painter, screen_rect, &status_title, player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        // 1. Search Box Bar
        let search_h = 22.0_f32 * scale;
        let search_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x + 6.0_f32 * scale, content_rect.min.y + 4.0_f32 * scale),
            Vec2::new(content_rect.width() - 12.0_f32 * scale, search_h),
        );

        let search_bg = if is_dark { Color32::from_rgb(22, 25, 30) } else { Color32::from_rgb(240, 243, 250) };
        painter.rect_filled(search_rect, 4.0_f32 * scale, search_bg);
        painter.rect_stroke(search_rect, 4.0_f32 * scale, Stroke::new(1.0_f32 * scale, Color32::from_rgb(30, 215, 96)));

        // Search icon
        painter.text(
            Pos2::new(search_rect.min.x + 6.0_f32 * scale, search_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "🔍",
            FontId::proportional(9.0_f32 * scale),
            Color32::from_rgb(30, 215, 96),
        );

        let text_start_x = search_rect.min.x + 22.0_f32 * scale;
        let text_font = FontId::proportional(10.0_f32 * scale);

        if search_query.is_empty() {
            painter.text(
                Pos2::new(text_start_x, search_rect.center().y),
                egui::Align2::LEFT_CENTER,
                "Type song or artist to filter...",
                text_font.clone(),
                Color32::from_rgb(140, 145, 155),
            );
        } else {
            painter.text(
                Pos2::new(text_start_x, search_rect.center().y),
                egui::Align2::LEFT_CENTER,
                search_query,
                text_font.clone(),
                LcdPalette::text_primary(is_dark),
            );
        }

        // Blinking white underscore in search bar
        let approx_char_w = 6.0_f32 * scale;
        let text_w = if search_query.is_empty() { 0.0_f32 } else { search_query.len() as f32 * approx_char_w };
        let cur_x = text_start_x + text_w;
        let cur_y = search_rect.center().y + 5.5_f32 * scale;
        painter.line_segment(
            [Pos2::new(cur_x + 1.0_f32 * scale, cur_y), Pos2::new(cur_x + 7.0_f32 * scale, cur_y)],
            Stroke::new(1.5_f32 * scale, if is_dark { Color32::WHITE } else { Color32::BLACK }),
        );

        // Clear button [✕]
        if !search_query.is_empty() {
            let clear_btn_rect = Rect::from_center_size(
                Pos2::new(search_rect.max.x - 10.0_f32 * scale, search_rect.center().y),
                Vec2::splat(16.0_f32 * scale),
            );
            painter.rect_filled(clear_btn_rect, 3.0_f32 * scale, Color32::from_white_alpha(35));
            painter.text(
                clear_btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                FontId::proportional(8.5_f32 * scale),
                Color32::WHITE,
            );
        }

        // 2. Filtered Songs List
        let list_top_y = search_rect.max.y + 4.0_f32 * scale;
        let list_rect = Rect::from_min_max(
            Pos2::new(content_rect.min.x, list_top_y),
            screen_rect.max,
        );

        let filtered_songs = Self::get_filtered_songs_for_add(library, search_query);
        let total_songs = filtered_songs.len();

        if total_songs == 0 {
            painter.text(
                Pos2::new(list_rect.center().x, list_rect.center().y - 10.0_f32 * scale),
                egui::Align2::CENTER_CENTER,
                "No matching songs found.",
                FontId::proportional(11.0_f32 * scale),
                LcdPalette::text_secondary(is_dark),
            );
            return;
        }

        let sel = state.get_selected_index("playlist_add_songs").min(total_songs.saturating_sub(1));
        let item_h = 22.0_f32 * scale;
        let visible_count = ((list_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if sel >= visible_count {
            sel - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let item_idx = scroll_offset + i;
            if item_idx >= total_songs {
                break;
            }

            let song_id = filtered_songs[item_idx];
            let song = match library.songs.get(song_id) {
                Some(s) => s,
                None => continue,
            };

            let item_rect = Rect::from_min_size(
                Pos2::new(list_rect.min.x, list_rect.min.y + (i as f32) * item_h),
                Vec2::new(list_rect.width(), item_h),
            );
            let is_selected = item_idx == sel;

            let in_playlist = playlist.contains_song(song_id, &library.songs);
            let check_icon = if in_playlist { "✔ Added" } else { "+ Add" };

            let time = painter.ctx().input(|inp| inp.time);
            let display_artist = LcdRenderer::get_display_artist(&song.artist, time);
            let label = format!("{} • {}", song.title, display_artist);
            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                &label,
                Some(check_icon),
                false,
                is_selected,
                is_dark,
            );
        }

        if total_songs > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (list_rect.height() * (visible_count as f32 / total_songs as f32)).max(12.0_f32 * scale);
            let thumb_y = list_rect.min.y + (list_rect.height() - thumb_h) * (scroll_offset as f32 / (total_songs - visible_count) as f32);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)),
                2.0_f32 * scale,
                Color32::from_rgb(180, 185, 195),
            );
        }
    }

    /// Render Playlist Options Screen (Rename, Change Cover, Delete)
    pub fn render_playlist_options_screen(
        painter: &Painter,
        screen_rect: Rect,
        playlist_idx: usize,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
    ) {
        let playlist = match library.playlists.get(playlist_idx) {
            Some(pl) => pl,
            None => return,
        };

        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Playlist Options", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let cover_str = if playlist.custom_cover_path.is_some() { "Custom Image Set" } else { "Default" };
        let items = [
            ("Rename Playlist...", Some(playlist.name.as_str()), true),
            ("Change Cover Image...", Some(cover_str), true),
            ("Clear Cover Image", None, false),
            ("Add Songs...", Some("Search library"), true),
            ("Delete Playlist", Some("Remove permanently"), false),
        ];

        let sel = state.get_selected_index("playlist_options").min(items.len() - 1);
        let item_h = 24.0_f32 * scale;

        for (i, (name, detail, arrow)) in items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            let is_selected = i == sel;
            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                name,
                *detail,
                *arrow,
                is_selected,
                is_dark,
            );
        }
    }

    /// Render Playlist View Selector (Default vs Modern)
    pub fn render_playlist_view_selector(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Playlist View", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let modes = [
            (PlaylistViewMode::Default, "Default (Classic iPod)"),
            (PlaylistViewMode::Modern, "Modern (Spotify Layout)"),
        ];

        let selected_idx = state.get_selected_index("playlist_view_selector").min(modes.len() - 1);
        let item_h = 24.0_f32 * scale;

        for (i, (mode, name)) in modes.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(content_rect.width(), item_h),
            );

            let is_selected = i == selected_idx;
            let is_active = state.playlist_view_mode == *mode;
            let detail = if is_active { "✔" } else { "" };

            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                name,
                Some(detail),
                false,
                is_selected,
                is_dark,
            );
        }
    }
}
