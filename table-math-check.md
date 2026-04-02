# Table Math Check

## Table

| Feature | Example | Expected |
| :-- | :-- | --: |
| Inline code | `cargo build -p markdown_viewer` | right-aligned cell |
| Link | [README](README.md) | local file link |
| Math | $E = mc^2$ | inline math in table |
| Root | $\sqrt{\pi}$ | no raw delimiters |
| Escaped pipe | a \| b | literal pipe |

## Mixed Content

| Section | Snippet |
| :-- | :-- |
| Mermaid | See the two fenced diagrams above. |
| Rust | See the fenced Rust sample above. |
| Math | $\sqrt{2}$, $\hat{f}$, $\times$, $\int_a^b$, $\sum_{k=1}^{n}$ |
