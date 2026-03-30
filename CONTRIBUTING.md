# Contributing to Markdown Viewer

Thanks for helping improve this repository.

## Scope

This is a Windows x64, read-only Markdown viewer derived from the Zed codebase. Please keep changes focused on the viewer itself, its packaging, the workspace crates required to build it, and the public documentation that explains its origin and licensing.

## Before you open a pull request

- Prefer small, focused changes.
- Open an issue or discussion first for large features or architectural changes.
- Include tests when code behavior changes.
- Include screenshots for visible UI changes.
- Avoid mass refactors unrelated to the main change.

## Licensing of contributions

By submitting a contribution, you confirm that you have the right to contribute it under the license terms used in this repository.

- Changes to files derived from GPL-licensed Zed crates must remain GPL-compatible and are distributed here under `GPL-3.0-or-later`.
- New standalone files that do not copy GPL-derived material may be contributed under `Apache-2.0` so newly authored code can stay as permissive as possible.
- Keep existing copyright notices, license headers, and adjacent `LICENSE-*` files intact.
- When adapting upstream files, add a clear modification notice consistent with the applicable license requirements.

See `README.md` and `THIRD_PARTY_LICENSES.md` for the current project-wide and crate-level licensing model.

## Local development

The default Cargo member is `crates\markdown_viewer`.

```powershell
cargo run -p markdown_viewer -- path\to\file.md
cargo build -p markdown_viewer --release
script\generate-licenses.ps1 assets\licenses.md
```

Generate the dependency notice bundle before publishing a release.
