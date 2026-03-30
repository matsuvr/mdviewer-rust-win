# Markdown Viewer

A fast, read-only Markdown viewer for Windows x64.

## What this repository contains

This repository packages `crates\markdown_viewer` as a standalone Windows application. The surrounding workspace is a curated subset of the Zed codebase that remains necessary to build the viewer and render Markdown with Zed's existing UI, theme, asset, and parsing crates.

## Current scope

- Windows x64 only
- read-only Markdown viewing
- local Markdown links resolved relative to the current document
- external URLs opened in the system browser
- math, Mermaid diagrams, code blocks, bundled fonts, and bundled themes inherited from the retained Zed stack

## Relationship to Zed

This project is derived in part from the Zed repository and reworked into an independent Markdown viewer.

- Upstream project: `https://github.com/zed-industries/zed`
- Product entry point in this repository: `crates\markdown_viewer`
- This project is **not** the Zed editor
- This project is **not affiliated with or endorsed by Zed Industries**

Large portions of the implementation were adapted from Zed crates. Copyright in those portions remains with their respective original authors and contributors.

## Building and running

`markdown_viewer` currently targets `x86_64-pc-windows-msvc` only.

```powershell
cargo run -p markdown_viewer -- path\to\file.md
cargo build -p markdown_viewer --release
```

Running without a path opens an empty viewer window.

## Licensing

The repository and release artifacts are distributed under `GPL-3.0-or-later`. A copy of the GPL text is provided in `LICENSE` for GitHub license detection and in `LICENSE-GPL` to preserve the upstream layout.

This repository also retains Apache-2.0-licensed crates from Zed, especially the `gpui` platform stack and shared utility crates. Their original notices remain in place, and the corresponding license text is included in `LICENSE-APACHE`.

The upstream tree still contains `LICENSE-AGPL`, but the current `markdown_viewer` dependency closure does not include AGPL-licensed workspace crates.

For the current Windows viewer build, the normal workspace dependency closure of `markdown_viewer` contains `50` internal workspace crates:

- `35` crates under `GPL-3.0-or-later`
- `15` crates under `Apache-2.0`
- `0` AGPL crates in the shipped viewer dependency closure

See `THIRD_PARTY_LICENSES.md` for the crate-by-crate inventory and the source of each license decision.

### New code in this fork

New standalone code written specifically for this fork may be licensed under `Apache-2.0` when it does not copy or adapt GPL-derived material. If a file is derived from a GPL-licensed Zed crate, or is added inside a GPL-derived crate and forms part of that adapted work, it should remain GPL-compatible. Regardless of individual file terms, redistribution of the combined application remains `GPL-3.0-or-later`.

## Third-party dependency notices

External dependencies from crates.io and other upstreams are tracked separately with `cargo-about`.

Generate the release-time notice bundle with:

```powershell
script\generate-licenses.ps1 assets\licenses.md
```

Include the generated `assets\licenses.md` file (or a copied equivalent) with any binary release.

## Source availability

If binaries are published, the corresponding source code is the matching Git tag and source archive in this repository.

## Repository notes

Some upstream Zed documentation is still present in `docs\` and other folders for reference. Unless a page explicitly mentions `markdown_viewer`, treat it as upstream background material rather than product documentation for this standalone viewer.

## Credits

This project would not exist without the Zed project and its contributors.
