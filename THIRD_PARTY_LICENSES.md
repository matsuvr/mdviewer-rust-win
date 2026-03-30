# Third-Party and Upstream Licensing

## What this file covers

This document describes the first-party and upstream workspace crates that are part of the `markdown_viewer` binary for the `x86_64-pc-windows-msvc` target. It is intentionally focused on code retained from the Zed workspace and used by the current Windows viewer build.

The combined repository is distributed under `GPL-3.0-or-later`; see `LICENSE`. That combined distribution still contains crates that remain individually marked `Apache-2.0`, so their original notices and license texts are preserved in this repository.

## Upstream origin

- Upstream project: `https://github.com/zed-industries/zed`
- Current application entry point: `crates\markdown_viewer`
- Inventory method: `cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-msvc`, filtered to the normal dependency closure of `markdown_viewer`

## Summary

- Normal workspace dependency closure: `50` crates
- Effective `GPL-3.0-or-later` crates: `35`
- Effective `Apache-2.0` crates: `15`
- Crates resolved via adjacent `LICENSE-*` file instead of `Cargo.toml`: `2`
- `AGPL` crates in the current `markdown_viewer` closure: `0`

## Repository-wide licensing model

1. `LICENSE` is the repository-wide distribution license for the combined viewer application.
2. `LICENSE-GPL`, `LICENSE-APACHE`, and `LICENSE-AGPL` are retained from the upstream source tree.
3. The current viewer build does not include AGPL workspace crates.
4. New standalone files may be authored under `Apache-2.0` when they do not incorporate GPL-derived material. Once shipped together with GPL-derived code, the combined application remains distributed under `GPL-3.0-or-later`.
5. Upstream license symlinks are part of the compliance story; `script\check-licenses` verifies that each crate carries the expected `LICENSE-GPL`, `LICENSE-APACHE`, or `LICENSE-AGPL` link.

## Workspace crate inventory

| Crate | Effective license | Evidence | Note |
|---|---|---|---|
| `askpass` | `GPL-3.0-or-later` | `crates\askpass\Cargo.toml` | Declared in Cargo.toml. |
| `assets` | `GPL-3.0-or-later` | `crates\assets\Cargo.toml` | Declared in Cargo.toml. |
| `clock` | `GPL-3.0-or-later` | `crates\clock\Cargo.toml` | Declared in Cargo.toml. |
| `collections` | `Apache-2.0` | `crates\collections\Cargo.toml` | Declared in Cargo.toml. |
| `component` | `GPL-3.0-or-later` | `crates\component\Cargo.toml` | Declared in Cargo.toml. |
| `derive_refineable` | `Apache-2.0` | `crates\refineable\derive_refineable\Cargo.toml` | Declared in Cargo.toml. |
| `fs` | `GPL-3.0-or-later` | `crates\fs\Cargo.toml` | Declared in Cargo.toml. |
| `fuzzy` | `GPL-3.0-or-later` | `crates\fuzzy\Cargo.toml` | Declared in Cargo.toml. |
| `git` | `GPL-3.0-or-later` | `crates\git\Cargo.toml` | Declared in Cargo.toml. |
| `gpui` | `Apache-2.0` | `crates\gpui\Cargo.toml` | Declared in Cargo.toml. |
| `gpui_macros` | `Apache-2.0` | `crates\gpui_macros\Cargo.toml` | Declared in Cargo.toml. |
| `gpui_platform` | `Apache-2.0` | `crates\gpui_platform\Cargo.toml` | Declared in Cargo.toml. |
| `gpui_util` | `Apache-2.0` | `crates\gpui_util\LICENSE-APACHE` | Cargo.toml omits `license`; repository license symlink supplies the Apache notice. |
| `gpui_windows` | `Apache-2.0` | `crates\gpui_windows\Cargo.toml` | Declared in Cargo.toml. |
| `http_client` | `Apache-2.0` | `crates\http_client\Cargo.toml` | Declared in Cargo.toml. |
| `icons` | `GPL-3.0-or-later` | `crates\icons\Cargo.toml` | Declared in Cargo.toml. |
| `language` | `GPL-3.0-or-later` | `crates\language\Cargo.toml` | Declared in Cargo.toml. |
| `language_core` | `GPL-3.0-or-later` | `crates\language_core\LICENSE-GPL` | Cargo.toml omits `license`; repository license symlink supplies the GPL notice. |
| `lsp` | `GPL-3.0-or-later` | `crates\lsp\Cargo.toml` | Declared in Cargo.toml. |
| `markdown` | `GPL-3.0-or-later` | `crates\markdown\Cargo.toml` | Declared in Cargo.toml. |
| `markdown_viewer` | `GPL-3.0-or-later` | `crates\markdown_viewer\Cargo.toml` | Declared in Cargo.toml. |
| `menu` | `GPL-3.0-or-later` | `crates\menu\Cargo.toml` | Declared in Cargo.toml. |
| `migrator` | `GPL-3.0-or-later` | `crates\migrator\Cargo.toml` | Declared in Cargo.toml. |
| `net` | `GPL-3.0-or-later` | `crates\net\Cargo.toml` | Declared in Cargo.toml. |
| `paths` | `GPL-3.0-or-later` | `crates\paths\Cargo.toml` | Declared in Cargo.toml. |
| `perf` | `Apache-2.0` | `tooling\perf\Cargo.toml` | Declared in Cargo.toml. |
| `proto` | `GPL-3.0-or-later` | `crates\proto\Cargo.toml` | Declared in Cargo.toml. |
| `refineable` | `Apache-2.0` | `crates\refineable\Cargo.toml` | Declared in Cargo.toml. |
| `release_channel` | `GPL-3.0-or-later` | `crates\release_channel\Cargo.toml` | Declared in Cargo.toml. |
| `rope` | `GPL-3.0-or-later` | `crates\rope\Cargo.toml` | Declared in Cargo.toml. |
| `rpc` | `GPL-3.0-or-later` | `crates\rpc\Cargo.toml` | Declared in Cargo.toml. |
| `scheduler` | `Apache-2.0` | `crates\scheduler\Cargo.toml` | Declared in Cargo.toml. |
| `settings` | `GPL-3.0-or-later` | `crates\settings\Cargo.toml` | Declared in Cargo.toml. |
| `settings_content` | `GPL-3.0-or-later` | `crates\settings_content\Cargo.toml` | Declared in Cargo.toml. |
| `settings_json` | `GPL-3.0-or-later` | `crates\settings_json\Cargo.toml` | Declared in Cargo.toml. |
| `settings_macros` | `GPL-3.0-or-later` | `crates\settings_macros\Cargo.toml` | Declared in Cargo.toml. |
| `sum_tree` | `Apache-2.0` | `crates\sum_tree\Cargo.toml` | Declared in Cargo.toml. |
| `task` | `GPL-3.0-or-later` | `crates\task\Cargo.toml` | Declared in Cargo.toml. |
| `text` | `GPL-3.0-or-later` | `crates\text\Cargo.toml` | Declared in Cargo.toml. |
| `theme` | `GPL-3.0-or-later` | `crates\theme\Cargo.toml` | Declared in Cargo.toml. |
| `theme_settings` | `GPL-3.0-or-later` | `crates\theme_settings\Cargo.toml` | Declared in Cargo.toml. |
| `ui` | `GPL-3.0-or-later` | `crates\ui\Cargo.toml` | Declared in Cargo.toml. |
| `ui_macros` | `GPL-3.0-or-later` | `crates\ui_macros\Cargo.toml` | Declared in Cargo.toml. |
| `util` | `Apache-2.0` | `crates\util\Cargo.toml` | Declared in Cargo.toml. |
| `util_macros` | `Apache-2.0` | `crates\util_macros\Cargo.toml` | Declared in Cargo.toml. |
| `watch` | `Apache-2.0` | `crates\watch\Cargo.toml` | Declared in Cargo.toml. |
| `zed_actions` | `GPL-3.0-or-later` | `crates\zed_actions\Cargo.toml` | Declared in Cargo.toml. |
| `zlog` | `GPL-3.0-or-later` | `crates\zlog\Cargo.toml` | Declared in Cargo.toml. |
| `ztracing` | `GPL-3.0-or-later` | `crates\ztracing\Cargo.toml` | Declared in Cargo.toml. |
| `ztracing_macro` | `GPL-3.0-or-later` | `crates\ztracing_macro\Cargo.toml` | Declared in Cargo.toml. |

## External dependencies

Crates.io and other external dependencies are tracked separately with `cargo-about`.

Generate the release notice bundle with:

```powershell
script\generate-licenses.ps1 assets\licenses.md
```

Include the generated file with any binary release.

## Trademark and affiliation notice

This project is derived from Zed code but is not affiliated with or endorsed by Zed Industries. Names, logos, and marks remain the property of their respective owners.
