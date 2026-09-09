use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VideoAspectRatio {
    Widescreen16x9,
    Fullscreen4x3,
    Zoom,
}

impl VideoAspectRatio {
    pub fn name(&self) -> &'static str {
        match self {
            VideoAspectRatio::Widescreen16x9 => "16:9 Widescreen",
            VideoAspectRatio::Fullscreen4x3 => "4:3 Fullscreen",
            VideoAspectRatio::Zoom => "Widescreen Zoom",
        }
    }
}

pub struct VideoPlayer {
    pub current_video_id: Option<usize>,
    pub is_playing: bool,
    pub current_time_sec: f32,
    pub total_duration_sec: f32,
    pub aspect_ratio: VideoAspectRatio,
    pub show_hud: bool,
    pub hud_timer_sec: f32,
    last_update: Instant,
}

impl VideoPlayer {
    pub fn new() -> Self {
        Self {
            current_video_id: None,
            is_playing: false,
            current_time_sec: 0.0_f32,
            total_duration_sec: 180.0_f32,
            aspect_ratio: VideoAspectRatio::Widescreen16x9,
            show_hud: true,
            hud_timer_sec: 3.0_f32,
            last_update: Instant::now(),
        }
    }

    pub fn load_video(&mut self, video_id: usize, duration_sec: f32, resume_pos: f32) {
        self.current_video_id = Some(video_id);
        self.total_duration_sec = duration_sec;
        self.current_time_sec = resume_pos;
        self.is_playing = true;
        self.show_hud = true;
        self.hud_timer_sec = 3.0_f32;
        self.last_update = Instant::now();
    }

    pub fn toggle_play_pause(&mut self) {
        self.is_playing = !self.is_playing;
        self.ping_hud();
    }

    pub fn ping_hud(&mut self) {
        self.show_hud = true;
        self.hud_timer_sec = 3.0_f32;
    }

    pub fn seek(&mut self, delta_sec: f32) {
        self.current_time_sec = (self.current_time_sec + delta_sec).clamp(0.0_f32, self.total_duration_sec);
        self.ping_hud();
    }

    pub fn toggle_aspect_ratio(&mut self) {
        self.aspect_ratio = match self.aspect_ratio {
            VideoAspectRatio::Widescreen16x9 => VideoAspectRatio::Fullscreen4x3,
            VideoAspectRatio::Fullscreen4x3 => VideoAspectRatio::Zoom,
            VideoAspectRatio::Zoom => VideoAspectRatio::Widescreen16x9,
        };
        self.ping_hud();
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        if self.is_playing {
            self.current_time_sec += dt;
            if self.current_time_sec >= self.total_duration_sec {
                self.current_time_sec = self.total_duration_sec;
                self.is_playing = false;
                self.show_hud = true;
            }
        }

        if self.show_hud {
            self.hud_timer_sec -= dt;
            if self.hud_timer_sec <= 0.0_f32 && self.is_playing {
                self.show_hud = false;
            }
        }
    }

    pub fn render_frame(&self, painter: &Painter, screen_rect: Rect, video_id: usize) {
        let t = self.current_time_sec;
        let w = screen_rect.width();
        let h = screen_rect.height();

        let video_rect = match self.aspect_ratio {
            VideoAspectRatio::Fullscreen4x3 => screen_rect,
            VideoAspectRatio::Widescreen16x9 => {
                let target_h = w * (9.0_f32 / 16.0_f32);
                let pad_y = (h - target_h) / 2.0_f32;
                painter.rect_filled(screen_rect, 0.0_f32, Color32::BLACK);
                Rect::from_min_size(
                    Pos2::new(screen_rect.min.x, screen_rect.min.y + pad_y),
                    Vec2::new(w, target_h),
                )
            }
            VideoAspectRatio::Zoom => screen_rect,
        };

        match video_id % 4 {
            0 => render_keynote_scene(painter, video_rect, t),
            1 => render_synthwave_mv_scene(painter, video_rect, t),
            2 => render_cyber_odyssey_scene(painter, video_rect, t),
            _ => render_podcast_scene(painter, video_rect, t),
        }
    }
}

fn render_synthwave_mv_scene(painter: &Painter, rect: Rect, t: f32) {
    painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(15, 5, 30));

    let sun_center = Pos2::new(rect.center().x, rect.min.y + rect.height() * 0.42_f32);
    let sun_radius = rect.height() * 0.28_f32;

    for r in (0..=sun_radius as i32).rev() {
        let frac = r as f32 / sun_radius;
        let c = Color32::from_rgb(
            255,
            (frac * 180.0_f32) as u8,
            (frac * 40.0_f32) as u8,
        );
        painter.circle_filled(sun_center, r as f32, c);
    }

    let horizon_y = rect.min.y + rect.height() * 0.52_f32;
    for i in 1..8 {
        let line_y = sun_center.y - sun_radius * 0.2_f32 + (i as f32) * (sun_radius * 0.15_f32);
        if line_y < horizon_y {
            painter.line_segment(
                [
                    Pos2::new(sun_center.x - sun_radius, line_y),
                    Pos2::new(sun_center.x + sun_radius, line_y),
                ],
                Stroke::new(2.0_f32, Color32::from_rgb(15, 5, 30)),
            );
        }
    }

    painter.rect_filled(
        Rect::from_min_max(Pos2::new(rect.min.x, horizon_y), rect.max),
        0.0_f32,
        Color32::from_rgb(10, 0, 20),
    );

    let grid_speed = (t * 2.5_f32).fract();
    let num_horiz = 12;
    for i in 0..num_horiz {
        let p = (i as f32 + grid_speed) / (num_horiz as f32);
        let y = horizon_y + p * p * (rect.max.y - horizon_y);
        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0, 220, 255, (p * 200.0_f32) as u8)),
        );
    }

    for i in -6..=6 {
        let spread = (i as f32) * (rect.width() * 0.12_f32);
        painter.line_segment(
            [
                Pos2::new(rect.center().x, horizon_y),
                Pos2::new(rect.center().x + spread * 2.5_f32, rect.max.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(255, 0, 150)),
        );
    }
}

fn render_keynote_scene(painter: &Painter, rect: Rect, t: f32) {
    painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(12, 14, 20));

    let spot_top = Pos2::new(rect.center().x, rect.min.y);
    painter.line_segment(
        [spot_top, Pos2::new(rect.min.x + 30.0_f32, rect.max.y)],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(80, 120, 200, 40)),
    );
    painter.line_segment(
        [spot_top, Pos2::new(rect.max.x - 30.0_f32, rect.max.y)],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(80, 120, 200, 40)),
    );

    let screen_w = rect.width() * 0.55_f32;
    let screen_h = rect.height() * 0.45_f32;
    let stage_screen = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.min.y + rect.height() * 0.35_f32),
        Vec2::new(screen_w, screen_h),
    );
    painter.rect_filled(stage_screen, 4.0_f32, Color32::from_rgb(30, 40, 60));
    painter.rect_stroke(stage_screen, 4.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(100, 150, 255)));

    let pulse = ((t * 2.0_f32).sin() * 0.5_f32 + 0.5_f32) * 50.0_f32;
    painter.text(
        stage_screen.center(),
        egui::Align2::CENTER_CENTER,
        "Epod (5th Gen)\nOne more thing...",
        egui::FontId::proportional(12.0_f32),
        Color32::from_rgb(200 + pulse as u8, 220, 255),
    );

    painter.rect_filled(
        Rect::from_min_max(Pos2::new(rect.min.x, rect.min.y + rect.height() * 0.7_f32), rect.max),
        0.0_f32,
        Color32::from_rgb(25, 28, 38),
    );
}

fn render_cyber_odyssey_scene(painter: &Painter, rect: Rect, t: f32) {
    painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(5, 5, 12));

    let center = rect.center();
    for i in 0..40 {
        let angle = (i as f32 * 0.3_f32 + 0.1_f32) * std::f32::consts::TAU;
        let speed = (i % 5 + 1) as f32 * 15.0_f32;
        let dist = ((t * speed + i as f32 * 25.0_f32) % (rect.width() * 0.75_f32)) + 5.0_f32;
        let x = center.x + angle.cos() * dist;
        let y = center.y + angle.sin() * dist * 0.75_f32;
        if rect.contains(Pos2::new(x, y)) {
            let size = (dist / 60.0_f32).clamp(1.0_f32, 3.5_f32);
            painter.circle_filled(Pos2::new(x, y), size, Color32::from_rgb(180, 220, 255));
        }
    }

    painter.circle_stroke(center, 24.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(0, 255, 180)));
    painter.line_segment(
        [Pos2::new(center.x - 30.0_f32, center.y), Pos2::new(center.x - 10.0_f32, center.y)],
        Stroke::new(1.0_f32, Color32::from_rgb(0, 255, 180)),
    );
    painter.line_segment(
        [Pos2::new(center.x + 10.0_f32, center.y), Pos2::new(center.x + 30.0_f32, center.y)],
        Stroke::new(1.0_f32, Color32::from_rgb(0, 255, 180)),
    );
}

fn render_podcast_scene(painter: &Painter, rect: Rect, t: f32) {
    painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(20, 22, 28));

    let num_bars = 16;
    let bar_w = (rect.width() * 0.7_f32) / num_bars as f32;
    let start_x = rect.center().x - (num_bars as f32 * bar_w) / 2.0_f32;

    for i in 0..num_bars {
        let phase = i as f32 * 0.6_f32 + t * 4.0_f32;
        let height = ((phase.sin() * 0.5_f32 + 0.5_f32) * 45.0_f32 + 10.0_f32).clamp(5.0_f32, 60.0_f32);
        let x = start_x + (i as f32) * bar_w;
        let y = rect.center().y + 20.0_f32 - height;
        let bar_rect = Rect::from_min_size(Pos2::new(x + 2.0_f32, y), Vec2::new(bar_w - 4.0_f32, height));
        painter.rect_filled(bar_rect, 2.0_f32, Color32::from_rgb(80, 170, 255));
    }

    painter.text(
        Pos2::new(rect.center().x, rect.min.y + 35.0_f32),
        egui::Align2::CENTER_CENTER,
        "RETRO TECH PODCAST",
        egui::FontId::proportional(13.0_f32),
        Color32::WHITE,
    );
}
