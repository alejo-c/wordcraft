# WordCraft TODO

## Pending

- Verify Web Layout's compact line-number gutter in the browser after changing document page margins.
- Verify Helix Normal/Select switching after modifier shortcuts and clipboard-only copy/paste on desktop and web.
  - Source: docs/roadmap.md and docs/ui-parity.md.
  - Hook: canvas_events and the Helix keyboard adapter in crates/ui-egui/src/keys/.
- Verify Helix viewport alignment, caret shape after clicking, relative Layout line numbers and active-line shading on desktop and web.
- Verify Ctrl+F and Ctrl+E in Helix Normal and Insert on desktop and web.

## Deferred / low priority

- Resolve the existing full quality gate failures tracked in docs/roadmap.md.
- Resolve engine test failures in Swiss German/Portuguese language rules and moved ODT frame page anchoring.
- Investigate UI library failures in backstage::tests::rtl_template_titles_are_right_aligned and theme::tests::interface_symbols_have_glyphs.

## DO-NOT list (load-bearing constraints)

- Keep project context in docs/; do not create or read context/rust.md or docs/index.md.
- Keep Helix copy/paste on the user's system clipboard; do not add registers.
- Preserve native paste releases whose key press is consumed by the platform.

## Known commented-out code (do not re-enable without asking)

## Test commands

- cargo c -qp wordcraft-ui-egui
- cargo test -p wordcraft-ui-egui --lib keys::
- cargo xtask ci

## External references
