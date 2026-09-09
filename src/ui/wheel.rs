use crate::audio::clicker::{ClickerAudio, ClickerSetting};
use crate::theme::{ChassisColor, CustomThemeConfig};
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelButton {
    Menu,
    PlayPause,
    Previous,
    Next,
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelAction {
    Tick(i32), // +1 CW, -1 CCW
    Click(WheelButton),
    Hold(WheelButton),
}

pub struct ClickWheelWidget {
    last_drag_angle: Option<f32>,
    accumulated_angle: f32,
    pressed_button: Option<WheelButton>,
    press_start_time: Option<f64>,
}

impl ClickWheelWidget {
    pub fn new() -> Self {
        Self {
            last_drag_angle: None,
            accumulated_angle: 0.0_f32,
            pressed_button: None,
            press_start_time: None,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        center: Pos2,
        outer_radius: f32,
        center_radius: f32,
        chassis: ChassisColor,
        custom: &CustomThemeConfig,
        clicker: &ClickerAudio,
        clicker_setting: ClickerSetting,
        is_locked: bool,
        pixel_mode: bool,
    ) -> Vec<WheelAction> {
        let mut actions = Vec::new();

        let wheel_rect = Rect::from_center_size(center, Vec2::splat(outer_radius * 2.0_f32));
        let response = ui.allocate_rect(wheel_rect, Sense::click_and_drag());
        let painter = ui.painter().clone();

        // Colors
        let wheel_bg = chassis.wheel_color_with_custom(custom);
        let wheel_text = chassis.wheel_text_color_with_custom(custom);
        let center_bg = chassis.center_button_color_with_custom(custom);

        let scale = (outer_radius / 101.75_f32).max(0.2_f32);

        if !pixel_mode {
            // 1. Draw Outer Ring Bevel Shadow
            painter.circle_stroke(
                center,
                outer_radius + 0.5_f32 * scale,
                Stroke::new((1.5_f32 * scale).max(0.8_f32), Color32::from_black_alpha(35)),
            );
            // Outer wheel fill
            painter.circle_filled(center, outer_radius, wheel_bg);

            // Subtle wheel brushed texture / border ring
            painter.circle_stroke(
                center,
                outer_radius,
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_white_alpha(30)),
            );
            painter.circle_stroke(
                center,
                outer_radius - 1.0_f32 * scale,
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_black_alpha(20)),
            );

            // 2. Draw Wheel Labels
            let label_dist = (outer_radius + center_radius) * 0.5_f32;

            // MENU (Top)
            let menu_pos = Pos2::new(center.x, center.y - label_dist);
            painter.text(
                menu_pos,
                egui::Align2::CENTER_CENTER,
                "MENU",
                FontId::proportional(12.0_f32 * scale),
                wheel_text,
            );

            // PLAY/PAUSE (Bottom)
            let play_pos = Pos2::new(center.x, center.y + label_dist);
            painter.text(
                play_pos,
                egui::Align2::CENTER_CENTER,
                "▶❙❙",
                FontId::proportional(11.0_f32 * scale),
                wheel_text,
            );

            // PREVIOUS (Left)
            let prev_pos = Pos2::new(center.x - label_dist, center.y);
            painter.text(
                prev_pos,
                egui::Align2::CENTER_CENTER,
                "❙◀◀",
                FontId::proportional(11.0_f32 * scale),
                wheel_text,
            );

            // NEXT (Right)
            let next_pos = Pos2::new(center.x + label_dist, center.y);
            painter.text(
                next_pos,
                egui::Align2::CENTER_CENTER,
                "▶▶❙",
                FontId::proportional(11.0_f32 * scale),
                wheel_text,
            );

            // 3. Draw Center Button
            let center_is_pressed = self.pressed_button == Some(WheelButton::Select);
            let center_actual_bg = if center_is_pressed {
                Color32::from_rgb(
                    (center_bg.r() as f32 * 0.85_f32) as u8,
                    (center_bg.g() as f32 * 0.85_f32) as u8,
                    (center_bg.b() as f32 * 0.85_f32) as u8,
                )
            } else {
                center_bg
            };

            // Center button outer groove shadow
            painter.circle_stroke(
                center,
                center_radius + 0.5_f32 * scale,
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_black_alpha(40)),
            );
            painter.circle_filled(center, center_radius, center_actual_bg);
            painter.circle_stroke(
                center,
                center_radius,
                Stroke::new((1.0_f32 * scale).max(0.5_f32), Color32::from_white_alpha(35)),
            );
        }

        // Pixel Art Theme: the sprite carries the wheel artwork, so we only
        // stamp a chunky translucent press indicator over the active region.
        if pixel_mode {
            if let Some(btn) = self.pressed_button {
                let shade = Color32::from_black_alpha(45);
                if btn == WheelButton::Select {
                    painter.circle_filled(center, center_radius, shade);
                } else {
                    let (a0, a1) = match btn {
                        WheelButton::Menu => (-0.75_f32 * PI, -0.25_f32 * PI),
                        WheelButton::PlayPause => (0.25_f32 * PI, 0.75_f32 * PI),
                        WheelButton::Previous => (0.75_f32 * PI, 1.25_f32 * PI),
                        WheelButton::Next => (-0.25_f32 * PI, 0.25_f32 * PI),
                        WheelButton::Select => unreachable!(),
                    };
                    let steps = 16;
                    let mut pts: Vec<Pos2> = Vec::with_capacity((steps + 1) * 2);
                    for i in 0..=steps {
                        let a = a0 + (a1 - a0) * (i as f32 / steps as f32);
                        pts.push(center + Vec2::angled(a) * outer_radius);
                    }
                    for i in (0..=steps).rev() {
                        let a = a0 + (a1 - a0) * (i as f32 / steps as f32);
                        pts.push(center + Vec2::angled(a) * center_radius);
                    }
                    painter.add(egui::Shape::convex_polygon(pts, shade, Stroke::NONE));
                }
            }
        }

        if is_locked {
            return actions;
        }

        let time_now = ui.input(|i| i.time);

        // 4. Handle Mouse Interactions (Rotary drag + Button clicks)
        if response.dragged() || response.hovered() {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let dx = mouse_pos.x - center.x;
                let dy = mouse_pos.y - center.y;
                let dist = (dx * dx + dy * dy).sqrt();

                // Check Rotary drag in annular region
                if response.dragged() && dist > center_radius && dist <= outer_radius + 15.0_f32 {
                    let angle = dy.atan2(dx);
                    if let Some(last_angle) = self.last_drag_angle {
                        let mut diff = angle - last_angle;
                        if diff > PI {
                            diff -= 2.0_f32 * PI;
                        } else if diff < -PI {
                            diff += 2.0_f32 * PI;
                        }

                        self.accumulated_angle += diff;
                        let tick_step = 16.0_f32 * (PI / 180.0_f32); // 16 degrees per tick

                        while self.accumulated_angle >= tick_step {
                            self.accumulated_angle -= tick_step;
                            actions.push(WheelAction::Tick(1));
                            clicker.play_rotary_tick(clicker_setting);
                        }
                        while self.accumulated_angle <= -tick_step {
                            self.accumulated_angle += tick_step;
                            actions.push(WheelAction::Tick(-1));
                            clicker.play_rotary_tick(clicker_setting);
                        }
                    }
                    self.last_drag_angle = Some(angle);
                } else if !response.dragged() {
                    self.last_drag_angle = None;
                }
            }
        } else {
            self.last_drag_angle = None;
        }

        // Handle Pointer Down / Clicks on Buttons
        if response.is_pointer_button_down_on() {
            if self.pressed_button.is_none() {
                if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
                    let dx = mouse_pos.x - center.x;
                    let dy = mouse_pos.y - center.y;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist <= center_radius {
                        self.pressed_button = Some(WheelButton::Select);
                        self.press_start_time = Some(time_now);
                    } else if dist <= outer_radius + 5.0_f32 {
                        let angle = dy.atan2(dx); // -PI to +PI
                        let button = if angle >= -0.75_f32 * PI && angle <= -0.25_f32 * PI {
                            WheelButton::Menu
                        } else if angle >= 0.25_f32 * PI && angle <= 0.75_f32 * PI {
                            WheelButton::PlayPause
                        } else if angle.abs() > 0.75_f32 * PI {
                            WheelButton::Previous
                        } else {
                            WheelButton::Next
                        };
                        self.pressed_button = Some(button);
                        self.press_start_time = Some(time_now);
                    }
                }
            } else if let Some(btn) = self.pressed_button {
                // Check if button is being held (for fast forward / rewind)
                if let Some(start_t) = self.press_start_time {
                    if time_now - start_t > 0.4 {
                        actions.push(WheelAction::Hold(btn));
                    }
                }
            }
        } else if self.pressed_button.is_some() {
            // Button released!
            if let Some(btn) = self.pressed_button.take() {
                actions.push(WheelAction::Click(btn));
                clicker.play_button_click(clicker_setting);
            }
            self.press_start_time = None;
        }

        // NOTE: Mouse-wheel scrolling is routed centrally in ui/mod.rs
        // (center-button hover = volume, everywhere else = rotary navigation)
        // so the widget itself deliberately emits no scroll actions.

        actions
    }
}
