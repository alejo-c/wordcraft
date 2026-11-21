# WordCraft TODO

## Deferred / low priority

- Investigate the quality gate failures tracked in docs/roadmap.md, including Fit One Page, continuous-view navigation, ODT frame anchoring and language-pack rules.
- Investigate RTL template-caption alignment and U+3002 glyph coverage in backstage::tests::rtl_template_titles_are_right_aligned and theme::tests::interface_symbols_have_glyphs. See docs/spotlight.md.
- Align Spotlight tests and documentation with custom's disabled-by-default preference; two Spotlight regressions currently assume enabled.
- Resolve the existing full quality gate failures tracked in docs/roadmap.md.
- Resolve engine test failures in Swiss German/Portuguese language rules and moved ODT frame page anchoring.
- Investigate UI library failures in backstage::tests::rtl_template_titles_are_right_aligned and theme::tests::interface_symbols_have_glyphs.

## DO-NOT list (load-bearing constraints)

- Keep spotlight out of document formatting and exports; preserve native editing controls.
- Keep project context in docs/; do not create or read context/rust.md or docs/index.md.
- Keep Helix copy/paste on the user's system clipboard; do not add registers.
- Preserve native paste releases whose key press is consumed by the platform.

## Known commented-out code (do not re-enable without asking)

## Test commands

- cargo xtask ci
- cargo c -qp wordcraft-ui-egui
- cargo test -p wordcraft-ui-egui spotlight
- cargo test -p wordcraft-ui-egui keys::helix
- cargo test -p wordcraft-ui-egui --lib keys::
- cargo test -p wordcraft-engine helix

## External references
