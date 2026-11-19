# Native window chrome

- title_bar, crates/ui-egui/src/chrome.rs:308. Preserves custom's hidden Discord button and fits the search field to the remaining space.
  - Uses: TitleBarFit::new with no Discord button, search_box.
  - Used by: WordApp::paint_window.
- TitleBarFit, crates/ui-egui/src/chrome.rs:458. Calculates responsive search width for title-bar controls.
  - Uses: available room and control height.
  - Used by: title_bar, chrome tests.
- search_box, crates/ui-egui/src/chrome.rs:614. Draws the search control at the supplied width and opens command search.
  - Uses: typing_search, icons::paint.
  - Used by: title_bar, full-screen controls.
- status_bar, crates/ui-egui/src/chrome.rs:758. Fits Helix's mode indicator alongside the native status controls.
  - Uses: optional status-item responses, keys::helix::status, ui.helixMode.
  - Used by: WordApp::paint_window.
