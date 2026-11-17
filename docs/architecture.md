# WordCraft architecture

Status: 2026-11-17. Helix editing uses the system clipboard and records paste-key state before modal dispatch.

## Helix symbols

- crates/engine/src/cmd/helix.rs:9: specs registers modal movement, character selection, line selection, find, flip, text objects and replacement.
- Uses: Session, Selection, cmd::caret movement and page geometry, document cluster boundaries, existing selection and text-edit primitives.
- Used by: cmd::registry, the modal canvas adapter, CLI and MCP command dispatch, modal engine tests.
- crates/ui-egui/src/keys/helix.rs:58: State holds per-window mode, pending sequences, counts and bounded insertion recordings without a yank register.
- Uses: CanvasState, UiState::helix_editing, WordApp::run, existing engine edit and movement commands.
- Used by: canvas_events, UI mode commands, Options and the status bar; modal keyboard and focus regressions.
- crates/ui-egui/src/keys/helix.rs:521: event consumes modal keys and text before the ordinary canvas handlers; after_event at line 209 records accepted insert mutations.
- Uses: State, engine revision and undo depth, existing IME interruption and undo-join APIs.
- Used by: keys::canvas::canvas_events.
- crates/engine/src/cmd/caret.rs:155: left, right and page_step are shared within cmd for existing Word navigation and the Helix adapter.
- Uses: document caret boundaries, paragraph navigation and section page geometry.
- Used by: cmd::caret and cmd::helix.
- crates/ui-egui/src/keys/helix.rs:99: command implements ui.helix and ui.helixMode; status at line 73 formats the active mode and pending keys.
- Uses: UiState persistence, CanvasState::helix, existing caret commands.
- Used by: WordApp::ui_command, Options > Advanced > Customize, chrome::status_bar.
- UiState::helix_editing is an application preference; CanvasState::helix travels with the existing per-window canvas state. Document switches reset mode and pending input.
- crates/ui-egui/src/keys/helix/config.rs:32: Config implements FromStr, bounded native configuration loading, XDG path resolution, normalized key names and per-mode binding lookup. Configuration is held once per application and shared across document windows.
- Uses: the existing locked toml dependency, wordcraft_engine::io::read_regular_file_within, native environment variables, Mode and WordApp::helix_config.
- Used by: Helix enablement, the canvas event adapter, explicit configuration reload and configuration regression tests.
- crates/ui-egui/src/keys/helix/bindings.rs:287: custom dispatches nested user bindings; execute validates command arrays; clipboard_copy and clipboard_paste enforce the configured clipboard policy.
- Uses: Config::binding, existing Helix editing helpers, WordApp::run, native clipboard services and the single-selection model.
- Used by: modal key and committed-text handlers, built-in system-clipboard bindings and keyboard regression tests.
- Default y, d, c, p, P and R use clipboard_copy and clipboard_paste, including configured aliases. execute checks their clipboard policy before running command arrays. The private Helix register and its yank/paste helpers are removed.
- crates/ui-egui/src/keys/canvas.rs:171: note_key_state records key presses and paste modifiers before Helix consumes events, preventing a modal v release from inheriting stale paste modifiers.
- Uses: CanvasState::v_pressed, CanvasState::paste_mods, shortcuts::is_modifier_key.
- Used by: canvas_events and native paste-release fallback.
- Native clipboard provider names choose WordCraft's existing clipboard integration rather than executing the provider's Helix helper programs. Browser builds retain default keymaps without reading environment variables or local files.

## Helix revision history

- 2026-11-17: Added command and keyboard-adapter symbol references for optional Helix modal editing.
- 2026-11-17: Added XDG configuration and custom binding modules; reused the existing TOML parser and bounded file reader.
- 2026-11-17: Removed Helix register storage and reused system clipboard helpers; moved paste-key tracking before modal dispatch.

## Architecture overview

> **Last reviewed:** 2026-10-28 · **Last updated:** 2026-11-05 · **Change:** trivial (the hostile-parameter test now has a sibling for `file.*`, `insert.picture` and `insert.textFromFile`; previously trivial: craft-proof's row says what a build without the Language data keeps: English spelling from an installed Hunspell English or the built-in list, rule hyphenation for Bulgarian, Turkish and Russian; previously minor: proofing moved out: `wordcraft-proof` is craft-libs' `craft-proof`, its Language data craft-i18n's `language-data/` embedded from `CRAFT_I18N_DIR`; previously major: crate and layer inventory, source counts, proofing languages and module boundaries re-measured from this checkout; previously minor: `proof` is per language: English and Polish; previously major: first committed version; `plan/architecture.md` is local-only) · **Target:** Microsoft Word (Microsoft 365) for Mac 16.113.4

How WordCraft is built today. The longer design notes in
`plan/architecture.md` are gitignored and local to the original author's machine; this file is the
committed, current summary. Rules for working in the code are in [`AGENTS.md`](../AGENTS.md).

## Layers

On 2026-10-28, `find crates apps xtask -type f -name '*.rs' -print0 | xargs -0 wc -l` counted
333,448 Rust source lines across 16 crates, 3 apps and `xtask`. `Cargo.toml` declares workspace
members with `crates/*`, `apps/*` and `xtask`; `cargo xtask layers` verified 20 workspace packages
(16 crates, 3 apps and `xtask`) and no layering violations. The layer and line counts below are
from that command and per-directory `find ... -name '*.rs' ... | wc -l` counts on that date. Each
crate name links to its source tree.

| Layer | Crate | Lines | Job |
|---|---|---|---|
| L0 standalone | [`control-key`](../crates/control-key/src) | 87 | Per-window control-port key files shared by the app and MCP bridge |
| L0 | [`geom`](../crates/geom/src) | 1,271 | Units (points, twips, EMU, inches, cm), rectangles and measurement parsing |
| L1 | [`doc`](../crates/doc/src) | 23,719 | Document model: stories, paragraphs, runs, tables, sections, styles, numbering, fields, notes, comments, revisions and equations; edit primitives and bidi |
| L1 | [`fonts`](../crates/fonts/src) | 7,088 | Font database, metrics, outlines, HarfRust shaping, Word font substitution and OpenType MATH |
| L1 | [`metafile`](../crates/metafile/src) | 3,614 | Shared Windows Metafile parsing and drawing primitives used by format readers and renderers |
| L1 (external) | [`craft-proof`](https://github.com/storytold/craft-libs/tree/main/crates/proof) (package alias `wordcraft-proof`) | — | Proofing moved to craft-libs (CP1, 2026-11): fifteen spelling dictionaries: English (US, GB, CA), Polish, Portuguese, Italian, Croatian, Turkish, Russian, Czech, Indonesian and Spanish (Spain, Mexico, Latin America, Uruguay); language modules provide spelling, suggestions and grammar, while Bulgarian and German have hyphenation only. Its word lists and patterns are craft-i18n's `language-data/`, embedded from `CRAFT_I18N_DIR` at the commit in `craft-i18n.rev` (CI and releases set `CRAFT_I18N_REQUIRED=1`); without them English spelling falls back to an installed Hunspell English or craft-proof's built-in Moby list, and pattern hyphenation is off (Bulgarian, Turkish and Russian hyphenate by rule) |
| L2 | [`docbin`](../crates/docbin/src) | 8,815 | Word 97–2003 `.doc` reader (FIB, piece table, FKPs, SPRMs, stylesheet, lists, tables, notes, pictures) over a CFB container |
| L2 | [`docx`](../crates/docx/src) | 37,376 | OOXML WordprocessingML reader/writer (Transitional and Strict), OMML, custom properties and preserved VBA parts |
| L2 | [`formats`](../crates/formats/src) | 23,058 | TXT, Markdown, HTML, RTF, ODT and LaTeX import/export through the shared document model |
| L2 | [`layout`](../crates/layout/src) | 35,990 | Paragraph line layout (`para`), page/section flow and table layout, then display-list generation (`display`); hit testing maps points to document positions. `para` owns shaping, breaks, tabs and line geometry; page flow composes paragraphs, tables, headers/footers, notes and floats. |
| L3 | [`pdf`](../crates/pdf/src) | 4,513 | krilla PDF export: font subsets, selectable text, links, outline and tagging |
| L3 | [`render`](../crates/render/src) | 4,499 | vello_cpu rasterizer: pages and thumbnails from the display list; PNG/JPEG |
| L4 | [`engine`](../crates/engine/src) | 72,851 | `Session` and editing state; command registry in `cmd/`; file dispatch in `io`; catalog, keymap, recovery, ribbon model, speech and view state |
| L5 | [`mcp`](../crates/mcp/src) | 1,822 | MCP server exposing commands as tools; headless or bridged to the running app |
| L5 | [`zotero`](../crates/zotero/src) | 2,372 | Zotero word-processor integration protocol answered against a `Session` |
| L6 | [`ui-egui`](../crates/ui-egui/src) | 81,759 | Front end: ribbon, keytips, mini-toolbar, Backstage, canvas, rulers, panes, dialogs, i18n, themes, control channel and headless `ui_shot` example |
| apps | [`wordcraft`](../apps/wordcraft/src) | 3,740 | Desktop app (eframe + wgpu; logging, window geometry and file associations) |
| apps | [`wordcraft-cli`](../apps/wordcraft-cli/src) | 1,901 | `convert`, `text`, `inspect`, `run`, `commands`, `parity`, `mcp` and `zotero` |
| apps | [`wordcraft-web`](../apps/wordcraft-web/src) | 1,159 | Browser build (eframe web runner, WebGPU with WebGL2 fallback) |
| tooling | [`xtask`](../xtask/src) | 1,488 | Workspace maintenance and quality-gate commands, including `layers` |

## Data flow

1. **Open:** `engine::io` picks a reader by extension (`docx`, `docbin`, `formats`, JSON) and
   produces a `doc::Document`.
2. **Edit:** every user-visible action is a command id run through `Session::run` (ribbon,
   shortcuts, command search, CLI, control channel and MCP all dispatch by id). `Session::run`
   snapshots for undo (blocks are `Arc`-shared, so snapshots are cheap) and catches an escaped
   panic, restoring the document (never-crash standard).
3. **Layout:** `layout` turns the document into pages of positioned glyphs, boxes and images
   (display list), incrementally: an edit relayouts only what changed (1.4 ms on a 188-page
   document, measured 2026-10-06).
4. **Draw:** the UI paints pages through `render` (CPU raster, cached per page, pixel-aligned) on
   an egui/wgpu canvas; PDF export walks the same display list through `pdf`.
5. **Save:** `engine::io` picks a writer by extension.

## Testing and gates

- 821 `#[test]` functions and 10 property-test blocks: model, layout, DOCX round-trip and hostile
  input, `.doc`, formats, typing parity (`engine/src/tests/typing.rs`), bidi, i18n catalogs, UI
  interaction (egui_kittest), MCP acceptance; `hostile_file_params_never_panic` runs `file.*`,
  `insert.picture` and `insert.textFromFile` with junk params and unusable paths, held to a
  temporary folder, and `hostile_params_never_panic` every other command with junk params.
- `cargo xtask ci`: fmt, clippy `-D warnings`, tests, asset attribution, layering, wasm build.
- `cargo xtask parity` regenerates [`parity-checklist.md`](parity-checklist.md); a test keeps the
  catalog percentage above a floor that only rises.
- GitHub Actions on PRs: packaging lint, and FreeBSD / Windows ARM64 when build files change;
  release builds on pushes to `release`.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-05 | trivial | `hostile_file_params_never_panic` runs `file.*`, `insert.picture` and `insert.textFromFile` with junk params and unusable paths (#3719) |
| 2026-11-02 | trivial | craft-proof's row: without the Language data English spelling falls back to an installed Hunspell English or the built-in Moby list, and only pattern hyphenation is off (#5202 review) |
| 2026-11-01 | minor | `proof` left the workspace: it is `craft-proof` in storytold/craft-libs (with its history), and the spelling and hyphenation data is craft-i18n's `language-data/` (`CRAFT_I18N_DIR`); `assets/spelling` and `assets/hyphenation` are gone |
| 2026-10-28 | major | Recounted workspace crates, layers and Rust source lines; refreshed proofing languages and documented current engine/layout module boundaries and source links |
| 2026-10-10 | minor | `proof`: proofing languages (English, Polish), affix engine, Polish patterns; layout and engine route each run by its language |
| 2026-10-10 | major | First committed architecture doc, from the crates' sources and `AGENTS.md` |
