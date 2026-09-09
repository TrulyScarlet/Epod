use crate::audio::player::AudioPlayer;
use crate::library::model::Library;
use crate::state::AppState;
use crate::theme::{ChassisColor, ColorTarget, DisplayTheme, LcdPalette};
use crate::ui::art_cache::ArtCache;
use crate::ui::lcd::LcdRenderer;
use egui::{Color32, Context, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub struct MenuView;

impl MenuView {
    pub fn render_main_menu(
        painter: &Painter,
        ctx: &Context,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
        art_cache: &mut ArtCache,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Epod", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let split_x = screen_rect.min.x + screen_rect.width() * 0.43_f32;
        let item_h = 22.0_f32 * scale;

        let active_items = state.get_active_main_menu_items(player);
        let selected_idx = state.get_selected_index("main_menu").min(active_items.len().saturating_sub(1));

        for (i, item) in active_items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(split_x - screen_rect.min.x, item_h),
            );
            LcdRenderer::draw_list_item(painter, item_rect, item.title(), None, item.has_arrow(), i == selected_idx, is_dark);
        }

        // Split view vertical divider
        LcdRenderer::draw_split_view_divider(painter, split_x, content_rect.min.y, content_rect.max.y, is_dark);

        // Right Preview Pane
        let preview_rect = Rect::from_min_max(Pos2::new(split_x + 1.0_f32, content_rect.min.y), content_rect.max);
        render_right_preview_pane(painter, ctx, preview_rect, selected_idx, state, library, art_cache, scale);
    }

    pub fn render_music_menu(
        painter: &Painter,
        ctx: &Context,
        screen_rect: Rect,
        state: &AppState,
        library: &Library,
        player: &AudioPlayer,
        art_cache: &mut ArtCache,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Music", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let split_x = screen_rect.min.x + screen_rect.width() * 0.43_f32;

        let items = [
            "Playlists",
            "Artists",
            "Albums",
            "Songs",
            "Genres",
            "Composers",
            "Search",
        ];

        let selected_idx = state.get_selected_index("music_menu").min(items.len() - 1);
        let item_h = 22.0_f32 * scale;

        for (i, name) in items.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(split_x - screen_rect.min.x, item_h),
            );
            LcdRenderer::draw_list_item(painter, item_rect, name, None, true, i == selected_idx, is_dark);
        }

        LcdRenderer::draw_split_view_divider(painter, split_x, content_rect.min.y, content_rect.max.y, is_dark);

        // Right Preview Pane
        let preview_rect = Rect::from_min_max(Pos2::new(split_x + 1.0_f32, content_rect.min.y), content_rect.max);
        render_right_preview_pane(painter, ctx, preview_rect, selected_idx, state, library, art_cache, scale);
    }

    pub fn render_full_list(
        painter: &Painter,
        screen_rect: Rect,
        title: &str,
        items: &[String],
        details: Option<&[String]>,
        selected_idx: usize,
        has_arrow: bool,
        player: &AudioPlayer,
        is_locked: bool,
        display_theme: DisplayTheme,
    ) {
        let is_dark = display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, title, player, is_locked, display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

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

            let det = details.and_then(|d| d.get(item_idx)).map(|s| s.as_str());
            LcdRenderer::draw_list_item(
                painter,
                item_rect,
                &items[item_idx],
                det,
                has_arrow,
                item_idx == selected_idx,
                is_dark,
            );
        }

        // Scrollbar if needed
        if items.len() > visible_count {
            let bar_w = 4.0_f32 * scale;
            let bar_x = content_rect.max.x - bar_w;
            let thumb_h = (content_rect.height() * (visible_count as f32 / items.len() as f32)).max(12.0_f32 * scale);
            let thumb_y = content_rect.min.y + (content_rect.height() - thumb_h) * (scroll_offset as f32 / (items.len() - visible_count) as f32);
            let thumb_rect = Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h));
            painter.rect_filled(thumb_rect, 2.0_f32 * scale, Color32::from_rgb(180, 185, 195));
        }
    }
}

fn render_right_preview_pane(
    painter: &Painter,
    ctx: &Context,
    rect: Rect,
    selected_idx: usize,
    state: &AppState,
    library: &Library,
    art_cache: &mut ArtCache,
    scale: f32,
) {
    let is_dark = state.display_theme.is_dark();
    if state.display_theme.is_album_cover() {
        painter.rect_filled(rect, 0.0_f32, Color32::from_black_alpha(15));
    } else if is_dark {
        painter.rect_filled(rect, 0.0_f32, Color32::from_black_alpha(20));
    } else {
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(246, 248, 250));
    }

    if let Some(&song_id) = state.current_queue.get(state.current_queue_idx) {
        if let Some(song) = library.songs.get(song_id) {
            let max_text_w = (rect.width() - 14.0_f32 * scale).max(10.0_f32);
            let title_h = 13.0_f32 * scale;
            let artist_h = 11.0_f32 * scale;
            let gap_art_text = 6.0_f32 * scale;
            let gap_title_artist = 3.0_f32 * scale;
            let text_block_h = title_h + gap_title_artist + artist_h;

            let max_art_w = (rect.width() - 16.0_f32 * scale).max(20.0_f32);
            let max_art_h = (rect.height() - text_block_h - gap_art_text - 14.0_f32 * scale).max(20.0_f32);
            let target_art_size = 96.0_f32 * scale;
            let art_size = target_art_size.min(max_art_w).min(max_art_h);

            let total_content_h = art_size + gap_art_text + text_block_h;
            let start_y = (rect.center().y - total_content_h * 0.5_f32).max(rect.min.y + 4.0_f32 * scale);

            let art_rect = Rect::from_min_size(
                Pos2::new(rect.center().x - art_size * 0.5_f32, start_y),
                Vec2::splat(art_size),
            );

            painter.rect_filled(art_rect.expand(1.5_f32 * scale), 3.0_f32 * scale, Color32::from_black_alpha(25));

            if let Some(texture) = art_cache.get_or_load(ctx, song.id, song.artwork_bytes.as_deref(), song.file_path.as_deref()) {
                painter.image(
                    texture.id(),
                    art_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0_f32, 1.0_f32)),
                    Color32::WHITE,
                );
                painter.rect_stroke(art_rect, 2.0_f32 * scale, Stroke::new(1.0_f32 * scale, Color32::from_black_alpha(50)));
            } else {
                draw_procedural_album_art(painter, art_rect, &song.album, song.id);
            }

            // Now Playing Track Info (Clipped with smooth marquee to never overflow)
            let title_col = if state.chassis_color == ChassisColor::Custom {
                state.custom_theme.get_color32(ColorTarget::TitleText)
            } else {
                LcdPalette::text_primary(is_dark)
            };
            let artist_col = if state.chassis_color == ChassisColor::Custom {
                state.custom_theme.get_color32(ColorTarget::ArtistText)
            } else {
                LcdPalette::text_secondary(is_dark)
            };

            let time = ctx.input(|i| i.time);
            let title_y = art_rect.max.y + gap_art_text;
            let title_rect = Rect::from_center_size(
                Pos2::new(rect.center().x, title_y + title_h * 0.5_f32),
                Vec2::new(max_text_w, title_h),
            );
            LcdRenderer::render_scrolling_text(
                painter,
                ctx,
                &song.title,
                FontId::proportional(11.0_f32 * scale),
                title_col,
                title_rect,
                time,
                egui::Align2::CENTER_CENTER,
            );

            let artist_y = title_y + title_h + gap_title_artist;
            let artist_rect = Rect::from_center_size(
                Pos2::new(rect.center().x, artist_y + artist_h * 0.5_f32),
                Vec2::new(max_text_w, artist_h),
            );
            let display_artist = LcdRenderer::get_display_artist(&song.artist, time);
            LcdRenderer::render_scrolling_text(
                painter,
                ctx,
                &display_artist,
                FontId::proportional(10.0_f32 * scale),
                artist_col,
                artist_rect,
                time + 1.0,
                egui::Align2::CENTER_CENTER,
            );
            return;
        }
    }

    // Default Category Artwork Preview
    let max_icon_size = (rect.width() - 20.0_f32 * scale).min(rect.height() - 20.0_f32 * scale).max(16.0_f32);
    let icon_size = (80.0_f32 * scale).min(max_icon_size);
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(icon_size));
    draw_category_artwork_icon(painter, icon_rect, selected_idx, is_dark, scale);
}

pub fn draw_procedural_album_art(painter: &Painter, rect: Rect, album_name: &str, seed: usize) {
    let colors = [
        (Color32::from_rgb(230, 45, 90), Color32::from_rgb(85, 20, 150)),
        (Color32::from_rgb(40, 140, 245), Color32::from_rgb(10, 45, 120)),
        (Color32::from_rgb(245, 160, 30), Color32::from_rgb(210, 40, 60)),
        (Color32::from_rgb(45, 200, 130), Color32::from_rgb(15, 90, 80)),
        (Color32::from_rgb(170, 70, 240), Color32::from_rgb(50, 15, 110)),
    ];
    let (c1, c2) = colors[seed % colors.len()];

    let half_h = rect.height() * 0.5_f32;
    painter.rect_filled(
        Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + half_h)),
        3.0_f32,
        c1,
    );
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(rect.min.x, rect.min.y + half_h), rect.max),
        3.0_f32,
        c2,
    );

    painter.circle_stroke(rect.center(), rect.width() * 0.28_f32, Stroke::new(2.0_f32, Color32::from_white_alpha(160)));
    painter.circle_filled(rect.center(), rect.width() * 0.12_f32, Color32::from_white_alpha(200));

    let snippet = if album_name.len() > 16 { &album_name[..16] } else { album_name };
    let text_size = (rect.width() * 0.088_f32).clamp(6.0_f32, 14.0_f32);
    let text_clip = painter.with_clip_rect(rect.shrink(2.0_f32));
    text_clip.text(
        Pos2::new(rect.center().x, rect.max.y - rect.height() * 0.12_f32),
        egui::Align2::CENTER_CENTER,
        snippet,
        FontId::proportional(text_size),
        Color32::WHITE,
    );

    painter.rect_stroke(rect, 3.0_f32, Stroke::new(1.0_f32, Color32::from_black_alpha(40)));
}

fn draw_category_artwork_icon(painter: &Painter, rect: Rect, idx: usize, is_dark: bool, scale: f32) {
    let r_rounding = (8.0_f32 * scale).min(rect.width() * 0.2_f32);
    if is_dark {
        painter.rect_filled(rect, r_rounding, Color32::from_white_alpha(20));
        painter.rect_stroke(rect, r_rounding, Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_white_alpha(40)));
    } else {
        painter.rect_filled(rect, r_rounding, Color32::from_rgb(230, 235, 242));
        painter.rect_stroke(rect, r_rounding, Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_rgb(200, 205, 215)));
    }

    let icon_char = match idx {
        0 => "🎵",
        1 => "🎬",
        2 => "🔀",
        3 => "▶",
        _ => "⚙",
    };

    let text_col = if is_dark { Color32::WHITE } else { Color32::from_rgb(70, 80, 95) };
    let font_size = (rect.width() * 0.45_f32).clamp(10.0_f32, 48.0_f32);

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon_char,
        FontId::proportional(font_size),
        text_col,
    );
}
