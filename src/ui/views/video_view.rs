use crate::audio::player::AudioPlayer;
use crate::library::model::{Library, VideoItem};
use crate::state::AppState;
use crate::ui::lcd::LcdRenderer;
use crate::video::engine::VideoPlayer;
use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub struct VideoView;

impl VideoView {
    pub fn render_videos_menu(
        painter: &Painter,
        screen_rect: Rect,
        state: &AppState,
        _library: &Library,
        player: &AudioPlayer,
    ) {
        let is_dark = state.display_theme.is_dark();
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        LcdRenderer::draw_status_bar(painter, screen_rect, "Videos", player, state.is_hold_locked, state.display_theme);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        let split_x = screen_rect.min.x + screen_rect.width() * 0.43_f32;

        let categories = [
            "Video Playlists",
            "Movies",
            "Music Videos",
            "TV Shows",
            "Video Podcasts",
        ];

        let selected_idx = state.get_selected_index("videos_menu").min(categories.len() - 1);
        let item_h = 22.0_f32 * scale;

        for (i, name) in categories.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x, content_rect.min.y + (i as f32) * item_h),
                Vec2::new(split_x - screen_rect.min.x, item_h),
            );
            LcdRenderer::draw_list_item(painter, item_rect, name, None, true, i == selected_idx, is_dark);
        }

        LcdRenderer::draw_split_view_divider(painter, split_x, content_rect.min.y, content_rect.max.y, is_dark);

        // Right Preview Pane: Video Thumbnail preview
        let preview_rect = Rect::from_min_max(Pos2::new(split_x + 1.0_f32, content_rect.min.y), content_rect.max);
        if is_dark {
            painter.rect_filled(preview_rect, 0.0_f32, Color32::from_black_alpha(20));
        } else {
            painter.rect_filled(preview_rect, 0.0_f32, Color32::from_rgb(246, 248, 250));
        }

        let max_thumb_w = (preview_rect.width() - 16.0_f32 * scale).max(20.0_f32);
        let max_thumb_h = (preview_rect.height() - 20.0_f32 * scale).max(20.0_f32);
        let thumb_w = (120.0_f32 * scale).min(max_thumb_w).min(max_thumb_h * (120.0_f32 / 80.0_f32));
        let thumb_h = thumb_w * (80.0_f32 / 120.0_f32);
        let thumb_rect = Rect::from_center_size(preview_rect.center(), Vec2::new(thumb_w, thumb_h));
        painter.rect_filled(thumb_rect, 4.0_f32 * scale, Color32::from_rgb(25, 28, 35));
        painter.rect_stroke(thumb_rect, 4.0_f32 * scale, Stroke::new(1.0_f32 * scale, Color32::from_rgb(100, 105, 120)));

        let icon_font_size = (thumb_h * 0.45_f32).clamp(10.0_f32, 40.0_f32);
        painter.text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            "🎬",
            FontId::proportional(icon_font_size),
            Color32::WHITE,
        );
    }

    pub fn render_video_player(
        painter: &Painter,
        screen_rect: Rect,
        _state: &AppState,
        video_player: &VideoPlayer,
        video: &VideoItem,
    ) {
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        // 1. Render Video Frame
        video_player.render_frame(painter, screen_rect, video.id);

        // 2. Render On-Screen Controls HUD if active
        if video_player.show_hud {
            // Top HUD Bar (Title & Mode)
            let top_h = 24.0_f32 * scale;
            let top_hud = Rect::from_min_size(screen_rect.min, Vec2::new(screen_rect.width(), top_h));
            painter.rect_filled(top_hud, 0.0_f32, Color32::from_black_alpha(170));

            painter.text(
                Pos2::new(top_hud.min.x + 8.0_f32 * scale, top_hud.center().y),
                egui::Align2::LEFT_CENTER,
                &video.title,
                FontId::proportional(11.0_f32 * scale),
                Color32::WHITE,
            );

            painter.text(
                Pos2::new(top_hud.max.x - 8.0_f32 * scale, top_hud.center().y),
                egui::Align2::RIGHT_CENTER,
                video_player.aspect_ratio.name(),
                FontId::proportional(10.0_f32 * scale),
                Color32::from_rgb(180, 220, 255),
            );

            // Bottom HUD Bar (Progress & Time)
            let bot_h = 32.0_f32 * scale;
            let bot_hud = Rect::from_min_max(
                Pos2::new(screen_rect.min.x, screen_rect.max.y - bot_h),
                screen_rect.max,
            );
            painter.rect_filled(bot_hud, 0.0_f32, Color32::from_black_alpha(170));

            // Play / Pause status icon
            let status_icon = if video_player.is_playing { "▶" } else { "❙❙" };
            painter.text(
                Pos2::new(bot_hud.min.x + 12.0_f32 * scale, bot_hud.center().y),
                egui::Align2::LEFT_CENTER,
                status_icon,
                FontId::proportional(11.0_f32 * scale),
                Color32::WHITE,
            );

            // Progress Bar
            let bar_x = bot_hud.min.x + 32.0_f32 * scale;
            let bar_w = bot_hud.width() - 110.0_f32 * scale;
            let bar_y = bot_hud.center().y - 3.0_f32 * scale;
            let bar_rect = Rect::from_min_size(Pos2::new(bar_x, bar_y), Vec2::new(bar_w, 6.0_f32 * scale));

            painter.rect_filled(bar_rect, 2.0_f32 * scale, Color32::from_white_alpha(70));
            let progress = if video_player.total_duration_sec > 0.0_f32 {
                (video_player.current_time_sec / video_player.total_duration_sec).clamp(0.0_f32, 1.0_f32)
            } else {
                0.0_f32
            };
            let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(bar_w * progress, 6.0_f32 * scale));
            painter.rect_filled(fill_rect, 2.0_f32 * scale, Color32::from_rgb(40, 140, 255));

            // Time Remaining
            let rem_s = (video_player.total_duration_sec - video_player.current_time_sec).max(0.0_f32);
            let time_str = format!("-{}", format_video_time(rem_s));
            painter.text(
                Pos2::new(bot_hud.max.x - 8.0_f32 * scale, bot_hud.center().y),
                egui::Align2::RIGHT_CENTER,
                time_str,
                FontId::proportional(10.0_f32 * scale),
                Color32::WHITE,
            );
        }
    }
}

fn format_video_time(seconds: f32) -> String {
    let s = seconds.floor() as u32;
    let mins = s / 60;
    let rem_s = s % 60;
    format!("{}:{:02}", mins, rem_s)
}
