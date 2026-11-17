# WordCraft TODO

## Pending

- Verify spotlight manually in the native desktop app with real DOCX and ODT documents, including zoom, scrolling and docked styles.
- Verify Helix modal editing interactively on the native desktop and web alongside Spotlight.

## Deferred / low priority

- Investigate the engine CI failures in Fit One Page, continuous-view navigation, ODT frame anchoring and language-pack rules.
- Investigate existing UI test failures for RTL template-caption alignment and U+3002 glyph coverage; see docs/spotlight.md.
- Align Spotlight tests and documentation with custom's disabled-by-default preference; two Spotlight regressions currently assume enabled.

## DO-NOT list (load-bearing constraints)

- Keep spotlight out of document formatting and exports; preserve native editing controls.

## Known commented-out code (do not re-enable without asking)

## Test commands

- cargo xtask ci
- cargo test -p wordcraft-ui-egui spotlight
- cargo test -p wordcraft-ui-egui keys::helix
- cargo test -p wordcraft-engine helix

## External references
