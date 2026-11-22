.
├── AGENTS.md
├── ATTRIBUTION.md
├── CLAUDE.md
├── Cargo.lock
├── Cargo.toml
├── GLOSSARY.md
├── LICENSE-APACHE
├── LICENSE-MIT
├── NOTICE
├── README.md
├── ROADMAP.md
├── SECURITY.md
├── apps
│   ├── wordcraft
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── src
│   │   └── tests
│   ├── wordcraft-cli
│   │   ├── Cargo.toml
│   │   ├── src
│   │   └── tests
│   └── wordcraft-web
│       ├── Cargo.toml
│       ├── Trunk.toml
│       ├── index.html
│       └── src
├── assets
│   ├── app-icon
│   │   ├── hicolor
│   │   ├── wordcraft-1024.png
│   │   ├── wordcraft-macos-512.png
│   │   ├── wordcraft.icns
│   │   ├── wordcraft.ico
│   │   └── wordcraft.svg
│   └── fonts
│       ├── Inter-Medium.ttf
│       ├── Inter-Regular.ttf
│       ├── Inter-SemiBold.ttf
│       ├── JetBrainsMono-Regular.ttf
│       ├── OFL-Inter.txt
│       ├── OFL-JetBrainsMono.txt
│       ├── OFL-SourceSans3.txt
│       ├── OFL-SourceSerif4.txt
│       ├── SourceSans3-Bold.ttf
│       ├── SourceSans3-It.ttf
│       ├── SourceSans3-Regular.ttf
│       ├── SourceSans3-Semibold.ttf
│       ├── SourceSerif4-Bold.ttf
│       ├── SourceSerif4-BoldIt.ttf
│       ├── SourceSerif4-It.ttf
│       ├── SourceSerif4-Regular.ttf
│       └── SourceSerif4-Semibold.ttf
├── clippy.toml
├── contributors
│   ├── contributors.json
│   └── people.toml
├── craft-i18n.rev
├── crates
│   ├── control-key
│   │   ├── Cargo.toml
│   │   └── src
│   ├── doc
│   │   ├── Cargo.toml
│   │   ├── src
│   │   └── tests
│   ├── docbin
│   │   ├── Cargo.toml
│   │   └── src
│   ├── docx
│   │   ├── Cargo.toml
│   │   ├── src
│   │   └── tests
│   ├── engine
│   │   ├── Cargo.toml
│   │   ├── examples
│   │   ├── src
│   │   └── tests
│   ├── fonts
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── src
│   │   └── tests
│   ├── formats
│   │   ├── Cargo.toml
│   │   └── src
│   ├── geom
│   │   ├── Cargo.toml
│   │   └── src
│   ├── layout
│   │   ├── Cargo.toml
│   │   └── src
│   ├── mcp
│   │   ├── Cargo.toml
│   │   ├── src
│   │   └── tests
│   ├── pdf
│   │   ├── Cargo.toml
│   │   └── src
│   ├── render
│   │   ├── Cargo.toml
│   │   └── src
│   ├── ui-egui
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── examples
│   │   ├── icons
│   │   ├── src
│   │   └── tests
│   └── zotero
│       ├── Cargo.toml
│       └── src
├── docs
│   ├── architecture.md
│   ├── brand
│   │   ├── LICENSE-brand.txt
│   │   ├── artcraft-logo-white.png
│   │   ├── artcraft-logo-white.svg
│   │   ├── artcraft-logo.png
│   │   ├── artcraft-logo.svg
│   │   ├── artcraft-mark-black.png
│   │   ├── artcraft-mark-black.svg
│   │   ├── artcraft-mark.png
│   │   └── artcraft-mark.svg
│   ├── context
│   │   └── structure.md
│   ├── contributors.md
│   ├── control-protocol.md
│   ├── egui-037-migration-plan.md
│   ├── feature-names.md
│   ├── file-format-parity.md
│   ├── gaps.md
│   ├── hardware-parity.md
│   ├── images
│   │   ├── backstage.png
│   │   ├── dark.png
│   │   ├── design.png
│   │   ├── hero.png
│   │   ├── layout.png
│   │   ├── references.png
│   │   └── review.png
│   ├── index.md
│   ├── layout-parity.md
│   ├── localization-bs.md
│   ├── localization-hu.md
│   ├── localization-parity.md
│   ├── localization-sl.md
│   ├── localization-sv.md
│   ├── localization-tr.md
│   ├── mardok-map.md
│   ├── mardok.md
│   ├── mcp.md
│   ├── odf-interpretation.md
│   ├── odt-element-audit.md
│   ├── parity-checklist.md
│   ├── recovery.md
│   ├── releasing.md
│   ├── roadmap.md
│   ├── shared-pdf.md
│   ├── spelling-dictionaries.md
│   ├── spelling-review.md
│   ├── target-app-parity.md
│   ├── testing.md
│   ├── typing-parity.md
│   ├── ui-parity.md
│   └── zotero.md
├── packaging
│   ├── env.sh
│   ├── freebsd
│   │   └── package.sh
│   ├── icons.sh
│   ├── linux
│   │   ├── ai.storyteller.wordcraft.desktop
│   │   ├── ai.storyteller.wordcraft.metainfo.xml.in
│   │   ├── ai.storyteller.wordcraft.mime.xml
│   │   ├── flatpak
│   │   ├── flatpak-bundle.sh
│   │   ├── flatpak-oci.sh
│   │   ├── nfpm.yaml
│   │   ├── package.sh
│   │   └── postinst.sh
│   ├── macos
│   │   ├── Info.plist.in
│   │   ├── dmg
│   │   ├── entitlements.plist
│   │   ├── import-cert.sh
│   │   └── package.sh
│   ├── web
│   │   ├── README.md
│   │   ├── _headers
│   │   └── package.sh
│   └── windows
│       ├── INSTALLATION.md
│       ├── check-install-policy.ps1
│       ├── install-policy.json
│       ├── install-policy.wxi
│       ├── package.ps1
│       ├── sign.ps1
│       ├── test-install-policy.ps1
│       ├── test-packaging-script.ps1
│       └── wordcraft.wxs
├── rustfmt.toml
├── sonar-project.properties
└── xtask
    ├── Cargo.toml
    └── src
        ├── assets.rs
        ├── ico.rs
        ├── layers.rs
        ├── main.rs
        ├── parity.rs
        ├── stats.rs
        ├── version.rs
        └── visual_diff.rs

67 directories, 148 files
