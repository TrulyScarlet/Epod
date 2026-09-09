use crate::audio::player::AudioPlayer;
use crate::library::model::{Library, Song};
use crate::state::{AppState, NowPlayingSubState};
use crate::theme::{ChassisColor, ColorTarget, LcdPalette};
use crate::ui::art_cache::ArtCache;
use crate::ui::lcd::LcdRenderer;
use crate::ui::views::menu_view::draw_procedural_album_art;
use egui::{Color32, Context, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub struct NowPlayingView;

/// Bottom shuffle toggle hotzone (prominent pill button), shared with the click dispatcher.
pub fn shuffle_btn_rect(screen_rect: Rect, scale: f32) -> Rect {
    let btn_h = 16.0_f32 * scale;
    let btn_w = 66.0_f32 * scale;
    let gap = 10.0_f32 * scale;
    let btn_y = screen_rect.max.y - 19.0_f32 * scale;
    let center_x = screen_rect.center().x;
    Rect::from_min_size(
        Pos2::new(center_x - btn_w - gap * 0.5_f32, btn_y),
        Vec2::new(btn_w, btn_h),
    )
}

/// Bottom repeat toggle hotzone (prominent pill button), shared with the click dispatcher.
pub fn repeat_btn_rect(screen_rect: Rect, scale: f32) -> Rect {
    let btn_h = 16.0_f32 * scale;
    let btn_w = 66.0_f32 * scale;
    let gap = 10.0_f32 * scale;
    let btn_y = screen_rect.max.y - 19.0_f32 * scale;
    let center_x = screen_rect.center().x;
    Rect::from_min_size(
        Pos2::new(center_x + gap * 0.5_f32, btn_y),
        Vec2::new(btn_w, btn_h),
    )
}

fn draw_transport_toggle(
    painter: &Painter,
    rect: Rect,
    icon: &str,
    label: &str,
    is_active: bool,
    is_dark: bool,
    scale: f32,
) {
    let accent_blue = Color32::from_rgb(40, 130, 240);
    let bg = if is_active {
        accent_blue
    } else if is_dark {
        Color32::from_white_alpha(20)
    } else {
        Color32::from_black_alpha(18)
    };
    painter.rect_filled(rect, 4.0_f32 * scale, bg);

    let border = if is_active {
        Color32::from_white_alpha(140)
    } else if is_dark {
        Color32::from_white_alpha(35)
    } else {
        Color32::from_black_alpha(30)
    };
    painter.rect_stroke(rect, 4.0_f32 * scale, Stroke::new((0.7_f32 * scale).max(0.5_f32), border));

    let text_col = if is_active {
        Color32::WHITE
    } else {
        LcdPalette::text_secondary(is_dark)
    };

    let text = format!("{} {}", icon, label);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        &text,
        FontId::proportional(8.5_f32 * scale),
        text_col,
    );
}

impl NowPlayingView {
    pub fn render(
        painter: &Painter,
        ctx: &Context,
        screen_rect: Rect,
        state: &mut AppState,
        library: &mut Library,
        player: &AudioPlayer,
        art_cache: &mut ArtCache,
    ) {
        let scale = (screen_rect.width() / 333.0_f32).max(0.2_f32);
        let song_opt = state
            .current_queue
            .get(state.current_queue_idx)
            .and_then(|&id| library.songs.get(id))
            .cloned();

        let has_song_cover = song_opt.as_ref().is_some_and(|s| !s.is_synthetic_demo);
        let is_dark = state.display_theme.is_dark() || (state.display_theme.is_album_cover() && has_song_cover);

        let total_queue = state.current_queue.len().max(1);
        let current_num = state.current_queue_idx + 1;
        let header_title = format!("{} of {}", current_num, total_queue);

        let update_available = matches!(state.update_status, crate::updater::UpdateStatus::UpdateAvailable { .. });
        LcdRenderer::draw_status_bar_with_update(painter, screen_rect, &header_title, player, state.is_hold_locked, state.display_theme, update_available);

        let bar_h = 20.0_f32 * scale;
        let content_rect = Rect::from_min_max(
            Pos2::new(screen_rect.min.x, screen_rect.min.y + bar_h),
            screen_rect.max,
        );

        // Prominent Shuffle / Repeat transport pill buttons at the bottom of Now Playing
        let rep_label = match player.repeat {
            crate::audio::player::RepeatMode::One => "1",
            _ => "Repeat",
        };
        draw_transport_toggle(
            painter,
            shuffle_btn_rect(screen_rect, scale),
            "🔀",
            "Shuffle",
            player.shuffle != crate::audio::player::ShuffleMode::Off,
            is_dark,
            scale,
        );
        draw_transport_toggle(
            painter,
            repeat_btn_rect(screen_rect, scale),
            "↻",
            rep_label,
            player.repeat != crate::audio::player::RepeatMode::Off,
            is_dark,
            scale,
        );

        let Some(song) = song_opt else {
            painter.text(
                content_rect.center(),
                egui::Align2::CENTER_CENTER,
                "No Track Playing",
                FontId::proportional(14.0_f32 * scale),
                LcdPalette::text_secondary(is_dark),
            );
            return;
        };

        let (title_col, artist_col) = if state.chassis_color == ChassisColor::Custom {
            (
                state.custom_theme.get_color32(ColorTarget::TitleText),
                state.custom_theme.get_color32(ColorTarget::ArtistText),
            )
        } else if state.display_theme.is_album_cover() {
            // Warm ivory / cream ("Humpty") & muted warm stone ("Mitski")
            (
                LcdPalette::ALBUM_COVER_TEXT_PRIMARY,
                LcdPalette::ALBUM_COVER_TEXT_SECONDARY,
            )
        } else {
            // Warm butter / popcorn yellow info typography
            (
                Color32::from_rgb(255, 202, 48),
                Color32::from_rgb(255, 220, 118),
            )
        };

        let progress_col = if state.chassis_color == ChassisColor::Custom {
            state.custom_theme.get_color32(ColorTarget::ProgressBar)
        } else {
            LcdPalette::SCRUBBER_BAR_FILL
        };

    let is_album_cover = state.display_theme.is_album_cover();

    match state.now_playing_substate {
        NowPlayingSubState::Standard => {
            render_standard_view(painter, ctx, content_rect, &song, player, art_cache, is_dark, is_album_cover, title_col, artist_col, progress_col, scale);
        }
        NowPlayingSubState::FullArtwork => {
            render_full_artwork_view(painter, ctx, content_rect, &song, player, art_cache, is_album_cover, scale);
        }
        NowPlayingSubState::Lyrics => {
            render_lyrics_view(painter, content_rect, &song, state, player, is_dark, scale);
        }
        NowPlayingSubState::Visualizer => {
            render_visualizer_view(painter, ctx, content_rect, &song, player, is_dark, is_album_cover, scale);
        }
    }
    }
}

fn render_standard_view(
    painter: &Painter,
    ctx: &Context,
    rect: Rect,
    song: &Song,
    player: &AudioPlayer,
    art_cache: &mut ArtCache,
    is_dark: bool,
    is_album_cover: bool,
    title_col: Color32,
    artist_col: Color32,
    progress_col: Color32,
    scale: f32,
) {
    let margin = 14.0_f32 * scale;
    let bar_y = rect.max.y - 38.0_f32 * scale;
    let avail_h = (bar_y - rect.min.y - margin * 1.2_f32).max(30.0_f32);
    let art_size = (110.0_f32 * scale).min(rect.width() * 0.45_f32).min(avail_h);
    let art_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + margin, rect.min.y + margin),
        Vec2::splat(art_size),
    );

    // Drop shadow
    painter.rect_filled(art_rect.expand(2.0_f32 * scale), 3.0_f32 * scale, Color32::from_black_alpha(30));

    // Try loading and rendering real album artwork
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

    // Track Metadata (Right side)
    let text_x = art_rect.max.x + margin;
    let text_w = (rect.max.x - margin - text_x).max(20.0_f32);
    let time = ctx.input(|i| i.time);

    let title_h = 22.0_f32 * scale;
    let artist_h = 18.0_f32 * scale;
    let album_h = 16.0_f32 * scale;
    let info_h = 14.0_f32 * scale;
    let total_text_h = title_h + artist_h + album_h + info_h + 10.0_f32 * scale;

    let title_y = rect.min.y + margin + ((art_size - total_text_h) * 0.5_f32).max(0.0_f32);
    let artist_y = title_y + title_h + 4.0_f32 * scale;
    let album_y = artist_y + artist_h + 3.0_f32 * scale;
    let info_y = album_y + album_h + 3.0_f32 * scale;

    let title_rect = Rect::from_min_size(Pos2::new(text_x, title_y), Vec2::new(text_w, title_h));
    let artist_rect = Rect::from_min_size(Pos2::new(text_x, artist_y), Vec2::new(text_w, artist_h));
    let album_rect = Rect::from_min_size(Pos2::new(text_x, album_y), Vec2::new(text_w, album_h));
    let info_rect = Rect::from_min_size(Pos2::new(text_x, info_y), Vec2::new(text_w, info_h));

    // Title (Bold with ticker)
    LcdRenderer::render_scrolling_text(
        painter,
        ctx,
        &song.title,
        FontId::proportional(13.5_f32 * scale),
        title_col,
        title_rect,
        time,
        egui::Align2::LEFT_CENTER,
    );

    // Artist (with rotating multi-artist & ticker)
    let display_artist = LcdRenderer::get_display_artist(&song.artist, time);
    LcdRenderer::render_scrolling_text(
        painter,
        ctx,
        &display_artist,
        FontId::proportional(11.5_f32 * scale),
        artist_col,
        artist_rect,
        time + 1.0,
        egui::Align2::LEFT_CENTER,
    );

    // Album (with ticker)
    let album_col = if is_album_cover {
        Color32::from_rgb(195, 190, 175)
    } else {
        Color32::from_rgb(235, 180, 60)
    };
    LcdRenderer::render_scrolling_text(
        painter,
        ctx,
        &song.album,
        FontId::proportional(10.5_f32 * scale),
        album_col,
        album_rect,
        time + 2.0,
        egui::Align2::LEFT_CENTER,
    );

    // Format & Bitrate line (MP3 / FLAC + kbps)
    let info_label = if song.is_synthetic_demo {
        "Demo Track • Synth".to_string()
    } else {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref codec) = song.codec {
            parts.push(codec.clone());
        }
        if let Some(kbps) = song.bitrate_kbps {
            parts.push(format!("{} kbps", kbps));
        }
        parts.join(" • ")
    };
    if !info_label.is_empty() {
        let info_col = if is_album_cover {
            Color32::from_rgb(160, 155, 140)
        } else {
            Color32::from_rgb(255, 230, 150)
        };
        LcdRenderer::render_scrolling_text(
            painter,
            ctx,
            &info_label,
            FontId::proportional(9.5_f32 * scale),
            info_col,
            info_rect,
            time + 3.0,
            egui::Align2::LEFT_CENTER,
        );
    }

    // Bottom Progress Bar
    let bar_y = rect.max.y - 42.0_f32 * scale;
    let bar_x_start = rect.min.x + margin;
    let bar_w = rect.width() - margin * 2.0_f32;
    let bar_h = 6.0_f32 * scale;

    let bar_rect = Rect::from_min_size(Pos2::new(bar_x_start, bar_y), Vec2::new(bar_w, bar_h));

    // Background track
    let track_bg = if is_dark { Color32::from_white_alpha(70) } else { LcdPalette::SCRUBBER_BAR_BG };
    painter.rect_filled(bar_rect, 3.0_f32 * scale, track_bg);
    painter.rect_stroke(bar_rect, 3.0_f32 * scale, Stroke::new(0.5_f32 * scale, Color32::from_black_alpha(40)));

    let progress = if song.duration_sec > 0.0 {
        (player.current_time_sec / song.duration_sec).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let fill_w = bar_w * progress;
    let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_h));
    painter.rect_filled(fill_rect, 3.0_f32 * scale, progress_col);

    // Progress dot cursor
    let cursor_x = bar_rect.min.x + fill_w;
    let cursor_y = bar_rect.center().y;
    painter.circle_filled(Pos2::new(cursor_x, cursor_y), 3.2_f32 * scale, progress_col);

    // Time Labels
    let elapsed_str = format_time(player.current_time_sec);
    let remaining_sec = (song.duration_sec - player.current_time_sec).max(0.0);
    let remaining_str = format!("-{}", format_time(remaining_sec));

    let time_y = bar_y + bar_h + 4.0_f32 * scale;
    let time_col = if is_album_cover {
        LcdPalette::ALBUM_COVER_TEXT_SECONDARY
    } else {
        Color32::from_rgb(230, 185, 70)
    };
    painter.text(
        Pos2::new(bar_x_start, time_y),
        egui::Align2::LEFT_TOP,
        elapsed_str,
        FontId::proportional(10.5_f32 * scale),
        time_col,
    );

    painter.text(
        Pos2::new(rect.max.x - margin, time_y),
        egui::Align2::RIGHT_TOP,
        remaining_str,
        FontId::proportional(10.5_f32 * scale),
        time_col,
    );
}

fn render_full_artwork_view(
    painter: &Painter,
    ctx: &Context,
    rect: Rect,
    song: &Song,
    _player: &AudioPlayer,
    art_cache: &mut ArtCache,
    is_album_cover: bool,
    scale: f32,
) {
    let caption_h = 32.0_f32 * scale;
    let max_art_h = (rect.height() - caption_h - 16.0_f32 * scale).max(20.0_f32);
    let max_art_w = (rect.width() - 24.0_f32 * scale).max(20.0_f32);
    let art_size = max_art_h.min(max_art_w);
    let art_center_y = rect.min.y + (rect.height() - caption_h) * 0.5_f32;
    let art_rect = Rect::from_center_size(
        Pos2::new(rect.center().x, art_center_y),
        Vec2::splat(art_size),
    );

    painter.rect_filled(art_rect.expand(3.0_f32 * scale), 4.0_f32 * scale, Color32::from_black_alpha(35));

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

    // Overlay Bottom Caption
    let caption_bg = Rect::from_min_max(
        Pos2::new(rect.min.x, rect.max.y - caption_h),
        rect.max,
    );
    painter.rect_filled(caption_bg, 0.0_f32, Color32::from_black_alpha(160));

    let time = ctx.input(|i| i.time);
    let display_artist = LcdRenderer::get_display_artist(&song.artist, time);
    let caption = format!("{} — {}", song.title, display_artist);
    let caption_col = if is_album_cover {
        LcdPalette::ALBUM_COVER_TEXT_PRIMARY
    } else {
        Color32::from_rgb(255, 220, 118)
    };
    LcdRenderer::render_scrolling_text(
        painter,
        ctx,
        &caption,
        FontId::proportional(11.0_f32 * scale),
        caption_col,
        caption_bg.shrink2(Vec2::new(8.0_f32 * scale, 0.0_f32)),
        time,
        egui::Align2::CENTER_CENTER,
    );
}

/// Draw text with a faux-bold double-strike (egui default fonts have no bold variant).
fn draw_bold_text(painter: &Painter, pos: Pos2, align: egui::Align2, text: &str, font: FontId, color: Color32, scale: f32) {
    let offset = (0.45_f32 * scale).max(0.35_f32);
    painter.text(Pos2::new(pos.x - offset, pos.y), align, text, font.clone(), color);
    painter.text(Pos2::new(pos.x + offset, pos.y), align, text, font.clone(), color);
    painter.text(pos, align, text, font, color);
}

fn render_lyrics_view(
    painter: &Painter,
    rect: Rect,
    song: &Song,
    state: &AppState,
    player: &AudioPlayer,
    is_dark: bool,
    scale: f32,
) {
    if song.parsed_lyrics.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No Lyrics Available",
            FontId::proportional(13.0_f32 * scale),
            LcdPalette::text_secondary(is_dark),
        );
        return;
    }

    let line_h = 24.0_f32 * scale;
    let curr_time = player.current_time_sec;

    // Find active lyric line index
    let mut active_idx = 0;
    for (i, line) in song.parsed_lyrics.iter().enumerate() {
        if line.timestamp_sec <= curr_time {
            active_idx = i;
        } else {
            break;
        }
    }

    let center_y = rect.center().y;

    // Warm butter yellow highlight for the synced active line (theme-adaptive)
    let active_color = if is_dark {
        Color32::from_rgb(255, 202, 48)
    } else {
        Color32::from_rgb(224, 152, 8)
    };

    for (i, line) in song.parsed_lyrics.iter().enumerate() {
        let rel_idx = (i as i32) - (active_idx as i32);
        let y = center_y + (rel_idx as f32) * line_h + state.lyrics_scroll_offset;

        if y >= rect.min.y - 10.0_f32 * scale && y <= rect.max.y + 10.0_f32 * scale {
            let is_active = i == active_idx;
            let font = if is_active {
                FontId::proportional(13.0_f32 * scale)
            } else {
                FontId::proportional(11.0_f32 * scale)
            };

            let color = if is_active {
                active_color
            } else {
                LcdPalette::text_secondary(is_dark)
            };

            // All lyrics render bold; the synced active line pops in warm yellow
            draw_bold_text(
                painter,
                Pos2::new(rect.center().x, y),
                egui::Align2::CENTER_CENTER,
                &line.text,
                font,
                color,
                scale,
            );
        }
    }
}

fn render_visualizer_view(
    painter: &Painter,
    ctx: &Context,
    rect: Rect,
    song: &Song,
    player: &AudioPlayer,
    is_dark: bool,
    is_album_cover: bool,
    scale: f32,
) {
    let primary = if is_album_cover {
        LcdPalette::ALBUM_COVER_TEXT_PRIMARY
    } else {
        LcdPalette::text_primary(is_dark)
    };
    let secondary = if is_album_cover {
        LcdPalette::ALBUM_COVER_TEXT_SECONDARY
    } else {
        LcdPalette::text_secondary(is_dark)
    };

    painter.text(
        Pos2::new(rect.center().x, rect.min.y + 18.0 * scale),
        egui::Align2::CENTER_CENTER,
        "SPECTRUM",
        FontId::proportional(11.0 * scale),
        primary,
    );

    let bars_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + 18.0 * scale, rect.min.y + 38.0 * scale),
        Pos2::new(rect.max.x - 18.0 * scale, rect.max.y - 38.0 * scale),
    );
    let count = crate::audio::visualizer::VIZ_BANDS;
    let gap = 3.0 * scale;
    let bar_w = (bars_rect.width() - gap * (count - 1) as f32) / count as f32;

    // Real band energies captured from the live audio tap; smooth with peak-decay.
    let live = player
        .viz_bands
        .lock()
        .map(|g| *g)
        .unwrap_or([0.0_f32; crate::audio::visualizer::VIZ_BANDS]);
    let energy = if player.is_playing { player.volume.max(0.18).sqrt() } else { 0.0_f32 };

    for i in 0..count {
        let target = live[i] * energy;
        let smoothed = (target * 2.2_f32).min(1.0_f32);
        let min_h = if player.is_playing { 2.5 * scale } else { 1.5 * scale };
        let h = (bars_rect.height() * smoothed).clamp(min_h, bars_rect.height());
        let x = bars_rect.min.x + i as f32 * (bar_w + gap);
        let bar = Rect::from_min_size(Pos2::new(x, bars_rect.max.y - h), Vec2::new(bar_w, h));
        let col = if is_album_cover {
            Color32::from_rgb(210, 201, 176)
        } else if i < 6 {
            Color32::from_rgb(75, 210, 135)
        } else if i < 12 {
            Color32::from_rgb(245, 190, 70)
        } else {
            Color32::from_rgb(235, 100, 85)
        };
        painter.rect_filled(bar, 1.5 * scale, col);
    }

    let artist = LcdRenderer::get_display_artist(&song.artist, ctx.input(|i| i.time));
    painter.text(
        Pos2::new(rect.center().x, rect.max.y - 17.0 * scale),
        egui::Align2::CENTER_CENTER,
        format!("{} • {}", song.title, artist),
        FontId::proportional(9.5 * scale),
        secondary,
    );
}

fn format_time(seconds: f32) -> String {
    let s = seconds.floor() as u32;
    let mins = s / 60;
    let rem_s = s % 60;
    format!("{}:{:02}", mins, rem_s)
}
