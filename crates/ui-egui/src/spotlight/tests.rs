use super::*;
use crate::{Services, UiState, WordApp};
use egui_kittest::kittest::Queryable;
use serde_json::json;
use wordcraft_engine::Session;

fn app(text: &str) -> WordApp {
    WordApp::new(Session::new(Document::from_text(text)), Services::default())
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
    let old: UiState = serde_json::from_str("{}").unwrap();
    assert!(old.spotlight);
}

#[test]
fn spotlight_geometry_stays_in_the_margin_on_either_side_at_any_zoom() {
    let mut app = app("Paragraph");
    let layout = app.session.layout();
    let page = &layout.pages[0];
    let block = blocks(&app.session.doc, page).remove(0);
    for scale in [0.3, 1.0, 2.5] {
        let page_rect = Rect::from_min_size(pos2(120.0, -250.0), vec2(page.w * scale, page.h * scale));
        for left in [true, false] {
            let rect = block_rect(page, page_rect, &block, left).unwrap();
            assert!(rect.left() >= page_rect.left());
            assert!(rect.right() <= page_rect.right() - page.markup * scale);
            assert_eq!(rect.top(), page_rect.top() + block.top * scale);
            assert!((rect.height() - (block.bottom - block.top) * scale).abs() < 0.001);
            if left {
                assert!(rect.right() < page_rect.left() + page.body.x * scale);
            } else {
                assert!(rect.left() > page_rect.left() + (page.body.x + page.body.w) * scale);
            }
        }
    }
}

#[test]
fn spotlight_pane_indicators_match_badges_and_style_buttons_still_apply() {
    let mut app = app("Paragraph");
    app.run("view.stylesPane", json!({"value": true})).unwrap();
    app.run("ui.tab", json!({"tab": "View"})).unwrap();
    let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
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
    assert!(rects.iter().any(|r| r.width() > 15.0), "the document shows the matching badge");
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
    let page = h.state().canvas.page_rects[0];
    assert!(h.output().shapes.iter().any(
        |s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == color("Heading1") && r.rect.width() > 15.0 && r.rect.center().x > page.center().x)
    ));
}
