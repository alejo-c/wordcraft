use craft::behavior::dock::{Layout, Node, SplitSize};
use craft::behavior::{Axis, IconId};
use craft::keymap::CommandRef;
use craft::menu::{RibbonGroup, RibbonRow};
use craft::ui::buttons::Button;
use craft::ui::choice::{Checkbox, Segmented};
use craft::ui::contract::{Bind, Entry, Gap, Props as _, Scroll, Show as _, ShowContainer as _, SizeLimits};
use craft::ui::display::{Divider, Spinner, Text};
use craft::ui::docking::tree::FrameRules;
use craft::ui::docking::{SplitArea, SplitFramesCx as _};
use craft::ui::layout_components::{Row, Stack};
use craft::ui::sections::Section;
use craft::ui::text::{SelectableText, TextField};
use serde_json::{Value, json};
use wordcraft_engine::mardok::FORMATS;

use crate::WordApp;

fn run(app: &mut WordApp, id: &str, params: Value) {
    if let Err(error) = app.run(id, params) {
        app.status(error);
    }
}

pub(crate) fn ribbon(row: &mut RibbonRow<'_>, app: &mut WordApp) {
    row.group(RibbonGroup::new("Mardok Article", tl!("Article"), IconId::new("export").into()), |ui| {
        Stack::new().gap(Gap::Tight).show(ui, |ui, _| {
            let mut journal = app.session.mardok.settings.journal.clone();
            let before = journal.clone();
            TextField::new(tl!("Journal"), Bind::bound(&mut journal)).placeholder(tl!("Journal acronym")).show(ui);
            if journal != before {
                run(app, "mardok.settings", json!({"journal": journal}));
            }
            action(ui, app, "mardok.parse", "Parse Article");
        });
    });
    row.group(RibbonGroup::new("Mardok Formats", tl!("Output Formats"), IconId::new("export").into()), |ui| {
        ui.add_enabled_ui(!app.session.mardok.busy(), |ui| {
            let mut formats = app.session.mardok.settings.formats.clone();
            let before = formats.clone();
            let mut all = FORMATS.iter().all(|f| formats.iter().any(|selected| selected == f));
            let old_all = all;
            Checkbox::new(Bind::bound(&mut all)).label(tl!("All")).show(ui);
            if all != old_all {
                formats = if all { FORMATS.iter().map(|f| (*f).to_owned()).collect() } else { Vec::new() };
            }
            for group in FORMATS.chunks(3) {
                Stack::new().gap(Gap::Tight).show(ui, |ui, _| {
                    for format in group {
                        let mut on = formats.iter().any(|f| f == format);
                        Checkbox::new(Bind::bound(&mut on)).label(tl!(format_label(format))).show(ui);
                        if on && !formats.iter().any(|f| f == format) {
                            formats.push((*format).to_owned());
                        } else if !on {
                            formats.retain(|f| f != format);
                        }
                    }
                });
            }
            if formats != before {
                run(app, "mardok.settings", json!({"formats": formats}));
            }
        });
    });
    row.group(RibbonGroup::new("Mardok Validation", tl!("Validation"), IconId::new("check").into()), |ui| {
        Stack::new().gap(Gap::Tight).show(ui, |ui, _| {
            action(ui, app, "mardok.semantic", "Validate Semantics");
            action(ui, app, "mardok.structure", "Validate Structure");
            action(ui, app, "mardok.references", "Validate References");
        });
        action(ui, app, "mardok.xml", "Validate XML File…");
    });
    row.group(RibbonGroup::new("Mardok Results", tl!("Results"), IconId::new("info").into()), |ui| {
        Button::command(CommandRef::with_params("mardok.panel", json!({"value": true}))).label(tl!("Logs and Report")).show(ui);
    });
}

fn format_label(format: &str) -> &'static str {
    match format {
        "json" => "JSON",
        "html" => "HTML",
        "scielo-xml" => "SciELO XML",
        "scielo-markup" => "SciELO Markup",
        "designcraft" => "DesignCraft",
        "pdf" => "PDF",
        "idml" => "IDML",
        "epub" => "EPUB",
        _ => "",
    }
}

fn action(ui: &mut egui::Ui, app: &WordApp, id: &str, label: &str) {
    let button = Button::command(CommandRef::new(id)).label(tl!(label).to_string());
    if let Some(reason) = app.session.why_disabled(id) {
        button.tooltip(tl!(reason)).show(ui);
    } else {
        button.show(ui);
    }
}

pub(crate) fn poll(app: &mut WordApp, ctx: &egui::Context) {
    if app.session.mardok.poll() {
        let outcome = app.session.mardok.outcome.as_deref().unwrap_or("completed");
        let message = format!("{}: {}", tl!("Mardok"), tl!(outcome));
        app.status(message.clone());
        crate::announce::say(app, message, ctx);
    }
    if app.session.mardok.busy() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

pub(crate) fn show(app: &mut WordApp, ui: &mut egui::Ui, mut document: impl FnMut(&mut WordApp, &mut egui::Ui)) {
    if !app.session.mardok.open {
        document(app, ui);
        return;
    }
    let key = ui.scope_id().with("mardok-layout");
    let mut layout = ui.ctx().data_mut(|data| data.get_temp::<Layout<bool>>(key)).unwrap_or_else(|| {
        Layout::new(Some(Node::split(Axis::Y, SplitSize::Ratio(2.0 / 3.0), Node::tabs(vec![false], 0), Node::tabs(vec![true], 0))), Vec::new())
    });
    let limits = |_: &bool| SizeLimits::default();
    let stripless = |_: &bool| true;
    SplitArea::new(tl!("Document and Mardok"), Bind::bound(&mut layout), FrameRules::new(&limits).stripless(&stripless))
        .id_salt("mardok-split")
        .divider_label(tl!("Resize Mardok Panel"))
        .show(ui, |ui, cx| {
            for (is_results, rect) in cx.group_frames::<bool>() {
                Stack::new().id_salt(is_results).fill().show_at(ui, rect, |ui, _| {
                    if is_results {
                        results(app, ui);
                    } else {
                        document(app, ui);
                    }
                });
            }
        });
    ui.ctx().data_mut(|data| data.insert_temp(key, layout));
}

fn results(app: &mut WordApp, ui: &mut egui::Ui) {
    let tab_id = ui.scope_id().with("mardok-report-tab");
    let mut report_tab = ui.ctx().data_mut(|data| data.get_temp::<bool>(tab_id).unwrap_or(false));
    Row::new().show(ui, |ui, _| {
        Text::new(tl!("Mardok")).show(ui);
        let views = [Entry::new(false, tl!("Logs")), Entry::new(true, tl!("Report"))];
        Segmented::new(tl!("Logs and Report"), Bind::bound(&mut report_tab), &views).show(ui);
        if app.session.mardok.busy() {
            Spinner::new().show(ui);
            Text::new(tl!("Running…")).show(ui);
        } else if let Some(outcome) = &app.session.mardok.outcome {
            Text::new(tl!(outcome).to_string()).show(ui);
        }
        if Button::new(tl!("Close Mardok Panel")).show(ui).clicked() {
            run(app, "mardok.panel", json!({"value": false}));
            app.canvas.focus_page_after_pass();
        }
    });
    ui.ctx().data_mut(|data| data.insert_temp(tab_id, report_tab));
    Section::new(tl!("Mardok Settings")).initially_open(false).show(ui, |ui, _| settings(app, ui));
    if let Some(path) = &app.session.mardok.input {
        Text::new(format!("{}: {}", tl!("Input"), path.display())).show(ui);
    }
    if let Some(input) = &app.session.path
        && !app.session.mardok.input.as_ref().is_some_and(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")))
    {
        if let Some(dir) = app.session.mardok.output_directory(input) {
            Text::new(format!("{}: {}", tl!("Output directory"), dir.display())).show(ui);
        }
        if app.session.mardok.article_json(input).is_none() {
            Text::new(tl!("No matching article JSON found. Parse with JSON output or choose the existing article directory in Settings.")).show(ui);
        }
    }
    Divider::horizontal().show(ui);
    if report_tab {
        report(app, ui);
    } else {
        Stack::new().id_salt("mardok-logs").fill().scroll(Scroll::both().anchored_end()).show(ui, |ui, _| {
            SelectableText::new(tl!("Logs text"), &app.session.mardok.logs).mono().multiline().show(ui);
        });
    }
}

fn settings(app: &mut WordApp, ui: &mut egui::Ui) {
    ui.add_enabled_ui(!app.session.mardok.busy(), |ui| {
        let settings = &app.session.mardok.settings;
        let mut repository = settings.repository.to_string_lossy().into_owned();
        let mut directory = settings.output_directory.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let before = (repository.clone(), directory.clone());
        TextField::new(tl!("Mardok repository"), Bind::bound(&mut repository)).show(ui);
        TextField::new(tl!("Existing article output directory"), Bind::bound(&mut directory)).show(ui);
        Text::new(tl!("Parsing uses Mardok's output directory. The directory override locates existing results for validation.")).show(ui);
        if (repository.clone(), directory.clone()) != before {
            run(app, "mardok.settings", json!({"repository": repository, "output_directory": directory}));
        }
    });
}

fn report(app: &WordApp, ui: &mut egui::Ui) {
    let report = &app.session.mardok.report;
    if report.is_null() {
        Text::new(tl!("Run a validation action to see its report.")).show(ui);
        return;
    }
    Row::new().show(ui, |ui, _| {
        Text::new(format!("{}: {}", tl!("Errors"), report["errors"].as_u64().unwrap_or(0))).show(ui);
        Text::new(format!("{}: {}", tl!("Warnings"), report["warnings"].as_u64().unwrap_or(0))).show(ui);
        if report["truncated"] == true {
            Text::new(tl!("Output was truncated; this report is incomplete.")).show(ui);
        }
        if report["complete"] == false {
            Text::new(tl!("No complete validation report was returned. See Logs for details.")).show(ui);
        }
        if report["process_success"] == false {
            Text::new(tl!("Mardok exited with an error. See Logs for details.")).show(ui);
        }
    });
    Stack::new().id_salt("mardok-report").fill().scroll(Scroll::both()).show(ui, |ui, _| {
        if let Some(error) = report.get("error").and_then(Value::as_str) {
            Text::new(error.to_owned()).show(ui);
        }
        if let Some(findings) = report["findings"].as_array() {
            for finding in findings {
                let severity = finding["severity"].as_str().unwrap_or_default();
                let code = finding["code"].as_str().unwrap_or_default();
                let message = finding["message"].as_str().unwrap_or_default();
                let label = if code.is_empty() { format!("{}: {message}", tl!(severity)) } else { format!("{} {code}: {message}", tl!(severity)) };
                SelectableText::new(label.clone(), &label).multiline().show(ui);
                if let Some(file) = finding["file"].as_str() {
                    let location = match (finding["line"].as_u64(), finding["column"].as_u64()) {
                        (Some(line), Some(column)) => format!("{file}:{line}:{column}"),
                        _ => file.to_owned(),
                    };
                    Text::new(location).show(ui);
                }
                Divider::horizontal().show(ui);
            }
        }
        Section::new(tl!("Raw validation output")).initially_open(false).show(ui, |ui, _| {
            if let Some(raw) = report["raw"].as_str() {
                SelectableText::new(tl!("Raw validation output text"), raw).mono().multiline().show(ui);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;
    use egui_kittest::kittest::{NodeT, Queryable};

    use super::*;

    #[test]
    fn choosing_an_xml_file_preserves_the_edited_article_when_cancelled() {
        use craft::host::{Host, Picked, ScriptedFiles};
        use std::sync::Arc;

        let files = Arc::new(ScriptedFiles::held());
        let host = Host::builder(crate::APP).files(files.clone()).build();
        let mut app =
            WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default()).with_host(Arc::new(host));
        app.run("text.insert", json!({"text": "Unsaved article"})).unwrap();
        let before = app.session.doc.clone();
        assert_eq!(app.run("mardok.xml", json!({})).unwrap()["pending"], "mardokXml");
        assert!(app.file_dialog_open());
        files.answer(Picked::Cancelled);
        assert_eq!(app.poll_file_dialog().unwrap().unwrap()["cancelled"], true);
        assert!(app.session.doc.same_content(&before));
        assert!(app.session.dirty);
        assert!(!app.session.mardok.busy());
    }

    #[test]
    fn all_outputs_can_be_toggled_by_keyboard_and_individual_formats_remain_selectable() {
        let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("ui.tab", json!({"tab": "Mardok"})).unwrap();
        let mut harness = crate::test_harness::builder().with_size(egui::vec2(1600.0, 1000.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                crate::test_harness::driver_logic(&ctx, app);
                app.ui(ui);
            },
            app,
        );
        harness.run_steps(4);
        harness.get_by_role_and_label(Role::CheckBox, "All").focus();
        harness.key_press(egui::Key::Space);
        harness.run_steps(4);
        assert!(harness.state().session.mardok.settings.formats.is_empty());
        harness.get_by_role_and_label(Role::CheckBox, "JSON").click();
        harness.run_steps(4);
        assert!(harness.state().session.mardok.settings.formats.iter().any(|f| f == "json"));
        assert!(harness.get_by_role_and_label(Role::Button, "Parse Article").accesskit_node().is_disabled());
    }

    #[test]
    fn the_bottom_panel_keeps_its_report_and_closes_from_the_keyboard() {
        let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("mardok.panel", json!({"value": true})).unwrap();
        app.session.mardok.report = json!({"errors": 1, "warnings": 0, "findings": [{"severity": "error", "message": "Missing article title"}]});
        let mut harness = crate::test_harness::builder().with_size(egui::vec2(1600.0, 1000.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                crate::test_harness::driver_logic(&ctx, app);
                app.ui(ui);
            },
            app,
        );
        harness.run_steps(4);
        harness.get_by_label("Report").click();
        harness.run_steps(4);
        assert!(harness.query_by_label("error: Missing article title").is_some());
        harness.get_by_role_and_label(Role::Button, "Close Mardok Panel").focus();
        harness.key_press(egui::Key::Enter);
        harness.run_steps(4);
        assert!(!harness.state().session.mardok.open);
        assert!(!harness.state().session.mardok.report.is_null());
        assert!(harness.state().canvas.focused(&harness.ctx));
    }
}
