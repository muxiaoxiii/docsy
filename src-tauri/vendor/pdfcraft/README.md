# Vendored PdfCraft (Minimal Candidate Backend)

- Source: [storytold/pdfcraft](https://github.com/storytold/pdfcraft)
- Upstream Version: v0.18.0
- Upstream Commit: `3936f93a1479366a951206fc6d3094680fe2d3dc`
- License: MIT OR Apache-2.0 (see `LICENSE-MIT`, `LICENSE-APACHE`, and `NOTICE`)
- Selected Crates:
  - `cos` (0.18.0): Core PDF object model with tolerant parsing and copy-on-write edits.
  - `crypt` (0.18.0): PDF standard security handler (RC4 / AES).
  - `filters` (0.18.0): Stream filters, CCITT and JBIG2 decoding.
  - `content` (0.18.0): Content stream operations.
  - `organize` (0.18.0): Page combinations, extraction, boxes, and document structure.
  - `hayro-syntax` (0.7.2) and `hayro-jbig2` (0.3): Upstream vendored syntax and image decoders.
- Intentionally excluded:
  - `render` / `ui-egui` (no UI bloat)
  - `ocr` (no OCR engine bloat)
  - `js` / `xfa` / `sign` (no JS or signature engines)
  - `engine` / `apps` (standalone CLI/web applications)
