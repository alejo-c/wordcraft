# Paragraph style spotlight

Status: 2026-11-17, badges sit outside the page in a dedicated canvas gutter. Desktop interaction still needs manual verification.

## Usage

- Enable Spotlight under View, Show, Spotlight or in the Styles pane.
- Paragraphs show solid colored blocks outside the page labeled with their paragraph style name. Hover reveals the full name.
- Paragraph styles in the existing Styles pane show the same color. Character styles keep their existing character marker.
- Dock the Styles pane on the left to move blocks to the right; docking on the right moves blocks to the left. Floating uses left blocks.
- Native style application, context menus, ribbon controls and dialogs retain their existing command paths.
- The saved spotlight preference applies across document windows. No document formatting or export data is added.
- Styles come from the open document. This feature does not bundle or install an external style template.

## Rendering constraints

- Paint inside the native canvas with its shown document and layout, including temporary formatting and table previews.
- Only collect paragraph fragments on visible pages. Merge wrapped fragments by story and paragraph path within each page.
- Reuse Placed::vertical_span for normal and rotated cell text. Keep page bands when white space is hidden.
- Colors depend on style identity rather than translated style names or theme. Both renderers call the same function.
- Reserve a screen-space gutter per page. Right-side badges begin beyond the comment balloon column. Clip badges to the canvas viewport and the visible vertical page band.
- Spotlight is a screen overlay, outside the page raster cache and document model.

## Verification

- 2026-11-17: All five Spotlight tests passed. Headless screenshots show the badges outside the page on both sides.
- 2026-11-17: Native crate check and WebAssembly check passed.

Initial integration checks, 2026-11-13

- cargo fmt --check -p wordcraft-ui-egui passed. Workspace formatting passed through cargo xtask ci.
- cargo c -qp wordcraft-ui-egui passed.
- cargo clippy --workspace --all-targets -- -D warnings passed after the final changes.
- All four spotlight regression tests passed, including native style-button interaction and saved visibility.
- Full native UI tests: 1243 passed, 2 failed and 3 ignored, with the translation audit run separately.
- UI failures: backstage::tests::rtl_template_titles_are_right_aligned and theme::tests::interface_symbols_have_glyphs. The latter reports U+3002 in existing tests.rs.
- cargo xtask ci stopped in wordcraft-engine: 1434 passed and 26 failed. Failures cover Fit One Page, continuous-view navigation, ODT frame anchoring and language-pack rules. Engine, layout, fonts, manifests and lockfile are unchanged by this branch.
- cargo xtask wasm passed. Existing WASM warnings report unused dialog imports and system_arabic in theme.rs.
- Asset attribution, dependency layering, interface translation audit, language-data check and workspace documentation tests passed independently.
- Cargo reports existing unused egui patch entries. No dependency or patch changes were made for spotlight.
- Inspected headless native screenshots in light and dark themes with the Styles pane on either side. Native desktop interaction remains for manual verification.

## Symbols

- gutter and on_left, crates/ui-egui/src/spotlight.rs:10: shared badge width, page gap and side preference.
  Uses: UiState::spotlight, StylesPaneDock.
  Used by: geometry, show, balloons.

- continuous_gutter, crates/ui-egui/src/spotlight.rs:18: space reserved for badges and any intervening markup in Read Mode and Web Layout.
  Uses: gutter, on_left, canvas::markup_width.
  Used by: geometry, update_viewport, reading_step.

- canvas::geometry, crates/ui-egui/src/canvas/geometry.rs:173: reserves the Spotlight gutter in page slots and fit calculations.
  Uses: gutter, continuous_gutter, place_pages, place_read_pages.
  Used by: canvas::prepare_view.

- reading_step, crates/ui-egui/src/canvas/reading.rs:128: horizontal distance between reading columns with their gutters.
  Uses: continuous_gutter, ViewState::read_width.
  Used by: Read Mode viewport and navigation.

- StyleBlock, crates/ui-egui/src/spotlight.rs:24: style identity, translated display name and vertical bounds of a page fragment.
  Uses: Document, Page, Placed.
  Used by: blocks, block_rect, paint_page.

- blocks, crates/ui-egui/src/spotlight.rs:31: groups laid-out paragraph fragments on one page and resolves their style names.
  Uses: Placed::vertical_span, Document::para, i18n::style_name.
  Used by: paint_page, spotlight tests.

- color, crates/ui-egui/src/spotlight.rs:59: assigns a stable solid RGB color to a style identity.
  Uses: Color32.
  Used by: indicator, paint_page, spotlight tests.

- indicator, crates/ui-egui/src/spotlight.rs:64: draws a paragraph-style color indicator in the native Styles pane.
  Uses: color, Ui::allocate_exact_size.
  Used by: panes::side::styles.

- block_rect, crates/ui-egui/src/spotlight.rs:70: maps paragraph bounds to the gutter on either side of the page at the current screen scale.
  Uses: Page width, canvas markup width, StyleBlock.
  Used by: paint_page, spotlight tests.

- paint_page, crates/ui-egui/src/spotlight.rs:79: draws clipped paragraph labels and matching solid blocks on one visible page.
  Uses: blocks, color, block_rect, theme::regular.
  Used by: show.

- show, crates/ui-egui/src/spotlight.rs:97: selects visible page bands and places badges opposite the docked Styles pane.
  Uses: paint_page, canvas::page_doc, Geometry::bands, StylesPaneDock.
  Used by: canvas::show.

- UiState::spotlight, crates/ui-egui/src/lib.rs:332: persistent visibility preference, disabled by default.
  Uses: UiState serialization, WordApp::prefs, WordApp::apply_prefs.
  Used by: canvas::show, panes::side::styles, ribbon::view, WordApp::ui_command, WordApp::swap_window.

- WordApp::ui_command, crates/ui-egui/src/app/commands.rs:522: handles ui.spotlight with an optional boolean value; no value toggles.
  Uses: UiState::spotlight.
  Used by: WordApp::execute, View checkbox, Styles pane checkbox.

- panes::side::styles, crates/ui-egui/src/panes/side.rs:46: keeps native style actions and adds the spotlight toggle and paragraph indicators.
  Uses: indicator, ui.spotlight, para.style, existing style commands.
  Used by: native pane host.

- canvas::show, crates/ui-egui/src/canvas.rs:1630: paints spotlight beside the shown page bands, opposite the docked Styles pane.
  Uses: page_doc, spotlight::show, Geometry::bands, StylesPaneDock.
  Used by: WordApp::ui.

- WordApp::swap_window, crates/ui-egui/src/window_ui.rs:160: spotlight stays in the application-wide preference set.
  Uses: UiState::spotlight.
  Used by: document window switching.

- spotlight tests, crates/ui-egui/src/spotlight/tests.rs: paragraph wrapping and pagination, stable colors, saved visibility, outside-page placement and native style interaction.
  Uses: Session, WordApp, egui_kittest.
  Used by: cargo test -p wordcraft-ui-egui spotlight.

## Revision history

- 2026-11-17: Moved the paragraph labels into the canvas gutter and reserved space for them in page rows.
