use super::navigation::{page_at, page_to_screen};
use super::pages::{page_render_options, page_screen_scale};
use super::story::dims_body;
use super::{Color32, DocLayout, Hash, Hasher, Pos, Pos2, WordApp};

pub(super) fn paint_helix_lines(
    app: &WordApp,
    painter: &egui::Painter,
    layout: &DocLayout,
    rects: &[egui::Rect],
    scale: f32,
    focused: bool,
    tokens: &crate::theme::Tokens,
) {
    if !app.ui.helix_editing || app.session.view.read_mode || super::preview_sel(app).is_some() {
        return;
    }
    let modal = crate::keys::helix::block_caret(app);
    let pos = if modal { wordcraft_engine::cmd::helix::cursor(&app.session) } else { app.session.sel.focus.clone() };
    let caret = layout.caret_on(&pos, app.session.page_hint);
    let settings = app.helix_config.as_ref();
    let relative = focused && modal && !settings.is_some_and(|config| config.absolute_line_numbers);
    let highlight = focused && settings.is_some_and(|config| config.cursorline);
    let clip = painter.clip_rect();
    let color = caret_contrast(page_render_options(app, false).paper_of(&app.session.doc));
    let Some(last_visible) = rects.iter().rposition(|rect| rect.intersects(clip)) else { return };
    let last_page = caret.filter(|_| relative).map_or(last_visible, |c| last_visible.max(c.page));
    let mut visible = Vec::new();
    let mut number = 0usize;
    let mut active = None;
    for (pi, (page, rect)) in layout.pages.iter().zip(rects).enumerate().take(last_page.saturating_add(1)) {
        let scale = page_screen_scale(*rect, page, scale);
        let on_screen = rect.intersects(clip);
        let lines = wordcraft_layout::hit::page_lines(page, pos.story);
        let active_line = caret
            .filter(|c| c.page == pi && c.width == 0.0)
            .and_then(|_| lines.iter().find(|line| line.path == &pos.path))
            .map(|line| line.para.line_of(pos.off));
        for line in lines {
            let Some(row) = line.para.lines.get(line.li) else { continue };
            if !row.beside || number == 0 {
                number += 1;
            }
            let is_active = active_line == Some(line.li) && line.path == &pos.path;
            if is_active {
                active = Some(number);
            }
            if !on_screen || (row.beside && !is_active) {
                continue;
            }
            let band = egui::Rect::from_min_max(
                egui::pos2(rect.left() + line.left * scale, rect.top() + line.top * scale),
                egui::pos2(rect.left() + line.right * scale, rect.top() + line.bottom * scale),
            );
            if band.intersects(clip) {
                if is_active && highlight {
                    painter.rect_filled(band, 0.0, tokens.accent.gamma_multiply(0.13));
                }
                if !row.beside {
                    visible.push((number, band.left_center(), band.height().min(10.0)));
                }
            }
        }
    }
    for (number, at, size) in visible {
        let label = active.filter(|&current| relative && current != number).map_or(number, |current| current.abs_diff(number));
        painter.text(
            at - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            label.to_string(),
            egui::FontId::monospace(size),
            color.gamma_multiply(if active == Some(number) { 0.85 } else { 0.55 }),
        );
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
    let p = page_to_screen(app, c.page, c.x, c.top)?;
    let scale = app
        .canvas
        .page_rects
        .get(c.page)
        .and_then(|r| l.pages.get(c.page).map(|page| page_screen_scale(*r, page, app.canvas.scale)))
        .unwrap_or(app.canvas.scale);
    Some((p, c.height.max(c.width) * scale))
}

pub fn pos_from_screen(app: &mut WordApp, p: Pos2) -> Option<Pos> {
    let layout = app.session.layout();
    let (page, x, y) = page_at(&app.canvas.page_rects, &layout, app.canvas.scale, p)?;
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
