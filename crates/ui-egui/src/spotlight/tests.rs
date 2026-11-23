use super::*;
use crate::prefs::Prefs;
use crate::{Services, WordApp};
use egui_kittest::kittest::Queryable;
use serde_json::json;
use wordcraft_engine::Session;

fn app(text: &str) -> WordApp {
    let mut app = WordApp::new(Session::new(Document::from_text(text)), Services::default());
    app.ui.spotlight = true;
    app
}

#[test]
fn spotlight_labels_follow_styles_across_wrapping_and_pages() {
    let mut app = app(&"A paragraph with many words. ".repeat(800));
    app.run("para.style", json!({"style": "Heading1"})).unwrap();
    let layout = app.session.layout();
    assert!(layout.pages.len() > 1);
    for page in &layout.pages {
        let spans = blocks(&app.session.doc, page);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style, "Heading1");
        assert_eq!(spans[0].name, crate::i18n::style_name(app.session.doc.styles.get("Heading1").unwrap()));
        assert!(spans[0].bottom > spans[0].top);
    }
    app.run("para.style", json!({"style": "Normal"})).unwrap();
    let layout = app.session.layout();
    assert!(layout.pages.iter().flat_map(|page| blocks(&app.session.doc, page)).all(|b| b.style == "Normal"));
}

#[test]
fn spotlight_uses_the_same_color_for_document_and_pane_after_reopening() {
    assert_eq!(color("Normal"), Color32::from_rgb(88, 109, 140));
    assert_ne!(color("Normal"), color("Heading1"));
    let mut app = app("Paragraph");
    let original = serde_json::to_value(&app.session.doc).unwrap();
    assert!(app.ui.spotlight);
    assert_eq!(app.run("ui.spotlight", json!({})).unwrap()["spotlight"], false);
    let prefs = serde_json::to_string(&app.prefs()).unwrap();
    let mut reopened = self::app("Another document");
    reopened.apply_prefs(serde_json::from_str(&prefs).unwrap());
    assert!(!reopened.ui.spotlight);
    assert_eq!(app.run("ui.spotlight", json!({"value": true})).unwrap()["spotlight"], true);
    assert!(app.run("ui.spotlight", json!({"value": "true"})).is_err());
    assert!(app.ui.spotlight);
    assert_eq!(serde_json::to_value(&app.session.doc).unwrap(), original);
    let old: Prefs = serde_json::from_str("{}").unwrap();
    assert!(!old.spotlight);
}

#[test]
fn spotlight_geometry_stays_outside_the_page_and_markup_at_any_zoom() {
    let mut app = app("Paragraph");
    let layout = app.session.layout();
    let page = &layout.pages[0];
    let block = blocks(&app.session.doc, page).remove(0);
    for scale in [0.3, 1.0, 2.5] {
        let page_rect = Rect::from_min_size(pos2(120.0, -250.0), vec2(page.w * scale, page.h * scale));
        for left in [true, false] {
            let markup = 216.0;
            let rect = block_rect(page, page_rect, &block, left, markup).unwrap();
            assert!((rect.width() - BLOCK_WIDTH).abs() < 0.001);
            assert_eq!(rect.top(), page_rect.top() + block.top * scale);
            assert!((rect.height() - (block.bottom - block.top) * scale).abs() < 0.001);
            if left {
                assert!(rect.right() < page_rect.left());
            } else {
                assert!(rect.left() > page_rect.right() + markup * scale);
            }
        }
    }
}

#[test]
fn spotlight_reserves_a_gutter_for_each_page_in_a_row() {
    let mut app = app("Paragraph");
    let layout = app.session.layout();
    let page = layout.pages[0].clone();
    for dock in [wordcraft_engine::StylesPaneDock::Left, wordcraft_engine::StylesPaneDock::Right] {
        app.session.view.styles_pane_dock = dock;
        let page_rect = Rect::from_min_size(pos2(120.0, 40.0), vec2(page.w * 0.35, page.h * 0.35));
        let block = blocks(&app.session.doc, &page).remove(0);
        let badge = block_rect(&page, page_rect, &block, on_left(&app), 0.0).unwrap();
        let gutter = gutter(&app);
        if on_left(&app) {
            assert!(badge.left() >= page_rect.left() - gutter);
            assert!(badge.right() <= page_rect.left());
        } else {
            assert!(badge.left() >= page_rect.right());
            assert!(badge.right() <= page_rect.right() + gutter);
        }
    }
}

#[test]
fn spotlight_pane_indicators_match_badges_and_style_buttons_still_apply() {
    let mut app = app("Paragraph");
    app.run("view.stylesPane", json!({"value": true})).unwrap();
    app.run("ui.tab", json!({"tab": "View"})).unwrap();
    let mut h = crate::test_harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    let fills = |h: &egui_kittest::Harness<'_, WordApp>| -> Vec<Rect> {
        h.output()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(r) if r.fill == color("Normal") => Some(r.rect),
                _ => None,
            })
            .collect()
    };
    let rects = fills(&h);
    assert!(rects.iter().any(|r| r.width() == 10.0 && r.height() == 10.0), "the Styles pane shows its color indicator");
    let page = h.state().canvas.shown.laid[0];
    assert!(rects.iter().any(|r| r.width() > 15.0 && r.right() < page.left()), "the canvas shows the matching badge");
    let pane_spotlight = h.get_all_by_label("Spotlight").last().unwrap();
    pane_spotlight.click();
    h.run_steps(3);
    assert!(!h.state().ui.spotlight);
    assert!(fills(&h).is_empty());
    h.get_all_by_label("Spotlight").last().unwrap().click();
    h.run_steps(3);
    assert!(h.state().ui.spotlight);
    let heading_name = crate::i18n::style_name(h.state().session.doc.styles.get("Heading1").unwrap());
    h.get_by_label(&heading_name).click();
    h.run_steps(3);
    assert_eq!(h.state().session.doc.para_at(&h.state().session.sel.focus).unwrap().props.style.as_deref(), Some("Heading1"));
    h.state_mut().run("view.stylesPaneDock", json!({"dock": "left"})).unwrap();
    h.run_steps(3);
    let page = h.state().canvas.shown.laid[0];
    assert!(
        h.output().shapes.iter().any(
            |s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == color("Heading1") && r.rect.width() > 15.0 && r.rect.left() > page.right())
        )
    );
}
