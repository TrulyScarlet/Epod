use crate::theme::{ChassisColor, CustomThemeConfig};
use egui::ColorImage;

struct PixelPalette {
    steel: [u8; 4],
    steel_edge: [u8; 4],
    face: [u8; 4],
    face_hi: [u8; 4],
    face_sh: [u8; 4],
    dither: [u8; 4],
    frame: [u8; 4],
    frame_hi: [u8; 4],
    wheel: [u8; 4],
    wheel_hi: [u8; 4],
    wheel_sh: [u8; 4],
    wheel_edge: [u8; 4],
    center: [u8; 4],
    center_sh: [u8; 4],
    center_edge: [u8; 4],
    glyph: [u8; 4],
    jack_rim: [u8; 4],
    jack_dark: [u8; 4],
    jack_center: [u8; 4],
    switch_slot: [u8; 4],
    switch_thumb: [u8; 4],
}

impl PixelPalette {
    fn for_chassis(chassis: ChassisColor, custom: &CustomThemeConfig) -> Self {
        match chassis {
            ChassisColor::White | ChassisColor::Unknown => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [245, 246, 248, 255],
                face_hi: [255, 255, 255, 255],
                face_sh: [221, 224, 229, 255],
                dither: [234, 236, 240, 255],
                frame: [35, 37, 42, 255],
                frame_hi: [82, 87, 96, 255],
                wheel: [227, 229, 233, 255],
                wheel_hi: [236, 238, 241, 255],
                wheel_sh: [208, 211, 217, 255],
                wheel_edge: [176, 182, 191, 255],
                center: [250, 251, 253, 255],
                center_sh: [240, 241, 245, 255],
                center_edge: [186, 191, 199, 255],
                glyph: [134, 139, 148, 255],
                jack_rim: [154, 160, 170, 255],
                jack_dark: [30, 32, 36, 255],
                jack_center: [10, 11, 13, 255],
                switch_slot: [58, 63, 72, 255],
                switch_thumb: [206, 211, 219, 255],
            },
            ChassisColor::Black => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [22, 22, 25, 255],
                face_hi: [46, 48, 54, 255],
                face_sh: [14, 14, 16, 255],
                dither: [28, 29, 33, 255],
                frame: [15, 15, 18, 255],
                frame_hi: [42, 44, 50, 255],
                wheel: [45, 46, 50, 255],
                wheel_hi: [60, 62, 68, 255],
                wheel_sh: [34, 35, 38, 255],
                wheel_edge: [18, 19, 21, 255],
                center: [22, 22, 25, 255],
                center_sh: [14, 14, 17, 255],
                center_edge: [36, 38, 42, 255],
                glyph: [180, 182, 190, 255],
                jack_rim: [140, 145, 155, 255],
                jack_dark: [20, 21, 24, 255],
                jack_center: [8, 8, 10, 255],
                switch_slot: [38, 40, 46, 255],
                switch_thumb: [180, 185, 195, 255],
            },
            ChassisColor::U2Edition => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [18, 18, 20, 255],
                face_hi: [42, 44, 50, 255],
                face_sh: [10, 10, 12, 255],
                dither: [24, 24, 28, 255],
                frame: [12, 12, 15, 255],
                frame_hi: [38, 40, 46, 255],
                wheel: [215, 30, 35, 255],
                wheel_hi: [242, 58, 64, 255],
                wheel_sh: [172, 18, 22, 255],
                wheel_edge: [128, 10, 14, 255],
                center: [18, 18, 20, 255],
                center_sh: [12, 12, 14, 255],
                center_edge: [128, 10, 14, 255],
                glyph: [255, 255, 255, 255],
                jack_rim: [140, 145, 155, 255],
                jack_dark: [20, 21, 24, 255],
                jack_center: [8, 8, 10, 255],
                switch_slot: [38, 40, 46, 255],
                switch_thumb: [180, 185, 195, 255],
            },
            ChassisColor::Green => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [88, 172, 108, 255],
                face_hi: [120, 204, 140, 255],
                face_sh: [62, 134, 80, 255],
                dither: [75, 153, 94, 255],
                frame: [30, 60, 40, 255],
                frame_hi: [70, 130, 90, 255],
                wheel: [228, 244, 232, 255],
                wheel_hi: [240, 252, 244, 255],
                wheel_sh: [205, 226, 210, 255],
                wheel_edge: [170, 196, 176, 255],
                center: [88, 172, 108, 255],
                center_sh: [62, 134, 80, 255],
                center_edge: [50, 110, 66, 255],
                glyph: [60, 115, 75, 255],
                jack_rim: [154, 160, 170, 255],
                jack_dark: [30, 32, 36, 255],
                jack_center: [10, 11, 13, 255],
                switch_slot: [45, 85, 55, 255],
                switch_thumb: [206, 211, 219, 255],
            },
            ChassisColor::Red => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [208, 30, 45, 255],
                face_hi: [238, 65, 80, 255],
                face_sh: [160, 18, 30, 255],
                dither: [184, 24, 38, 255],
                frame: [60, 15, 20, 255],
                frame_hi: [120, 35, 45, 255],
                wheel: [248, 248, 250, 255],
                wheel_hi: [255, 255, 255, 255],
                wheel_sh: [224, 224, 228, 255],
                wheel_edge: [190, 190, 196, 255],
                center: [208, 30, 45, 255],
                center_sh: [160, 18, 30, 255],
                center_edge: [130, 12, 22, 255],
                glyph: [180, 30, 42, 255],
                jack_rim: [154, 160, 170, 255],
                jack_dark: [30, 32, 36, 255],
                jack_center: [10, 11, 13, 255],
                switch_slot: [70, 20, 25, 255],
                switch_thumb: [206, 211, 219, 255],
            },
            ChassisColor::Brown => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [105, 68, 54, 255],
                face_hi: [138, 92, 75, 255],
                face_sh: [78, 48, 36, 255],
                dither: [92, 58, 45, 255],
                frame: [40, 25, 20, 255],
                frame_hi: [80, 52, 42, 255],
                wheel: [236, 220, 208, 255],
                wheel_hi: [246, 234, 224, 255],
                wheel_sh: [214, 195, 180, 255],
                wheel_edge: [182, 160, 144, 255],
                center: [105, 68, 54, 255],
                center_sh: [78, 48, 36, 255],
                center_edge: [60, 35, 25, 255],
                glyph: [105, 68, 54, 255],
                jack_rim: [154, 160, 170, 255],
                jack_dark: [30, 32, 36, 255],
                jack_center: [10, 11, 13, 255],
                switch_slot: [50, 32, 25, 255],
                switch_thumb: [206, 211, 219, 255],
            },
            ChassisColor::Yellow => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [238, 172, 34, 255],
                face_hi: [255, 202, 70, 255],
                face_sh: [198, 138, 18, 255],
                dither: [218, 155, 26, 255],
                frame: [70, 48, 10, 255],
                frame_hi: [140, 98, 22, 255],
                wheel: [255, 246, 222, 255],
                wheel_hi: [255, 252, 238, 255],
                wheel_sh: [238, 224, 192, 255],
                wheel_edge: [208, 190, 150, 255],
                center: [238, 172, 34, 255],
                center_sh: [198, 138, 18, 255],
                center_edge: [160, 108, 10, 255],
                glyph: [140, 98, 12, 255],
                jack_rim: [154, 160, 170, 255],
                jack_dark: [30, 32, 36, 255],
                jack_center: [10, 11, 13, 255],
                switch_slot: [80, 56, 12, 255],
                switch_thumb: [206, 211, 219, 255],
            },
            ChassisColor::Blue => Self {
                steel: [196, 201, 209, 255],
                steel_edge: [148, 155, 166, 255],
                face: [42, 118, 202, 255],
                face_hi: [75, 152, 235, 255],
                face_sh: [26, 88, 160, 255],
                dither: [34, 103, 181, 255],
                frame: [15, 40, 75, 255],
                frame_hi: [35, 80, 140, 255],
                wheel: [222, 238, 252, 255],
                wheel_hi: [238, 248, 255, 255],
                wheel_sh: [196, 218, 238, 255],
                wheel_edge: [160, 186, 212, 255],
                center: [42, 118, 202, 255],
                center_sh: [26, 88, 160, 255],
                center_edge: [18, 65, 125, 255],
                glyph: [28, 76, 138, 255],
                jack_rim: [154, 160, 170, 255],
                jack_dark: [30, 32, 36, 255],
                jack_center: [10, 11, 13, 255],
                switch_slot: [25, 55, 95, 255],
                switch_thumb: [206, 211, 219, 255],
            },
            ChassisColor::Custom => {
                let [fr, fg, fb] = custom.shell_color;
                let [wr, wg, wb] = custom.wheel_color;
                let [cr, cg, cb] = custom.center_button_color;
                
                let tint_hi = |r: u8, g: u8, b: u8| -> [u8; 4] {
                    [
                        (r as f32 + (255.0 - r as f32) * 0.25).clamp(0.0, 255.0) as u8,
                        (g as f32 + (255.0 - g as f32) * 0.25).clamp(0.0, 255.0) as u8,
                        (b as f32 + (255.0 - b as f32) * 0.25).clamp(0.0, 255.0) as u8,
                        255,
                    ]
                };
                let tint_sh = |r: u8, g: u8, b: u8| -> [u8; 4] {
                    [
                        (r as f32 * 0.78).clamp(0.0, 255.0) as u8,
                        (g as f32 * 0.78).clamp(0.0, 255.0) as u8,
                        (b as f32 * 0.78).clamp(0.0, 255.0) as u8,
                        255,
                    ]
                };
                let tint_edge = |r: u8, g: u8, b: u8| -> [u8; 4] {
                    [
                        (r as f32 * 0.58).clamp(0.0, 255.0) as u8,
                        (g as f32 * 0.58).clamp(0.0, 255.0) as u8,
                        (b as f32 * 0.58).clamp(0.0, 255.0) as u8,
                        255,
                    ]
                };

                let w_lum = 0.299 * (wr as f32) + 0.587 * (wg as f32) + 0.114 * (wb as f32);
                let glyph_col = if w_lum > 130.0 {
                    [
                        (wr as f32 * 0.45) as u8,
                        (wg as f32 * 0.45) as u8,
                        (wb as f32 * 0.45) as u8,
                        255,
                    ]
                } else {
                    [250, 250, 255, 255]
                };

                Self {
                    steel: [196, 201, 209, 255],
                    steel_edge: [148, 155, 166, 255],
                    face: [fr, fg, fb, 255],
                    face_hi: tint_hi(fr, fg, fb),
                    face_sh: tint_sh(fr, fg, fb),
                    dither: [
                        (fr as f32 * 0.88).clamp(0.0, 255.0) as u8,
                        (fg as f32 * 0.88).clamp(0.0, 255.0) as u8,
                        (fb as f32 * 0.88).clamp(0.0, 255.0) as u8,
                        255,
                    ],
                    frame: [18, 18, 22, 255],
                    frame_hi: [50, 52, 60, 255],
                    wheel: [wr, wg, wb, 255],
                    wheel_hi: tint_hi(wr, wg, wb),
                    wheel_sh: tint_sh(wr, wg, wb),
                    wheel_edge: tint_edge(wr, wg, wb),
                    center: [cr, cg, cb, 255],
                    center_sh: tint_sh(cr, cg, cb),
                    center_edge: tint_edge(cr, cg, cb),
                    glyph: glyph_col,
                    jack_rim: [154, 160, 170, 255],
                    jack_dark: [30, 32, 36, 255],
                    jack_center: [10, 11, 13, 255],
                    switch_slot: [45, 48, 55, 255],
                    switch_thumb: [206, 211, 219, 255],
                }
            }
        }
    }
}

#[inline]
fn put(buf: &mut [u8], w: i32, h: i32, x: i32, y: i32, c: [u8; 4]) {
    if x >= 0 && y >= 0 && x < w && y < h {
        let idx = ((y * w + x) * 4) as usize;
        buf[idx..idx + 4].copy_from_slice(&c);
    }
}

fn corner_cut(row_from_edge: i32) -> i32 {
    match row_from_edge {
        0 => 9,
        1 => 6,
        2 => 4,
        3 => 3,
        4 => 2,
        5 => 1,
        6 => 1,
        _ => 0,
    }
}

fn inside_silhouette(x: i32, y: i32, w: i32, h: i32, inset: i32) -> bool {
    if x < inset || y < inset || x >= w - inset || y >= h - inset {
        return false;
    }
    let lx = x - inset;
    let rx = (w - 1 - inset) - x;
    let ty = y - inset;
    let by = (h - 1 - inset) - y;

    if lx < 7 && ty < 7 && lx < (corner_cut(ty) - inset).max(0) {
        return false;
    }
    if rx < 7 && ty < 7 && rx < (corner_cut(ty) - inset).max(0) {
        return false;
    }
    if lx < 7 && by < 7 && lx < (corner_cut(by) - inset).max(0) {
        return false;
    }
    if rx < 7 && by < 7 && rx < (corner_cut(by) - inset).max(0) {
        return false;
    }
    true
}

const GLYPH_M: [&str; 5] = ["X.X", "XXX", "X.X", "X.X", "X.X"];
const GLYPH_E: [&str; 5] = ["XXX", "X..", "XX.", "X..", "XXX"];
const GLYPH_N: [&str; 5] = ["X.X", "XXX", "XXX", "X.X", "X.X"];
const GLYPH_U: [&str; 5] = ["X.X", "X.X", "X.X", "X.X", "XXX"];
const GLYPH_TRI_R: [&str; 5] = ["X..", "XX.", "XXX", "XX.", "X.."];
const GLYPH_TRI_L: [&str; 5] = ["..X", ".XX", "XXX", ".XX", "..X"];
const GLYPH_BAR: [&str; 5] = ["X", "X", "X", "X", "X"];

fn stamp(buf: &mut [u8], w: i32, h: i32, glyph: &[&str; 5], x: i32, y: i32, c: [u8; 4]) {
    for (row, line) in glyph.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == 'X' {
                put(buf, w, h, x + col as i32, y + row as i32, c);
            }
        }
    }
}

fn draw_circle(buf: &mut [u8], w: i32, h: i32, cx: i32, cy: i32, r: i32, c: [u8; 4]) {
    for y in cy - r..=cy + r {
        for x in cx - r..=cx + r {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r * r {
                put(buf, w, h, x, y, c);
            }
        }
    }
}

pub fn generate_pixel_body(chassis: ChassisColor, custom: &CustomThemeConfig) -> ColorImage {
    let w: i32 = 185;
    let h: i32 = 303;
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let pal = PixelPalette::for_chassis(chassis, custom);

    // 1. Chassis silhouette: steel rim + polycarbonate faceplate
    for y in 0..h {
        for x in 0..w {
            if inside_silhouette(x, y, w, h, 0) {
                let c = if !inside_silhouette(x, y, w, h, 1) {
                    pal.steel_edge
                } else {
                    pal.steel
                };
                put(&mut buf, w, h, x, y, c);
            }
            if inside_silhouette(x, y, w, h, 2) {
                let mut c = pal.face;
                if y == 2 || x == 2 {
                    c = pal.face_hi;
                }
                if y == h - 3 || x == w - 3 {
                    c = pal.face_sh;
                }
                if y > 148 && (x + y) % 2 == 0 && c == pal.face {
                    c = pal.dither;
                }
                put(&mut buf, w, h, x, y, c);
            }
        }
    }

    // 2. Screen bezel frame
    for y in 13..=143 {
        for x in 7..=178 {
            let on_ring = x <= 8 || x >= 177 || y <= 14 || y >= 142;
            if on_ring {
                let c = if y == 13 || x == 7 { pal.frame_hi } else { pal.frame };
                put(&mut buf, w, h, x, y, c);
            } else {
                // Transparent inner cutout so LCD screen renders seamlessly with zero white borders
                put(&mut buf, w, h, x, y, [0, 0, 0, 0]);
            }
        }
    }

    // 3. Click Wheel (center 92,217 | R=51 | r=19)
    let (cx, cy, big_r, small_r) = (92, 217, 51, 19);
    for y in cy - big_r - 1..=cy + big_r + 1 {
        for x in cx - big_r - 1..=cx + big_r + 1 {
            let dx = x - cx;
            let dy = y - cy;
            let d2 = dx * dx + dy * dy;
            if d2 > (big_r + 1) * (big_r + 1) {
                continue;
            }
            let c = if d2 > big_r * big_r {
                pal.wheel_edge
            } else if d2 <= small_r * small_r {
                if d2 > (small_r - 1) * (small_r - 1) {
                    pal.center_edge
                } else if dy > 3 && (x + y) % 2 == 0 {
                    pal.center_sh
                } else {
                    pal.center
                }
            } else if d2 <= (small_r + 2) * (small_r + 2) {
                pal.center_edge
            } else {
                let mut c = pal.wheel;
                if dy < -12 && (x + y) % 2 == 0 {
                    c = pal.wheel_hi;
                }
                if dy > 16 && (x + y) % 2 == 0 {
                    c = pal.wheel_sh;
                }
                c
            };
            put(&mut buf, w, h, x, y, c);
        }
    }

    // 4. Wheel glyphs
    stamp(&mut buf, w, h, &GLYPH_M, 85, cy - 37, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_E, 89, cy - 37, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_N, 93, cy - 37, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_U, 97, cy - 37, pal.glyph);

    stamp(&mut buf, w, h, &GLYPH_TRI_R, 89, cy + 31, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_BAR, 93, cy + 31, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_BAR, 95, cy + 31, pal.glyph);

    stamp(&mut buf, w, h, &GLYPH_BAR, cx - 39, cy - 2, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_TRI_L, cx - 37, cy - 2, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_TRI_L, cx - 33, cy - 2, pal.glyph);

    stamp(&mut buf, w, h, &GLYPH_TRI_R, cx + 31, cy - 2, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_TRI_R, cx + 35, cy - 2, pal.glyph);
    stamp(&mut buf, w, h, &GLYPH_BAR, cx + 39, cy - 2, pal.glyph);

    // 5. Top hardware: 3.5mm jack + hold switch
    draw_circle(&mut buf, w, h, 18, 8, 3, pal.jack_rim);
    draw_circle(&mut buf, w, h, 18, 8, 2, pal.jack_dark);
    put(&mut buf, w, h, 18, 8, pal.jack_center);

    for y in 6..=9 {
        for x in 34..=46 {
            put(&mut buf, w, h, x, y, pal.switch_slot);
        }
    }
    for y in 7..=8 {
        for x in 35..=38 {
            put(&mut buf, w, h, x, y, pal.switch_thumb);
        }
    }

    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &buf)
}
