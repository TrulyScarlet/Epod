use crate::audio::player::AudioPlayer;
use crate::library::model::Library;
use crate::state::AppState;
use crate::theme::{ChassisColor, ColorTarget, LcdPalette};
use crate::ui::lcd::LcdRenderer;
use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub const SEARCH_CHARS: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
    "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "␣", "⌫", "Clear",
];

pub struct SearchView;

impl SearchView {
    pub fn get_filtered_songs(library: &Library, query: &str) -> Vec<usize> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return library.sorted_song_indices.clone();
        }
        library
            .sorted_song_indices
            .iter()
            .copied()
            .filter(|&id| {
                if let Some(song) = library.songs.get(id) {
                    song.title.to_lowercase().contains(&q)
                        || song.artist.to_lowercase().contains(&q)
                        || song.album.to_lowercase().contains(&q)
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn render(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        
        let custom_ref = if state.chassis_color == ChassisColor::Custom {
            Some(&state.custom_theme)
        } else {
            None
        };

        let update_available = matches!(state.update_status, crate::updater::UpdateStatus::UpdateAvailable { .. });

        LcdRenderer::draw_status_bar_custom(
            painter,
            screen_rect,
            "Search",
            player,
            state.is_hold_locked,
            state.display_theme,
            custom_ref,
            update_available,
        );

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        // 1. Search Bar Box
        let search_box_h = 22.0_f32 * scale;
        let search_box_margin = 8.0_f32 * scale;
        let search_box_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x + search_box_margin, content_rect.min.y + 4.0_f32 * scale),
            Vec2::new(content_rect.width() - search_box_margin * 2.0_f32, search_box_h),
        );

        let box_bg = if is_dark {
            Color32::from_rgb(22, 24, 30)
        } else {
            Color32::from_rgb(245, 248, 252)
        };
        let box_border = if is_dark {
            Color32::from_white_alpha(50)
        } else {
            Color32::from_rgb(180, 195, 215)
        };

        painter.rect_filled(search_box_rect, 4.0_f32 * scale, box_bg);
        painter.rect_stroke(search_box_rect, 4.0_f32 * scale, Stroke::new(1.0_f32 * scale, box_border));

        // Search Icon
        painter.text(
            Pos2::new(search_box_rect.min.x + 8.0_f32 * scale, search_box_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "🔍",
            FontId::proportional(10.0_f32 * scale),
            Color32::from_rgb(120, 130, 145),
        );

        // Search Query Text
        let query_x = search_box_rect.min.x + 24.0_f32 * scale;
        let query_text = if state.search_query.is_empty() {
            "Type on keyboard or select letters below..."
        } else {
            &state.search_query
        };
        let query_color = if state.search_query.is_empty() {
            Color32::from_rgb(140, 145, 155)
        } else if let Some(custom) = custom_ref {
            custom.get_color32(ColorTarget::TitleText)
        } else if is_dark {
            Color32::from_rgb(250, 250, 255)
        } else {
            LcdPalette::TEXT_BLACK
        };

        painter.text(
            Pos2::new(query_x, search_box_rect.center().y),
            egui::Align2::LEFT_CENTER,
            query_text,
            FontId::proportional(11.0_f32 * scale),
            query_color,
        );

        // Clear [✕] button if query non-empty
        if !state.search_query.is_empty() {
            let clear_pos = Pos2::new(search_box_rect.max.x - 12.0_f32 * scale, search_box_rect.center().y);
            painter.text(
                clear_pos,
                egui::Align2::CENTER_CENTER,
                "✕",
                FontId::proportional(10.5_f32 * scale),
                Color32::from_rgb(160, 165, 175),
            );
        }

        // 2. Character Strip / Ribbon
        let ribbon_y = search_box_rect.max.y + 3.0_f32 * scale;
        let ribbon_h = 18.0_f32 * scale;
        let ribbon_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x, ribbon_y),
            Vec2::new(content_rect.width(), ribbon_h),
        );

        painter.rect_filled(
            ribbon_rect,
            0.0_f32,
            if is_dark { Color32::from_black_alpha(40) } else { Color32::from_rgb(238, 241, 246) },
        );

        let char_w = 16.0_f32 * scale;
        let visible_chars = ((content_rect.width() / char_w).floor() as usize).min(SEARCH_CHARS.len());
        let selected_char = state.search_char_idx.min(SEARCH_CHARS.len() - 1);

        let char_scroll = if selected_char >= visible_chars {
            selected_char - visible_chars + 1
        } else {
            0
        };

        for i in 0..visible_chars {
            let idx = char_scroll + i;
            if idx >= SEARCH_CHARS.len() {
                break;
            }
            let ch_str = SEARCH_CHARS[idx];
            let c_rect = Rect::from_min_size(
                Pos2::new(content_rect.min.x + (i as f32) * char_w, ribbon_y),
                Vec2::new(char_w, ribbon_h),
            );

            let is_sel = idx == selected_char;
            if is_sel {
                let badge_rect = c_rect.shrink(1.5_f32 * scale);
                let sel_bg = custom_ref
                    .map(|c| c.get_color32(ColorTarget::SelectionHighlight))
                    .unwrap_or(LcdPalette::SEL_TOP);
                painter.rect_filled(badge_rect, 3.0_f32 * scale, sel_bg);
            }

            let ch_col = if is_sel {
                Color32::WHITE
            } else if is_dark {
                Color32::from_rgb(200, 205, 220)
            } else {
                Color32::from_rgb(60, 65, 75)
            };

            painter.text(
                c_rect.center(),
                egui::Align2::CENTER_CENTER,
                ch_str,
                FontId::proportional(10.0_f32 * scale),
                ch_col,
            );
        }

        // 3. Results List
        let list_top_y = ribbon_rect.max.y + 2.0_f32 * scale;
        let list_rect = Rect::from_min_max(
            Pos2::new(content_rect.min.x, list_top_y),
            content_rect.max,
        );

        let filtered_ids = Self::get_filtered_songs(library, &state.search_query);

        if filtered_ids.is_empty() {
            painter.text(
                list_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("No songs found for '{}'", state.search_query),
                FontId::proportional(11.5_f32 * scale),
                LcdPalette::text_secondary(is_dark),
            );
            return;
        }

        let selected_row = state.get_selected_index("search_screen").min(filtered_ids.len().saturating_sub(1));
        let item_h = 22.0_f32 * scale;
        let visible_count = ((list_rect.height() / item_h).floor() as usize).max(1);

        let scroll_offset = if selected_row >= visible_count {
            selected_row - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let row_idx = scroll_offset + i;
            if row_idx >= filtered_ids.len() {
                break;
            }

            let song_id = filtered_ids[row_idx];
            let Some(song) = library.songs.get(song_id) else {
                continue;
            };

            let item_rect = Rect::from_min_size(
                Pos2::new(list_rect.min.x, list_rect.min.y + (i as f32) * item_h),
                Vec2::new(list_rect.width(), item_h),
            );

            let is_row_sel = row_idx == selected_row;
            let time = painter.ctx().input(|inp| inp.time);
            let display_artist = LcdRenderer::get_display_artist(&song.artist, time);

            LcdRenderer::draw_list_item_custom(
                painter,
                item_rect,
                &song.title,
                Some(&display_artist),
                false,
                is_row_sel,
                is_dark,
                custom_ref,
            );
        }

        // Scrollbar if needed
        if filtered_ids.len() > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = list_rect.max.x - bar_w;
            let thumb_h = (list_rect.height() * (visible_count as f32 / filtered_ids.len() as f32)).max(12.0_f32 * scale);
            let thumb_y = list_rect.min.y + (list_rect.height() - thumb_h) * (scroll_offset as f32 / (filtered_ids.len() - visible_count) as f32);
            painter.rect_filled(Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)), 2.0_f32 * scale, Color32::from_rgb(180, 185, 195));
        }
    }
}
