use crate::theme::{ChassisColor, CustomThemeConfig};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub struct BodyRenderer;

impl BodyRenderer {
    pub fn render_chassis(
        ui: &mut Ui,
        body_rect: Rect,
        chassis: ChassisColor,
        custom: &CustomThemeConfig,
        is_hold_locked: &mut bool,
        pixel_tex: Option<&egui::TextureHandle>,
        is_pixel: bool,
    ) -> (Rect, Option<WindowAction>) {
        let mut window_action = None;
        let pixel_mode = is_pixel && pixel_tex.is_some();
        let scale = (body_rect.width() / 370.0_f32).min(body_rect.height() / 606.0_f32).max(0.2);
        let rounding = (body_rect.width() * 0.068_f32).min(body_rect.height() * 0.042_f32).max(8.0_f32);

        // 0. Top Drag Header Zone
        let header_h = 32.0_f32 * scale;
        let top_header_rect = Rect::from_min_size(body_rect.min, Vec2::new(body_rect.width(), header_h));
        let header_resp = ui.allocate_rect(top_header_rect, Sense::drag());
        if header_resp.drag_started() {
            window_action = Some(WindowAction::StartDrag);
        }

        let top_y = body_rect.min.y + 8.0_f32 * scale;

        // 3.5mm Headphone / Aux Jack (Clickable shortcut to Settings!)
        let jack_pos = Pos2::new(body_rect.min.x + 36.0_f32 * scale, top_y + 4.0_f32 * scale);
        let jack_rect = Rect::from_center_size(jack_pos, Vec2::splat(18.0_f32 * scale));
        let jack_resp = ui.allocate_rect(jack_rect, Sense::click());
        if jack_resp.clicked() {
            window_action = Some(WindowAction::OpenSettings);
        }

        // Top Controls: Interactive Hold Switch (Left-center)
        let switch_w = 24.0_f32 * scale;
        let switch_h = 7.0_f32 * scale;
        let switch_rect = Rect::from_center_size(
            Pos2::new(body_rect.min.x + 80.0_f32 * scale, top_y + 4.0_f32 * scale),
            Vec2::new(switch_w, switch_h),
        );
        let switch_resp = ui.allocate_rect(switch_rect, Sense::click());
        if switch_resp.clicked() {
            *is_hold_locked = !*is_hold_locked;
        }

        // Window Control Buttons (Top Right): [ - ] [ □ ] [ ✕ ]
        let btn_size = 18.0_f32 * scale;
        let right_anchor = body_rect.max.x - 14.0_f32 * scale;

        // Close on far right
        let close_rect = Rect::from_center_size(
            Pos2::new(right_anchor - btn_size * 0.5_f32, top_y + 4.0_f32 * scale),
            Vec2::splat(btn_size),
        );
        // Enlarge in middle
        let max_rect = Rect::from_center_size(
            Pos2::new(right_anchor - btn_size * 1.6_f32, top_y + 4.0_f32 * scale),
            Vec2::splat(btn_size),
        );
        // Minimize on left
        let min_rect = Rect::from_center_size(
            Pos2::new(right_anchor - btn_size * 2.7_f32, top_y + 4.0_f32 * scale),
            Vec2::splat(btn_size),
        );

        let min_resp = ui.allocate_rect(min_rect, Sense::click());
        if min_resp.clicked() {
            window_action = Some(WindowAction::Minimize);
        }

        let max_resp = ui.allocate_rect(max_rect, Sense::click());
        if max_resp.clicked() {
            window_action = Some(WindowAction::Maximize);
        }

        let close_resp = ui.allocate_rect(close_resp_rect(close_rect), Sense::click());
        if close_resp.clicked() {
            window_action = Some(WindowAction::Close);
        }

        let painter = ui.painter().clone();

        if pixel_mode {
            // Pixel Art Theme: draw the procedural pixel-art iPod 5G sprite.
            let tex = pixel_tex.unwrap();
            painter.image(
                tex.id(),
                body_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0_f32, 1.0_f32)),
                Color32::WHITE,
            );
        } else {
            // 1. Stainless Steel Rim (outer back casing visible at the beveled edge)
            let rim_rect = body_rect;
            painter.rect_filled(rim_rect, rounding, chassis.rim_color_with_custom(custom));
            painter.rect_stroke(
                rim_rect,
                rounding,
                Stroke::new((1.5_f32 * scale).max(1.0_f32), Color32::from_rgb(170, 175, 185)),
            );

            // Specular highlight along top/left edge of steel rim
            painter.line_segment(
                [
                    Pos2::new(rim_rect.min.x + rounding, rim_rect.min.y + 1.0_f32),
                    Pos2::new(rim_rect.max.x - rounding, rim_rect.min.y + 1.0_f32),
                ],
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_white_alpha(180)),
            );

            // 2. Polycarbonate Faceplate
            let face_inset = 3.5_f32 * scale;
            let face_rect = body_rect.shrink(face_inset);
            painter.rect_filled(face_rect, (rounding - 2.0_f32 * scale).max(4.0_f32), chassis.body_color_with_custom(custom));

            // Specular shine on polycarbonate edge
            painter.rect_stroke(
                face_rect,
                (rounding - 2.0_f32 * scale).max(4.0_f32),
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_white_alpha(60)),
            );
        }

        // 3. Top Header: Headphone Jack, Hold Switch, Window Controls
        // 3.5mm Headphone / Aux Jack (Interactive)
        let jack_r = 4.5_f32 * scale;
        painter.circle_filled(jack_pos, jack_r, Color32::from_rgb(40, 42, 45));
        let jack_rim_color = if jack_resp.hovered() {
            Color32::from_rgb(100, 180, 255)
        } else {
            Color32::from_rgb(180, 185, 195)
        };
        painter.circle_stroke(jack_pos, jack_r, Stroke::new(if jack_resp.hovered() { 1.5_f32 * scale } else { 1.0_f32 * scale }, jack_rim_color));
        painter.circle_filled(jack_pos, 2.5_f32 * scale, Color32::from_rgb(15, 15, 18));

        // Hold Switch
        painter.rect_filled(switch_rect, 3.5_f32 * scale, Color32::from_rgb(60, 65, 75));
        if *is_hold_locked {
            let orange_rect = Rect::from_min_size(
                switch_rect.min + Vec2::new(2.0_f32 * scale, 1.5_f32 * scale),
                Vec2::new(7.0_f32 * scale, switch_h - 3.0_f32 * scale),
            );
            let hold_accent = if chassis == ChassisColor::Custom {
                Color32::from_rgb(custom.hold_switch_color[0], custom.hold_switch_color[1], custom.hold_switch_color[2])
            } else {
                Color32::from_rgb(255, 100, 20)
            };
            painter.rect_filled(orange_rect, 1.5_f32 * scale, hold_accent);
        }
        let slider_x = if *is_hold_locked {
            switch_rect.max.x - 10.0_f32 * scale
        } else {
            switch_rect.min.x + 2.0_f32 * scale
        };
        let thumb_rect = Rect::from_min_size(
            Pos2::new(slider_x, switch_rect.min.y + 1.0_f32 * scale),
            Vec2::new(8.0_f32 * scale, switch_h - 2.0_f32 * scale),
        );
        painter.rect_filled(thumb_rect, 2.0_f32 * scale, Color32::from_rgb(210, 215, 222));
        painter.rect_stroke(thumb_rect, 2.0_f32 * scale, Stroke::new(0.5_f32 * scale, Color32::from_rgb(120, 125, 135)));

        let ctrl_color = if chassis.is_dark() {
            Color32::from_rgb(160, 165, 175)
        } else {
            Color32::from_rgb(140, 145, 155)
        };

        // Vector Render Window Controls [ - ] [ □ ] [ ✕ ]
        let stroke_w = (1.8_f32 * scale).max(1.0_f32);
        // 1. Minimize Button [ - ]
        let min_col = if min_resp.hovered() { Color32::from_rgb(40, 130, 255) } else { ctrl_color };
        let min_center = min_rect.center();
        painter.line_segment(
            [Pos2::new(min_center.x - 4.5_f32 * scale, min_center.y), Pos2::new(min_center.x + 4.5_f32 * scale, min_center.y)],
            Stroke::new(stroke_w, min_col),
        );

        // 2. Enlarge Button [ □ ]
        let max_col = if max_resp.hovered() { Color32::from_rgb(40, 130, 255) } else { ctrl_color };
        let max_center = max_rect.center();
        let box_rect = Rect::from_center_size(max_center, Vec2::new(8.5_f32 * scale, 8.5_f32 * scale));
        painter.rect_stroke(box_rect, 1.0_f32 * scale, Stroke::new((1.5_f32 * scale).max(1.0_f32), max_col));

        // 3. Close Button [ ✕ ]
        let close_col = if close_resp.hovered() { Color32::from_rgb(245, 55, 55) } else { ctrl_color };
        let close_center = close_rect.center();
        let cross_size = 4.5_f32 * scale;
        painter.line_segment(
            [
                Pos2::new(close_center.x - cross_size, close_center.y - cross_size),
                Pos2::new(close_center.x + cross_size, close_center.y + cross_size),
            ],
            Stroke::new(stroke_w, close_col),
        );
        painter.line_segment(
            [
                Pos2::new(close_center.x - cross_size, close_center.y + cross_size),
                Pos2::new(close_center.x + cross_size, close_center.y - cross_size),
            ],
            Stroke::new(stroke_w, close_col),
        );

        // 4. Bottom: 30-Pin Dock Connector cutout
        let bottom_y = body_rect.max.y - 4.0_f32 * scale;
        let dock_rect = Rect::from_center_size(
            Pos2::new(body_rect.center().x, bottom_y),
            Vec2::new(48.0_f32 * scale, 5.0_f32 * scale),
        );
        painter.rect_filled(dock_rect, 2.0_f32 * scale, Color32::from_rgb(45, 48, 55));
        painter.rect_stroke(dock_rect, 2.0_f32 * scale, Stroke::new(0.5_f32 * scale, Color32::from_rgb(140, 145, 155)));

        // 5. Virtual 4:3 LCD Screen Rect
        let screen_rect = if pixel_mode {
            // Match procedural pixel art sprite screen cutout exactly
            let sx = body_rect.min.x + body_rect.width() * (9.0_f32 / 185.0_f32);
            let sy = body_rect.min.y + body_rect.height() * (15.0_f32 / 303.0_f32);
            let sw = body_rect.width() * (168.0_f32 / 185.0_f32);
            let sh = body_rect.height() * (127.0_f32 / 303.0_f32);
            Rect::from_min_size(Pos2::new(sx, sy), Vec2::new(sw, sh))
        } else {
            let screen_margin_top = (body_rect.height() * 0.052_f32).max(header_h + 2.0_f32 * scale);
            let max_screen_h = (body_rect.height() * 0.44_f32).max(60.0_f32);
            let max_screen_w = (body_rect.width() * 0.90_f32).max(80.0_f32);
            let screen_w = (max_screen_h * (4.0_f32 / 3.0_f32)).min(max_screen_w);
            let screen_h = screen_w * 0.75_f32; // Exact 4:3 LCD proportion

            let s_rect = Rect::from_min_size(
                Pos2::new(body_rect.center().x - screen_w * 0.5_f32, body_rect.min.y + screen_margin_top),
                Vec2::new(screen_w, screen_h),
            );

            let bezel_rect = s_rect.expand((2.0_f32 * scale).max(1.0_f32));
            painter.rect_filled(bezel_rect, (2.0_f32 * scale).max(1.0_f32), Color32::from_rgb(25, 27, 30));
            painter.rect_stroke(
                bezel_rect,
                (2.0_f32 * scale).max(1.0_f32),
                Stroke::new((1.0_f32 * scale).max(0.75_f32), Color32::from_black_alpha(120)),
            );

            s_rect
        };

        (screen_rect, window_action)
    }
}

#[inline]
fn close_resp_rect(r: Rect) -> Rect {
    r
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAction {
    StartDrag,
    Minimize,
    Maximize,
    Close,
    OpenSettings,
}
