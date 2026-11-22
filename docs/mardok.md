# Mardok integration

- Mardok is a local external CLI, invoked with cargo -q run from its repository. Default repository: ~/code/mardok.
- The invocation overrides profile.dev.codegen-backend to llvm because Mardok's Cranelift development default can abort image exports.
- Its source and runtime resources remain external. WordCraft does not depend on Mardok application crates.
- Read ~/code/mardok/AGENTS.md and docs/cli.md before changing CLI arguments; verify those documents against the CLI source.
- parse article requires --journal, --input-format docx or odt, and --output-formats.
- The development CLI writes resources/journals/JOURNAL/ARTICLE_ID beneath its working directory. There is no CLI output-directory option.
- The existing output directory override only controls validation discovery. Directory and JSON names must match the opened document stem.
- Parse requires a clean saved document. It reads the disk file without serializing or resaving through WordCraft.
- The journal and integration settings last for the session. Mardok owns journal configuration and formatting rules.
- Supported outputs: JSON, HTML, SciELO XML, SciELO Markup, DesignCraft, PDF, IDML and EPUB. JATS XML is advertised by Mardok's enum but rejected by its writer.
- Mardok's all mode omits SciELO Markup and ignores additional formats when all is present. WordCraft selects every supported format initially and passes the explicit list.
- Semantic validation reads article JSON. Structure validation reads matching XML files, including translations, with --format json.
- Validate XML File selects an independent XML path for structure validation, without a journal or article JSON. The opened article and unsaved edits remain in place.
- Native automation uses mardok.xml with path and optional wait=true. The desktop user command opens the existing host file-picker service when path is omitted.
- References validation extracts the article's references array into a temporary JSON file, passes it to validate refs with --journal, then removes it.
- Semantic validation can exit successfully while reporting errors. WordCraft keeps process exit status separate from finding counts.
- Missing, malformed or truncated reports are identified as incomplete. Logs and reports remain available after the panel closes.
- The interface uses a resizable bottom panel with separate Logs and Report views, chosen by the operator.
- Native unconfined CLI/control automation uses the same registered commands as the ribbon. wait=true runs an action to completion; otherwise poll mardok.status.
- Mardok cannot honor WordCraft automation read/write roots, so external actions and results are refused under confinement. Browser builds cannot launch the CLI.
- Native process filesystem operations live under engine/src/io/ to preserve the filesystem-gateway invariant; the logical process module is private to mardok.
- UI patterns: ribbon groups, standard checkboxes, nonmodal bottom region, selectable report text. Closing the panel returns focus to the document.
- The document/results split uses Craft SplitArea, expandable sections use Craft Section, and logs/findings use scrollable Craft Stack. The panel uses Craft controls, segmented views and SelectableText throughout: raw egui controls and containers collided during the shell's repeated layout measurements, caught by the strict UI test harness.
- Validation completion posts status and a polite live-region announcement. Keyboard tests cover All and closing the report panel.

## Verification commands

- cargo check -j 1 -p wordcraft-engine -p wordcraft-ui-egui
- cargo clippy -j 1 -p wordcraft-engine -p wordcraft-ui-egui --all-targets --features craft/screen-testing -- -D warnings
- cargo test -j 1 -p wordcraft-engine -p wordcraft-ui-egui --features craft/screen-testing --config 'profile.test.package.wordcraft-ui-egui.opt-level=0' mardok -- --test-threads=1
- cargo test -j 1 -p wordcraft-engine -p wordcraft-ui-egui --features craft/screen-testing --no-fail-fast --config 'profile.test.package.wordcraft-ui-egui.opt-level=0' -- --test-threads=2
- cargo test -j 1 -p wordcraft-engine -p wordcraft-ui-egui --features craft/screen-testing --config 'profile.test.package.wordcraft-ui-egui.opt-level=0' commands_do_not_touch_the_file_system_themselves -- --test-threads=1
- CARGO_BUILD_JOBS=1 cargo run --package xtask -- layers
- CARGO_BUILD_JOBS=1 cargo run --package xtask -- wasm
- The screenshot example and existing UI test harness require craft/screen-testing. Enable it at invocation time without changing application dependencies.
- Keeping both packages and craft/screen-testing in focused test invocations preserves Cargo's dependency feature graph. A UI-only test invocation changed that graph and began rebuilding dependencies.
- The test command disables optimization for the UI package to reduce rebuild time. Assertions, glyph checks and test coverage remain enabled; application manifests and production build profiles are unchanged.
- Full suites include language-pattern and interface-glyph checks that depend on the optional inputs described in AGENTS.md. The available fallback fonts lack symbols used by the existing glyph and typing-dialog tests.
- Speech and Zotero mock tests create local sockets; sandbox permissions can deny them before their assertions run. The translation audit clicks Help controls and can flag network-error strings when GitHub access fails.
