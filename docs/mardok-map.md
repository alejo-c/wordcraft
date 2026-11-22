# Mardok

## Engine state and commands

- FORMATS — crates/engine/src/mardok.rs:12, supported writer selections. Uses: verified external CLI formats. Used by: settings, ribbon, native arguments.
- Settings — crates/engine/src/mardok.rs:15, repository, journal, discovery override, output selection. Uses: PathBuf. Used by: State, settings, native arguments, UI.
- Settings::default — crates/engine/src/mardok.rs:22, local repository and all formats. Uses: FORMATS, HOME. Used by: State::default.
- State — crates/engine/src/mardok.rs:34, session-wide settings, job, logs and report. Uses: Settings, process::Job. Used by: Session, commands, UI.
- State::busy — crates/engine/src/mardok.rs:47, unfinished action state. Uses: action, outcome. Used by: gates, settings, UI, synchronous wait.
- State::poll — crates/engine/src/mardok.rs:51, native progress collection or browser no-op. Uses: process::poll. Used by: status, UI poll, synchronous wait.
- State::output_directory — crates/engine/src/mardok.rs:62, explicit discovery path or development CLI output. Uses: Settings. Used by: JSON/XML discovery, status, panel.
- State::article_json — crates/engine/src/mardok.rs:69, same-name directory and JSON detection. Uses: output_directory, file_access. Used by: validation gates, native arguments, status, panel.
- State::xml_files — crates/engine/src/mardok.rs:81, matching article/translation XML discovery. Uses: output_directory, io::list_dir. Used by: structure gate, arguments, status.
- specs — crates/engine/src/mardok.rs:105, registered settings/panel/parse/semantic/structure/XML/references/status actions. Uses: CommandSpec. Used by: cmd::registry.
- external_disabled — crates/engine/src/mardok.rs:132, platform, confinement and running-job gates. Uses: file_access::current, State::busy. Used by: base_disabled, XML spec, structure_disabled.
- base_disabled — crates/engine/src/mardok.rs:144, saved-path and journal gates. Uses: external_disabled. Used by: parse_disabled, validation_disabled.
- input_format — crates/engine/src/mardok.rs:156, DOCX/ODT suffix recognition. Uses: Path. Used by: parse_disabled, process::arguments.
- parse_disabled — crates/engine/src/mardok.rs:164, disk parse eligibility. Uses: base_disabled, input_format, dirty, formats. Used by: parse spec.
- validation_disabled — crates/engine/src/mardok.rs:178, matching article JSON eligibility. Uses: base_disabled, article_json. Used by: validation specs, structure_disabled.
- structure_disabled — crates/engine/src/mardok.rs:188, current XML or article-output XML eligibility. Uses: external_disabled, validation_disabled, xml_files. Used by: structure spec.
- settings — crates/engine/src/mardok.rs:201, transactional validated settings updates. Uses: expand_home, FORMATS, confinement. Used by: mardok.settings.
- expand_home — crates/engine/src/mardok.rs:246, leading tilde path expansion. Uses: HOME. Used by: settings, XML start.
- panel — crates/engine/src/mardok.rs:255, open/close results without changing the document. Uses: State::open. Used by: mardok.panel.
- status — crates/engine/src/mardok.rs:260, progress and report JSON. Uses: State::poll, discovery. Used by: mardok.status, start.
- parse — crates/engine/src/mardok.rs:275, article parse dispatch. Uses: start. Used by: mardok.parse.
- semantic — crates/engine/src/mardok.rs:279, semantic validation dispatch. Uses: start. Used by: mardok.semantic.
- structure — crates/engine/src/mardok.rs:283, XML validation dispatch. Uses: start. Used by: mardok.structure.
- xml — crates/engine/src/mardok.rs:287, independent XML path validation. Uses: start. Used by: mardok.xml.
- references — crates/engine/src/mardok.rs:295, reference-array validation dispatch. Uses: start. Used by: mardok.references.
- start — crates/engine/src/mardok.rs:299, native action and optional synchronous wait. Uses: process::start, State::poll, status. Used by: action handlers.

## Native filesystem gateway

- process module — crates/engine/src/io/mardok.rs, private path-mounted module of engine::mardok. Uses: native filesystem/process APIs. Used by: State and commands.
- Job — crates/engine/src/io/mardok.rs:15, child, output channel, bounded capture and temporary references. Uses: Child, ReferenceFile. Used by: State, process::poll.
- Job::drop — crates/engine/src/io/mardok.rs:25, reap unfinished cargo process. Uses: Child. Used by: job lifecycle.
- ReferenceFile — crates/engine/src/io/mardok.rs:34, temporary references JSON owner. Uses: PathBuf. Used by: arguments, Job.
- ReferenceFile::create — crates/engine/src/io/mardok.rs:36, exclusive private temporary file. Uses: write_references. Used by: arguments.
- ReferenceFile::drop — crates/engine/src/io/mardok.rs:54, remove temporary references. Uses: filesystem. Used by: job lifecycle and failed starts.
- write_references — crates/engine/src/io/mardok.rs:60, serialize extracted reference array. Uses: serde_json. Used by: ReferenceFile::create.
- failed — crates/engine/src/io/mardok.rs:65, contextual command failure. Uses: CmdError. Used by: native operations.
- reader — crates/engine/src/io/mardok.rs:69, bounded-channel pipe reader. Uses: thread, Read. Used by: process::start.
- arguments — crates/engine/src/io/mardok.rs:90, explicit CLI operands and reference extraction. Uses: discovery, ReferenceFile, canonicalize. Used by: process::start, tests.
- process::start — crates/engine/src/io/mardok.rs:141, cargo CLI process launch with LLVM override. Uses: arguments, reader, Job. Used by: engine start.
- plain — crates/engine/src/io/mardok.rs:174, remove terminal color escapes. Uses: regex. Used by: report, poll.
- process::report — crates/engine/src/io/mardok.rs:181, structured findings from XML JSON or tagged semantic/reference messages. Uses: serde_json, plain. Used by: poll, tests.
- process::poll — crates/engine/src/io/mardok.rs:224, drain output and collect completion. Uses: Job, report. Used by: State::poll.

## Interface

- UI run — crates/ui-egui/src/mardok.rs:19, command execution with status errors. Uses: WordApp::run. Used by: settings and panel controls.
- UI ribbon — crates/ui-egui/src/mardok.rs:25, article, formats, validation and results groups. Uses: engine commands, standard controls. Used by: ribbon::tab_body.
- format_label — crates/ui-egui/src/mardok.rs:78, supported output labels. Uses: format ids. Used by: ribbon.
- action — crates/ui-egui/src/mardok.rs:92, registered action button and disabled reason. Uses: CommandRef, why_disabled. Used by: ribbon.
- UI poll — crates/ui-egui/src/mardok.rs:101, progress repaint, completion status and announcement. Uses: State::poll, announce::say. Used by: WordApp::window_work.
- UI show — crates/ui-egui/src/mardok.rs:113, document/results split with a retained resize layout. Uses: SplitArea, results, document callback. Used by: WordApp::paint_window.
- UI results — crates/ui-egui/src/mardok.rs:141, bottom panel contents and retained Logs/Report selection. Uses: settings, report, engine state. Used by: show.
- UI settings — crates/ui-egui/src/mardok.rs:184, repository and existing-output fields. Uses: mardok.settings. Used by: results.
- UI report — crates/ui-egui/src/mardok.rs:199, findings, locations, completeness and raw output. Uses: engine report JSON. Used by: results.
- announce::say — crates/ui-egui/src/announce.rs:24, create a polite completion announcement. Uses: Announcement. Used by: UI poll.

## Existing hooks changed

- cmd::registry — crates/engine/src/cmd.rs:63, adds Mardok specs. Uses: mardok::specs. Used by: Session initialization.
- Session — crates/engine/src/session.rs:48, owns session-wide Mardok state. Uses: mardok::State. Used by: engine and UI.
- Session::new — crates/engine/src/session/lifecycle.rs:33, default Mardok state. Uses: State::default. Used by: hosts and tests.
- Session::swap_doc — crates/engine/src/session/documents.rs:195, retains Mardok outside per-document swaps. Uses: Session. Used by: document switching.
- Session::edit_snapshot — crates/engine/src/session/snapshot.rs:70, excludes Mardok state from undo snapshots. Uses: Session. Used by: command transactions.
- Engine TABS — crates/engine/src/ribbon.rs:18, appends Mardok to defaults. Uses: built-in tab names. Used by: layouts and UI TABS.
- UI TABS — crates/ui-egui/src/ribbon/tabs.rs:13, forwards engine tab defaults. Uses: Engine TABS. Used by: ribbon/customization.
- tab_body — crates/ui-egui/src/ribbon.rs:215, dispatches Mardok content. Uses: UI ribbon. Used by: ribbon::show.
- WordApp::window_work — crates/ui-egui/src/app.rs:382, polls native Mardok jobs. Uses: UI poll. Used by: window logic.
- WordApp::paint_window — crates/ui-egui/src/app/frame.rs:77, reserves Mardok panel before document panes. Uses: UI show. Used by: window paint.
- WordApp::run — crates/ui-egui/src/app/commands.rs:36, asks for an XML path when the user action has none. Uses: ask_file, AfterPick::MardokXml. Used by: ribbon commands.
- AfterPick::MardokXml — crates/ui-egui/src/file_dialogs.rs:82, XML chooser continuation. Uses: existing file service. Used by: WordApp::run, finish_pick.
- WordApp::finish_pick — crates/ui-egui/src/file_dialogs.rs:271, validates selected XML or preserves the document on cancellation. Uses: mardok.xml. Used by: poll_file_dialog.

## Tests

- Engine behavior tests — crates/engine/src/mardok/tests.rs:26, discovery, gates, settings, reports, confinement, reference extraction and CLI operands. Uses: temporary original fixtures. Used by: cargo test mardok.
- UI tests — crates/ui-egui/src/mardok.rs:248, XML cancellation, All selection and retained report closing/focus. Uses: file-service fake, test_harness, accessibility queries. Used by: cargo test mardok.
