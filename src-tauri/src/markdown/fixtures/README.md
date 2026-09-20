# Markdown render fixtures

Captured on 2026-09-20 from the real local Chromium / MathJax / Mermaid renderer at source baseline `8682623`.

- `rendered.json`: four editable equations and one Mermaid flowchart; source is in `real_browser_media_export`.
- `multilingual.md` and `multilingual.json`: French, Japanese, Korean, combining characters, emoji, native math and Mermaid.

Default Rust tests deserialize the actual frontend camelCase payload, validate PNG data, and export DOCX / HTML / XLSX / PPTX in private temporary directories. They assert native OMML, embedded images and preserved text. These are export regression fixtures, not a live browser test or a claim of installed-font parity across operating systems.
