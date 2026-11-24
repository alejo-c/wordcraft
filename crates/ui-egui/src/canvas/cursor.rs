use super::pages::{helix_overlay_line_numbers, page_doc, page_render_options, page_screen_scale, view_key};
use super::story::dims_body;
use super::{Color32, DocLayout, Hash, Hasher, Pos, Pos2, WordApp};

/// Cached transparent raster of visible line numbers for one placed page while Helix relative line
/// numbers are managed as a screen overlay.
pub(crate) struct LineNumberOverlay {
    key: (usize, usize, Option<u32>, u64, u32, u32, u32, u32),
    area: wordcraft_geom::Rect,
    rendered_size: (f32, f32),
    texture: egui::TextureHandle,
}

/// Paints visible line numbers over each placed page when Helix manages line numbers as a screen
/// overlay, keeping `WordPages::revision` constant as the caret moves.
pub(super) fn paint_helix_line_numbers(
    app: &mut WordApp,
    ui: &egui::Ui,
    layout: &std::sync::Arc<DocLayout>,
    pages: &[craft::pages::ui::PlacedPage],
    view_rect: egui::Rect,
    dim_body: bool,
    relative_origin: Option<u32>,
) {
    if !helix_overlay_line_numbers(app) {
        app.canvas.line_number_overlays.clear();
        return;
    }
    let ppp = ui.ctx().pixels_per_point();
    let vkey = view_key(app, dim_body, None);
    let opts = page_render_options(app, dim_body);
    let mut prior = std::mem::take(&mut app.canvas.line_number_overlays);
    let doc = page_doc(app);
    let mut kept = Vec::new();
    let clip_base = view_rect.intersect(ui.clip_rect());
    for placed in pages {
        let Some(page) = layout.pages.get(placed.page) else { continue };
        if page.numbered_lines.is_empty() {
            continue;
        }
        let (top, bottom) = placed.band;
        let band = egui::Rect::from_min_max(
            egui::pos2(placed.rect.min.x, placed.rect.min.y + top),
            egui::pos2(placed.rect.max.x, placed.rect.min.y + bottom),
        )
        .intersect(clip_base);
        if !band.is_positive() {
            continue;
        }
        let scale = page_screen_scale(placed.rect, page, 1.0);
        if !scale.is_finite() || scale <= 0.0 {
            continue;
        }
        let y0 = ((band.min.y - placed.rect.min.y) / scale).max(0.0);
        let y1 = ((band.max.y - placed.rect.min.y) / scale).min(page.h);
        // Quantize outward to 64 pt bands so tiny scrolls reuse the cached overlay texture.
        let qy0 = ((y0 / 64.0).floor() * 64.0).max(0.0);
        let qy1 = ((y1 / 64.0).ceil() * 64.0).min(page.h);
        let key = (
            std::sync::Arc::as_ptr(layout) as usize,
            placed.page,
            relative_origin,
            vkey,
            scale.to_bits(),
            ppp.to_bits(),
            qy0.to_bits(),
            qy1.to_bits(),
        );
        let entry = if let Some(pos) = prior.iter().position(|e| e.key == key) {
            Some(prior.swap_remove(pos))
        } else {
            let (area, draws) = wordcraft_layout::display::line_number_draws(doc, page, relative_origin, (qy0, qy1), &opts.display);
            area.filter(|_| !draws.is_empty()).map(|area| {
                let raster_scale = (scale * ppp).max(0.01);
                let pw = ((area.w * raster_scale).ceil() as u32).clamp(1, wordcraft_render::MAX_SIDE);
                let ph = ((area.h * raster_scale).ceil() as u32).clamp(1, wordcraft_render::MAX_SIDE);
                let img = wordcraft_render::render_items(doc, &draws, area, raster_scale, &opts);
                let color_image = egui::ColorImage::from_rgba_premultiplied([img.width as usize, img.height as usize], &img.pixels);
                let texture = ui.ctx().load_texture(format!("helix_ln_{}", placed.page), color_image, egui::TextureOptions::LINEAR);
                LineNumberOverlay { key, area, rendered_size: (pw as f32 / raster_scale, ph as f32 / raster_scale), texture }
            })
        };
        if let Some(entry) = entry {
            let screen_rect = egui::Rect::from_min_size(
                egui::pos2(placed.rect.min.x + entry.area.x * scale, placed.rect.min.y + entry.area.y * scale),
                egui::vec2(entry.rendered_size.0 * scale, entry.rendered_size.1 * scale),
            );
            let painter = ui.painter().with_clip_rect(band);
            painter.image(entry.texture.id(), screen_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            kept.push(entry);
        }
    }
    app.canvas.line_number_overlays = kept;
}

pub(super) fn relative_line_origin(app: &WordApp, layout: &DocLayout, focused: bool) -> Option<u32> {
    if !focused
        || !crate::keys::helix::block_caret(app)
        || app.session.view.read_mode
        || super::preview_sel(app).is_some()
        || app.helix_config.as_ref().is_some_and(|config| config.absolute_line_numbers)
    {
        return None;
    }
    layout.line_number_at(&wordcraft_engine::cmd::helix::cursor(&app.session), app.session.page_hint)
}

pub(super) fn paint_helix_cursorline(
    app: &WordApp,
    painter: &egui::Painter,
    layout: &DocLayout,
    rects: &[egui::Rect],
    scale: f32,
    focused: bool,
    tokens: &crate::theme::Tokens,
) {
    if !focused
        || !app.ui.helix_editing
        || app.session.view.read_mode
        || super::preview_sel(app).is_some()
        || !app.helix_config.as_ref().is_some_and(|config| config.cursorline)
    {
        return;
    }
    let pos = if crate::keys::helix::block_caret(app) { wordcraft_engine::cmd::helix::cursor(&app.session) } else { app.session.sel.focus.clone() };
    let Some(caret) = layout.caret_on(&pos, app.session.page_hint).filter(|c| c.width == 0.0) else { return };
    let (Some(page), Some(rect)) = (layout.pages.get(caret.page), rects.get(caret.page)) else { return };
    if !rect.intersects(painter.clip_rect()) {
        return;
    }
    let scale = page_screen_scale(*rect, page, scale);
    let lines = wordcraft_layout::hit::page_lines(page, pos.story);
    if let Some(line) = lines.iter().find(|line| line.path == &pos.path && line.li == line.para.line_of(pos.off)) {
        let band = egui::Rect::from_min_max(
            egui::pos2(rect.left() + line.left * scale, rect.top() + line.top * scale),
            egui::pos2(rect.left() + line.right * scale, rect.top() + line.bottom * scale),
        );
        painter.rect_filled(band, 0.0, tokens.accent.gamma_multiply(0.13));
    }
}

pub(super) fn paint_caret_bar(painter: &egui::Painter, bar: [Pos2; 2], color: Color32, opacity: f32, block_height: Option<f32>) {
    if let Some(height) = block_height {
        let thickness = (height * 0.5).max(2.0);
        let bar = egui::Rect::from_two_pos(bar[0], bar[1]);
        let block = egui::Rect::from_min_size(bar.min, bar.size().max(egui::Vec2::splat(thickness)));
        painter.rect_filled(block, 0.0, color.gamma_multiply(opacity * 0.4));
    } else {
        painter.line_segment(bar, egui::Stroke::new(1.5, color.gamma_multiply(opacity)));
    }
}

/// Caret position on screen.
pub fn caret_screen(app: &mut WordApp) -> Option<(Pos2, f32)> {
    let l = app.session.layout();
    let c = l.caret_on(&app.session.sel.focus, app.session.page_hint)?;
    let placed = app.canvas.shown.pages.iter().find(|pl| pl.page == c.page)?;
    let p = placed.to_screen(craft::behavior::kurbo::Point::new(f64::from(c.x), f64::from(c.top)));
    Some((p, c.height.max(c.width) * placed.scale as f32))
}

/// The document position under screen point `p`, from the last pass's page placement.
pub fn pos_from_screen(app: &mut WordApp, p: Pos2) -> Option<Pos> {
    let placed = app.canvas.shown.pages.iter().find(|pl| pl.rect.contains(p))?;
    let at = placed.to_page(p);
    let (page, x, y) = (placed.page, at.x as f32, at.y as f32);
    let story = app.session.sel.focus.story;
    app.session.layout().hit(page, x, y, story)
}

/// The caret colour that shows against `bg`: black or white, whichever contrasts more (WCAG
/// contrast ratio), as Word's caret inverts what lies under it (#299). A white caret on white
/// paper, or a black one on a dark page colour or highlight, would vanish.
pub(crate) fn caret_contrast(bg: wordcraft_doc::props::Rgb) -> Color32 {
    let lin = |v: u8| {
        let c = v as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let l = 0.2126 * lin(bg.0) + 0.7152 * lin(bg.1) + 0.0722 * lin(bg.2);
    // Contrast with black (l + 0.05) / 0.05 against with white 1.05 / (l + 0.05).
    if (l + 0.05) * (l + 0.05) >= 0.05 * 1.05 { Color32::BLACK } else { Color32::WHITE }
}

/// The input method's cursor in its composition, in chars: the end of the range it reports, or the
/// end of the composition when it reports none. The range comes from the OS: it is clamped to the text.
pub(super) fn preedit_cursor_chars(text: &str, range: Option<&std::ops::Range<usize>>) -> usize {
    let n = text.chars().count();
    range.map_or(n, |r| r.end.min(n))
}

/// The caret colour at page point (`x`, `y`) of page `page`: [`caret_contrast`] of the paper,
/// shading or highlight there. Cached until the layout, the view or the point changes, so the
/// blink doesn't rebuild the page's draw list every frame.
pub(super) fn caret_color_at(
    app: &mut WordApp,
    layout: &std::sync::Arc<DocLayout>,
    page: usize,
    x: f32,
    y: f32,
    view: u64,
    dim_body: bool,
) -> Color32 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (std::sync::Arc::as_ptr(layout) as usize, page, x.to_bits(), y.to_bits(), view).hash(&mut h);
    let key = h.finish();
    if let Some((k, c)) = app.canvas.caret_bg
        && k == key
    {
        return c;
    }
    let Some(p) = layout.pages.get(page) else { return Color32::BLACK };
    let bg = wordcraft_render::background_at(&app.session.doc, p, &page_render_options(app, dim_body), x, y);
    let c = caret_contrast(bg);
    app.canvas.caret_bg = Some((key, c));
    c
}

/// The caret colour at page point (`x`, `y`) of page `page`, uncached: for a caret drawn while
/// dragging, such as an inline picture's insertion cursor.
pub(crate) fn caret_color_on(app: &WordApp, layout: &DocLayout, page: usize, x: f32, y: f32) -> Color32 {
    let Some(p) = layout.pages.get(page) else { return Color32::BLACK };
    caret_contrast(wordcraft_render::background_at(&app.session.doc, p, &page_render_options(app, dims_body(app, layout)), x, y))
}
