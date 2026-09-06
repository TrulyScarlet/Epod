use image::{Rgba, RgbaImage};
use image::codecs::ico::{IcoEncoder, IcoFrame};
use std::fs::File;
use std::path::Path;

fn hex(hex: u32) -> Rgba<u8> {
    Rgba([
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
        255,
    ])
}

fn hex_a(hex: u32, a: u8) -> Rgba<u8> {
    Rgba([
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
        a,
    ])
}

struct Canvas {
    img: RgbaImage,
}

impl Canvas {
    fn new(w: u32, h: u32) -> Self {
        Self {
            img: RgbaImage::new(w, h),
        }
    }

    fn put(&mut self, x: i32, y: i32, col: Rgba<u8>) {
        if x >= 0 && x < 32 && y >= 0 && y < 32 {
            self.img.put_pixel(x as u32, y as u32, col);
        }
    }

    fn hline(&mut self, x1: i32, x2: i32, y: i32, col: Rgba<u8>) {
        for x in x1..=x2 {
            self.put(x, y, col);
        }
    }

    fn blend(&mut self, x: i32, y: i32, col: Rgba<u8>) {
        if x >= 0 && x < 32 && y >= 0 && y < 32 {
            let existing = self.img.get_pixel(x as u32, y as u32);
            let alpha = col[3] as f32 / 255.0;
            let inv_alpha = 1.0 - alpha;
            let r = (col[0] as f32 * alpha + existing[0] as f32 * inv_alpha) as u8;
            let g = (col[1] as f32 * alpha + existing[1] as f32 * inv_alpha) as u8;
            let b = (col[2] as f32 * alpha + existing[2] as f32 * inv_alpha) as u8;
            let a = (col[3].max(existing[3])) as u8;
            self.img.put_pixel(x as u32, y as u32, Rgba([r, g, b, a]));
        }
    }
}

pub fn create_master_epod_32() -> RgbaImage {
    let mut c = Canvas::new(32, 32);

    // ----------------------------------------------------
    // COLOR PALETTE - VINTAGE RETRO PIXEL EPOD
    // ----------------------------------------------------
    // Outer Border & Drop Shadow
    let c_outline = hex(0x181c26);          // Main device contour
    let c_outline_corner = hex(0x3a4050);   // Corner antialiased outline
    let c_shadow_1 = hex_a(0x000000, 140);  // Direct drop shadow
    let c_shadow_2 = hex_a(0x000000, 50);   // Ambient drop shadow

    // Device Body (Metallic Pearl White iPod Faceplate)
    let c_body_highlight = hex(0xffffff);    // Specular top & left edge
    let c_body_white = hex(0xf8f9fd);        // Pearl faceplate white
    let c_body_base = hex(0xe8ebf2);         // Metallic off-white base
    let c_body_bevel = hex(0xd5dae6);        // Bevel transition
    let c_body_shadow = hex(0xb2b9c7);       // Right & bottom bevel shadow
    let c_body_dark_shadow = hex(0x8e96a6);  // Bottom corner shadow

    // Retro LCD Screen Frame / Bezel
    let c_bezel_top = hex(0x18241b);         // Bezel top recession shadow
    let c_bezel_side = hex(0x243527);        // Bezel side
    let c_bezel_bot = hex(0x3c5040);         // Bezel bottom rim highlight

    // Backlit Retro LCD Screen (Classic Pale Mint Green)
    let c_lcd_bg_top = hex(0xa8d3ad);        // LCD top backlight glow
    let c_lcd_bg = hex(0x95c39a);            // LCD standard pale mint
    let c_lcd_bg_mid = hex(0x88b58d);        // LCD midtone
    let c_lcd_bg_bot = hex(0x7aa47e);        // LCD bottom gradient

    // Active LCD Pixels (Monochrome vintage forest darks)
    let c_lcd_dark = hex(0x122416);          // Main active LCD pixel (notes, icons, text)
    let c_lcd_mid = hex(0x38563f);           // Secondary LCD pixel (subtext, time)
    let c_lcd_bar_bg = hex(0x608a67);        // Progress bar inactive track
    let c_lcd_bar_fill = hex(0x122416);      // Progress bar active fill

    // Click Wheel Colors (Smooth Shaded Wheel)
    let c_wheel_hi = hex(0xf4f6fa);          // Top-left rim highlight
    let c_wheel_base = hex(0xdce0eb);        // Wheel surface base
    let c_wheel_mid = hex(0xd0d5e2);         // Wheel subtle shading
    let c_wheel_sh = hex(0xbcc3ce);          // Bottom-right rim shadow
    let c_wheel_glyph = hex(0x767f92);       // Crisp slate markings

    // Center Select Button
    let c_btn_hi = hex(0xffffff);            // Center button top gloss
    let c_btn_face = hex(0xf8f9fd);          // Center button pearl face
    let c_btn_sh = hex(0xd0d6e2);            // Center button bottom bevel

    // ----------------------------------------------------
    // 1. DROP SHADOW (Rows 30-31)
    // ----------------------------------------------------
    for x in 8..=23 {
        c.blend(x, 30, c_shadow_1);
    }
    c.blend(7, 30, c_shadow_2);
    c.blend(24, 30, c_shadow_2);
    for x in 10..=21 {
        c.blend(x, 31, c_shadow_2);
    }

    // ----------------------------------------------------
    // 2. DEVICE BODY SILHOUETTE & BASE FILL
    // ----------------------------------------------------
    // Base interior fill (Rows 4..27, X=7..24)
    for y in 4..=27 {
        for x in 7..=24 {
            c.put(x, y, c_body_base);
        }
    }
    // Row 3 inner fill
    for x in 9..=22 {
        c.put(x, 3, c_body_white);
    }
    // Row 28 inner fill
    for x in 9..=22 {
        c.put(x, 28, c_body_shadow);
    }

    // Specular Highlight (Top and Left edge)
    c.hline(9, 22, 3, c_body_highlight);
    for y in 4..=27 {
        c.put(7, y, c_body_highlight);
    }
    // Faceplate top gradient
    for y in 4..=5 {
        for x in 8..=23 {
            c.put(x, y, c_body_white);
        }
    }
    // Right & Bottom bevel shadows
    for y in 4..=27 {
        c.put(24, y, c_body_shadow);
    }
    c.hline(9, 22, 28, c_body_shadow);

    // ----------------------------------------------------
    // 3. OUTER OUTLINE & ROUNDED CORNERS
    // ----------------------------------------------------
    // Row 2 (Top edge)
    c.put(8, 2, c_outline_corner);
    c.hline(9, 22, 2, c_outline);
    c.put(23, 2, c_outline_corner);

    // Row 3 (Top corners)
    c.put(7, 3, c_outline);
    c.put(8, 3, c_body_highlight);
    c.put(23, 3, c_body_bevel);
    c.put(24, 3, c_outline);

    // Rows 4..27 (Side outlines)
    for y in 4..=27 {
        c.put(6, y, c_outline);
        c.put(25, y, c_outline);
    }

    // Row 28 (Bottom corners)
    c.put(7, 28, c_outline);
    c.put(8, 28, c_body_shadow);
    c.put(23, 28, c_body_dark_shadow);
    c.put(24, 28, c_outline);

    // Row 29 (Bottom edge)
    c.put(8, 29, c_outline_corner);
    c.hline(9, 22, 29, c_outline);
    c.put(23, 29, c_outline_corner);

    // ----------------------------------------------------
    // 4. RETRO LCD SCREEN (Bezel: X=8..23, Y=5..14; LCD: X=9..22, Y=6..13)
    // ----------------------------------------------------
    // Screen Bezel Outer Frame
    c.hline(8, 23, 5, c_bezel_top);
    for y in 6..=13 {
        c.put(8, y, c_bezel_side);
        c.put(23, y, c_bezel_side);
    }
    c.hline(8, 23, 14, c_bezel_bot);

    // LCD Screen Background Fill (with vertical backlight gradient)
    for x in 9..=22 {
        c.put(x, 6, c_lcd_bg_top);
        c.put(x, 7, c_lcd_bg);
        c.put(x, 8, c_lcd_bg);
        c.put(x, 9, c_lcd_bg);
        c.put(x, 10, c_lcd_bg_mid);
        c.put(x, 11, c_lcd_bg_mid);
        c.put(x, 12, c_lcd_bg_bot);
        c.put(x, 13, c_lcd_bg_bot);
    }

    // LCD Content (14x8 pixel area: X=9..22, Y=6..13):
    // --- Row 6: Status Bar (Play ▶, Center Title Dots, Battery [■]) ---
    c.put(9, 6, c_lcd_dark);  // Play ▶
    c.put(10, 6, c_lcd_dark);
    // Center title dot-matrix "EPOD"
    c.put(13, 6, c_lcd_mid);
    c.put(15, 6, c_lcd_mid);
    c.put(17, 6, c_lcd_mid);
    // Battery [■].
    c.put(19, 6, c_lcd_mid);  // [
    c.put(20, 6, c_lcd_dark); // ■
    c.put(21, 6, c_lcd_mid);  // ]
    c.put(22, 6, c_lcd_mid);  // tip

    // --- Rows 7..10: Crisp Beamed Music Note ♫ & Track Info ---
    // Beam at Row 7 (X=11..16):
    c.hline(11, 16, 7, c_lcd_dark);
    
    // Row 8: Stems (X=11, 16) & Track title line (X=18..21)
    c.put(11, 8, c_lcd_dark); // Left stem
    c.put(16, 8, c_lcd_dark); // Right stem
    c.hline(18, 21, 8, c_lcd_dark); // Track title

    // Row 9: Note heads upper + stems & Artist line
    c.put(9, 9, c_lcd_dark);  // Left head upper
    c.put(10, 9, c_lcd_dark);
    c.put(11, 9, c_lcd_dark); // Left stem join
    
    c.put(14, 9, c_lcd_dark); // Right head upper
    c.put(15, 9, c_lcd_dark);
    c.put(16, 9, c_lcd_dark); // Right stem join

    c.put(18, 9, c_lcd_mid);  // Artist line
    c.put(19, 9, c_lcd_mid);
    c.put(20, 9, c_lcd_mid);

    // Row 10: Note heads lower (X=9..11, X=14..16)
    c.hline(9, 11, 10, c_lcd_dark);  // Left note head bottom
    c.hline(14, 16, 10, c_lcd_dark); // Right note head bottom

    // --- Row 12: Playback Progress Bar ---
    c.hline(10, 21, 12, c_lcd_bar_bg);  // Full track bar
    c.hline(10, 14, 12, c_lcd_bar_fill); // Elapsed fill
    c.put(14, 12, c_lcd_dark);          // Scrubber knob

    // ----------------------------------------------------
    // 5. CLICK WHEEL (Perfect Smooth Disc: Diameter 12, X=10..21, Y=16..27)
    // ----------------------------------------------------
    // Row 16 (X=14..17: 4 px top highlight)
    c.hline(14, 17, 16, c_wheel_hi);

    // Row 17 (X=12..19: 8 px)
    c.put(12, 17, c_wheel_hi);
    c.put(13, 17, c_wheel_hi);
    c.hline(14, 17, 17, c_wheel_base);
    c.put(18, 17, c_wheel_base);
    c.put(19, 17, c_wheel_mid);

    // Row 18 (X=11..20: 10 px)
    c.put(11, 18, c_wheel_hi);
    c.put(12, 18, c_wheel_hi);
    c.hline(13, 18, 18, c_wheel_base);
    c.put(19, 18, c_wheel_mid);
    c.put(20, 18, c_wheel_sh);

    // Rows 19..24 (X=10..21: 12 px full width)
    for y in 19..=24 {
        c.put(10, y, c_wheel_hi);
        c.put(11, y, c_wheel_base);
        c.hline(12, 19, y, c_wheel_base);
        c.put(20, y, c_wheel_mid);
        c.put(21, y, c_wheel_sh);
    }

    // Row 25 (X=11..20: 10 px)
    c.put(11, 25, c_wheel_base);
    c.hline(12, 18, 25, c_wheel_mid);
    c.put(19, 25, c_wheel_sh);
    c.put(20, 25, c_wheel_sh);

    // Row 26 (X=12..19: 8 px)
    c.put(12, 26, c_wheel_mid);
    c.put(13, 26, c_wheel_mid);
    c.hline(14, 17, 26, c_wheel_sh);
    c.put(18, 26, c_wheel_sh);
    c.put(19, 26, c_wheel_sh);

    // Row 27 (X=14..17: 4 px bottom shadow)
    c.hline(14, 17, 27, c_wheel_sh);

    // ----------------------------------------------------
    // 6. CENTER SELECT BUTTON (Smooth Button: X=14..17, Y=20..23)
    // ----------------------------------------------------
    // Row 20
    c.put(14, 20, c_btn_hi);
    c.put(15, 20, c_btn_hi);
    c.put(16, 20, c_btn_hi);
    c.put(17, 20, c_btn_face);

    // Row 21
    c.put(13, 21, c_btn_hi);
    c.put(14, 21, c_btn_hi);
    c.put(15, 21, c_btn_face);
    c.put(16, 21, c_btn_face);
    c.put(17, 21, c_btn_sh);
    c.put(18, 21, c_btn_sh);

    // Row 22
    c.put(13, 22, c_btn_hi);
    c.put(14, 22, c_btn_face);
    c.put(15, 22, c_btn_face);
    c.put(16, 22, c_btn_sh);
    c.put(17, 22, c_btn_sh);
    c.put(18, 22, c_btn_sh);

    // Row 23
    c.put(14, 23, c_btn_sh);
    c.put(15, 23, c_btn_sh);
    c.put(16, 23, c_btn_sh);
    c.put(17, 23, c_btn_sh);

    // ----------------------------------------------------
    // 7. CLICK WHEEL GLYPHS (Refined subtle slate #767f92)
    // ----------------------------------------------------
    // TOP: "MENU" (Row 17, X=14..17)
    c.hline(14, 17, 17, c_wheel_glyph);

    // BOTTOM: Play/Pause ▶❚❚ (Row 26, X=14..17)
    c.put(14, 26, c_wheel_glyph); // Play ▶
    c.put(15, 26, c_wheel_glyph);
    c.put(17, 26, c_wheel_glyph); // Pause ❚❚

    // LEFT: Previous |◀◀ (Row 21..22, X=11)
    c.put(11, 21, c_wheel_glyph);
    c.put(11, 22, c_wheel_glyph);

    // RIGHT: Next ▶▶| (Row 21..22, X=20)
    c.put(20, 21, c_wheel_glyph);
    c.put(20, 22, c_wheel_glyph);

    c.img
}

fn upscale_nearest(src: &RgbaImage, target_size: u32) -> RgbaImage {
    let mut dst = RgbaImage::new(target_size, target_size);
    let scale = target_size / src.width();
    
    for y in 0..target_size {
        for x in 0..target_size {
            let src_x = (x / scale).min(src.width() - 1);
            let src_y = (y / scale).min(src.height() - 1);
            let pixel = src.get_pixel(src_x, src_y);
            dst.put_pixel(x, y, *pixel);
        }
    }
    dst
}

fn main() {
    println!("Generating Epod pixel art icons...");
    let assets_dir = Path::new("assets");
    if !assets_dir.exists() {
        std::fs::create_dir_all(assets_dir).unwrap();
    }

    let master_32 = create_master_epod_32();

    // 1. Generate 256x256 icon.png
    let icon_256 = upscale_nearest(&master_32, 256);
    let png_path = assets_dir.join("icon.png");
    icon_256.save(&png_path).expect("Failed to save icon.png");
    println!("Saved: {:?}", png_path);

    // 2. Generate multi-resolution icon.ico (16, 32, 48, 64, 128, 256)
    let sizes = [16, 32, 48, 64, 128, 256];
    let mut frames = Vec::new();

    for &size in &sizes {
        let img = if size == 32 {
            master_32.clone()
        } else if size % 32 == 0 {
            upscale_nearest(&master_32, size)
        } else {
            image::imageops::resize(&master_32, size, size, image::imageops::FilterType::Nearest)
        };
        let frame = IcoFrame::as_png(&img, size, size, image::ExtendedColorType::Rgba8)
            .expect("Failed to create IcoFrame");
        frames.push(frame);
    }

    let ico_path = assets_dir.join("icon.ico");
    let mut ico_file = File::create(&ico_path).expect("Failed to create icon.ico");
    let encoder = IcoEncoder::new(&mut ico_file);
    encoder.encode_images(&frames).expect("Failed to encode ICO");
    println!("Saved: {:?}", ico_path);
    println!("Icon generation complete!");
}
