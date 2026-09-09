use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::HashMap;

pub struct ArtCache {
    textures: HashMap<usize, TextureHandle>,
    bg_textures: HashMap<(usize, u8), TextureHandle>,
    playlist_textures: HashMap<String, TextureHandle>,
}

impl ArtCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            bg_textures: HashMap::new(),
            playlist_textures: HashMap::new(),
        }
    }

    /// Convert an image to RGBA, downscaling anything larger than 700px to
    /// keep GPU memory usage and decode cost tiny (album art never renders bigger).
    fn to_rgba_capped(img: image::DynamicImage) -> image::RgbaImage {
        let max_side = img.width().max(img.height());
        if max_side > 700 {
            let scale = 700.0_f32 / max_side as f32;
            let nw = ((img.width() as f32 * scale).round() as u32).max(1);
            let nh = ((img.height() as f32 * scale).round() as u32).max(1);
            img.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
                .to_rgba8()
        } else {
            img.to_rgba8()
        }
    }

    pub fn get_or_load_playlist_cover(
        &mut self,
        ctx: &Context,
        playlist_id: &str,
        custom_path: Option<&std::path::Path>,
        fallback_song_bytes: Option<&[u8]>,
        fallback_song_path: Option<&std::path::Path>,
    ) -> Option<&TextureHandle> {
        if self.playlist_textures.contains_key(playlist_id) {
            return self.playlist_textures.get(playlist_id);
        }

        // 1. Try custom path
        if let Some(path) = custom_path {
            if path.exists() {
                if let Ok(dyn_img) = image::open(path) {
                    let rgba = Self::to_rgba_capped(dyn_img);
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let texture = ctx.load_texture(
                        format!("pl_art_{}", playlist_id),
                        color_image,
                        TextureOptions::LINEAR,
                    );
                    self.playlist_textures.insert(playlist_id.to_string(), texture);
                    return self.playlist_textures.get(playlist_id);
                }
            }
        }

        // 2. Fall back to first song's embedded artwork
        if let Some(bytes) = fallback_song_bytes {
            if let Ok(dyn_img) = image::load_from_memory(bytes) {
                let rgba = Self::to_rgba_capped(dyn_img);
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture = ctx.load_texture(
                    format!("pl_art_{}", playlist_id),
                    color_image,
                    TextureOptions::LINEAR,
                );
                self.playlist_textures.insert(playlist_id.to_string(), texture);
                return self.playlist_textures.get(playlist_id);
            }
        }

        // 3. Fall back to extracting artwork on demand from first song's file path
        if let Some(path) = fallback_song_path {
            if let Some(bytes) = crate::audio::metadata::TrackMetadata::extract_artwork(path) {
                if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                    let rgba = Self::to_rgba_capped(dyn_img);
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let texture = ctx.load_texture(
                        format!("pl_art_{}", playlist_id),
                        color_image,
                        TextureOptions::LINEAR,
                    );
                    self.playlist_textures.insert(playlist_id.to_string(), texture);
                    return self.playlist_textures.get(playlist_id);
                }
            }
        }

        None
    }

    pub fn invalidate_playlist_cover(&mut self, playlist_id: &str) {
        self.playlist_textures.remove(playlist_id);
    }

    pub fn get_or_load(
        &mut self,
        ctx: &Context,
        song_id: usize,
        artwork_bytes: Option<&[u8]>,
        file_path: Option<&std::path::Path>,
    ) -> Option<&TextureHandle> {
        if self.textures.contains_key(&song_id) {
            return self.textures.get(&song_id);
        }

        if let Some(bytes) = artwork_bytes {
            if let Ok(dyn_img) = image::load_from_memory(bytes) {
                let rgba = Self::to_rgba_capped(dyn_img);
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture = ctx.load_texture(format!("art_{}", song_id), color_image, TextureOptions::LINEAR);
                self.textures.insert(song_id, texture);
                return self.textures.get(&song_id);
            }
        } else if let Some(path) = file_path {
            if let Some(bytes) = crate::audio::metadata::TrackMetadata::extract_artwork(path) {
                if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                    let rgba = Self::to_rgba_capped(dyn_img);
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let texture = ctx.load_texture(format!("art_{}", song_id), color_image, TextureOptions::LINEAR);
                    self.textures.insert(song_id, texture);
                    return self.textures.get(&song_id);
                }
            }
        }

        None
    }

    pub fn get_or_load_background(
        &mut self,
        ctx: &Context,
        song_id: usize,
        artwork_bytes: Option<&[u8]>,
        file_path: Option<&std::path::Path>,
        blur_radius: u8,
        seed: usize,
    ) -> Option<&TextureHandle> {
        let key = (song_id, blur_radius);
        if self.bg_textures.contains_key(&key) {
            return self.bg_textures.get(&key);
        }

        let target_w = 320u32;
        let target_h = 240u32;

        let loaded_bytes = artwork_bytes.map(|b| b.to_vec()).or_else(|| {
            file_path.and_then(|p| crate::audio::metadata::TrackMetadata::extract_artwork(p))
        });

        let (base_rgba, is_procedural) = if let Some(ref bytes) = loaded_bytes {
            if let Ok(dyn_img) = image::load_from_memory(bytes) {
                let (w, h) = (dyn_img.width(), dyn_img.height());
                let target_ratio = target_w as f32 / target_h as f32; // 4:3
                let current_ratio = w as f32 / h.max(1) as f32;

                let (crop_x, crop_y, crop_w, crop_h) = if current_ratio > target_ratio {
                    let target_cw = (h as f32 * target_ratio).round() as u32;
                    let offset_x = (w.saturating_sub(target_cw)) / 2;
                    (offset_x, 0, target_cw.min(w), h)
                } else {
                    let target_ch = (w as f32 / target_ratio).round() as u32;
                    let offset_y = (h.saturating_sub(target_ch)) / 2;
                    (0, offset_y, w, target_ch.min(h))
                };

                let cropped = dyn_img.crop_imm(crop_x, crop_y, crop_w.max(1), crop_h.max(1));
                (
                    cropped
                        .resize_exact(target_w, target_h, image::imageops::FilterType::Triangle)
                        .to_rgba8(),
                    false,
                )
            } else {
                (Self::generate_procedural_background(target_w, target_h, seed), true)
            }
        } else {
            (Self::generate_procedural_background(target_w, target_h, seed), true)
        };

        let processed = if blur_radius > 0 && !is_procedural {
            image::imageops::blur(&base_rgba, blur_radius as f32)
        } else {
            base_rgba
        };

        let size = [processed.width() as usize, processed.height() as usize];
        let pixels = processed.into_raw();
        let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
        let tex_options = if is_procedural {
            TextureOptions::NEAREST
        } else {
            TextureOptions::LINEAR
        };
        let texture = ctx.load_texture(
            format!("bg_art_{}_{}", song_id, blur_radius),
            color_image,
            tex_options,
        );
        self.bg_textures.insert(key, texture);
        self.bg_textures.get(&key)
    }

    fn generate_procedural_background(w: u32, h: u32, _seed: usize) -> image::RgbaImage {
        let mut img = image::RgbaImage::new(w, h);
        let wf = w as f32;
        let hf = h as f32;

        // 4x4 Bayer Matrix for authentic retro micro-dithering
        const BAYER_4X4: [[f32; 4]; 4] = [
            [ 0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0],
            [12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0],
            [ 3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0],
            [15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0],
        ];

        // 2x2 micro-pixel blocks for authentic retro pixel aesthetic
        let block_size = 2u32;

        for y in 0..h {
            let by = (y / block_size) as usize;
            let v = y as f32 / hf;
            for x in 0..w {
                let bx = (x / block_size) as usize;
                let u = x as f32 / wf;

                // Subtle smooth diagonal luminance gradient across screen
                // (dark grey retro matrix so cream/white text stays readable)
                let diag = u * 0.5 + v * 0.5;
                let base_lum = 0.36 - 0.12 * diag;

                // Subtle wave texture for tactile retro variation
                let wave = ((bx as f32 * 0.08).sin() * (by as f32 * 0.08).cos()) * 0.03;
                let lum = (base_lum + wave).clamp(0.20, 0.44);

                // Dither calculation
                let dither_val = BAYER_4X4[by % 4][bx % 4];
                let stepped_val = lum + (dither_val - 0.5) * 0.10;

                // Authentic retro pixel grid dot
                let is_dot = (bx % 2 == 0) && (by % 2 == 0);

                let (r, g, b) = if stepped_val > 0.38 {
                    if is_dot { (106, 110, 118) } else { (97, 101, 109) }
                } else if stepped_val > 0.32 {
                    if is_dot { (88, 92, 100) } else { (80, 84, 92) }
                } else if stepped_val > 0.26 {
                    if is_dot { (70, 74, 82) } else { (63, 67, 75) }
                } else {
                    if is_dot { (56, 60, 68) } else { (50, 54, 62) }
                };

                img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
            }
        }
        img
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.textures.clear();
        self.bg_textures.clear();
    }
}
