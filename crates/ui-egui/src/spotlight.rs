use std::collections::BTreeMap;

use egui::{Align2, Color32, Rect, Sense, Ui, pos2, vec2};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_layout::{DocLayout, Page, Placed};

#[derive(Debug)]
pub(crate) struct StyleBlock {
    style: String,
    name: String,
    top: f32,
    bottom: f32,
}

pub(crate) fn blocks(document: &Document, page: &Page) -> Vec<StyleBlock> {
    let mut spans: BTreeMap<(StoryRef, Path), (f32, f32)> = BTreeMap::new();
    for item in &page.items {
        if let Placed::Lines { story, path, .. } = item {
            let (top, bottom) = item.vertical_span();
            if !top.is_finite() || !bottom.is_finite() || bottom <= top {
                continue;
            }
            spans
                .entry((*story, path.clone()))
                .and_modify(|span| {
                    span.0 = span.0.min(top);
                    span.1 = span.1.max(bottom);
                })
                .or_insert((top, bottom));
        }
    }
    spans
        .into_iter()
        .filter_map(|((story, path), (top, bottom))| {
            let paragraph = document.para(story, &path)?;
            let id = paragraph.props.style.as_deref().unwrap_or("Normal");
            let name = document.styles.get(id).map(crate::i18n::style_name).unwrap_or_else(|| id.to_owned());
            Some(StyleBlock { style: id.to_owned(), name, top, bottom })
        })
        .collect()
}

pub fn color(id: &str) -> Color32 {
    let hash = id.bytes().fold(2166136261u32, |value, byte| (value ^ u32::from(byte)).wrapping_mul(16777619));
    Color32::from_rgb(70 + (hash % 140) as u8, 70 + ((hash >> 8) % 140) as u8, 70 + ((hash >> 16) % 140) as u8)
}

pub(crate) fn indicator(ui: &mut Ui, id: &str, name: &str) {
    let (rect, response) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color(id));
    response.on_hover_text(name);
}

fn block_rect(page: &Page, rect: Rect, block: &StyleBlock, left: bool) -> Option<Rect> {
    let scale = rect.width() / page.w;
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let edge = if left { page.body.x } else { page.body.x + page.body.w };
    let margin = if left { page.body.x } else { page.w - page.markup - edge };
    let width = (margin * scale - 8.0).clamp(0.0, 150.0);
    if width < 4.0 {
        return None;
    }
    let edge = rect.left() + edge * scale;
    let x = if left { edge - width - 4.0 } else { edge + 4.0 };
    Some(Rect::from_min_max(pos2(x, rect.top() + block.top * scale), pos2(x + width, rect.top() + block.bottom * scale)))
}

fn paint_page(document: &Document, page: &Page, ui: &Ui, page_rect: Rect, clip: Rect, left: bool) {
    for (index, block) in blocks(document, page).iter().enumerate() {
        let Some(rect) = block_rect(page, page_rect, block, left) else { continue };
        let visible = rect.intersect(clip);
        if !visible.is_positive() {
            continue;
        }
        let painter = ui.painter().with_clip_rect(visible);
        let fill = color(&block.style);
        painter.rect_filled(rect, 2.0, fill);
        let ink =
            if u32::from(fill.r()) * 299 + u32::from(fill.g()) * 587 + u32::from(fill.b()) * 114 > 145000 { Color32::BLACK } else { Color32::WHITE };
        painter.text(rect.left_center() + vec2(4.0, 0.0), Align2::LEFT_CENTER, &block.name, crate::theme::regular(10.0), ink);
        ui.interact(visible, ui.scope_id().with(("spotlight", page_rect.min.x.to_bits(), page_rect.min.y.to_bits(), index)), Sense::hover())
            .on_hover_text(&block.name);
    }
}

pub(crate) fn show(app: &crate::WordApp, ui: &Ui, layout: &DocLayout, rects: &[Rect], bands: &[(f32, f32)]) {
    if !app.ui.spotlight {
        return;
    }
    let left = app.session.view.styles_pane_dock != wordcraft_engine::StylesPaneDock::Left;
    for (index, page) in layout.pages.iter().enumerate() {
        let Some(rect) = rects.get(index).copied() else { continue };
        let Some(&(top, bottom)) = bands.get(index) else { continue };
        let visible = Rect::from_min_max(pos2(rect.min.x, rect.min.y + top), pos2(rect.max.x, rect.min.y + bottom));
        let clip = ui.clip_rect().intersect(visible);
        if clip.is_positive() {
            paint_page(crate::canvas::page_doc(app), page, ui, rect, clip, left);
        }
    }
}

#[cfg(test)]
mod tests;
