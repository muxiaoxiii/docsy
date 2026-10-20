# Vendored PdfCraft (Minimal Candidate Backend)

- Source: [storytold/pdfcraft](https://github.com/storytold/pdfcraft)
- Upstream Commit: `4db6ac4ae2448df6389d8bb0bc54db6ba175721c`
- License: MIT OR Apache-2.0 (see `LICENSE-MIT`, `LICENSE-APACHE`, and `NOTICE`)
- Selected Crates:
  - `cos` (0.9.0): Core PDF object model with tolerant parsing and copy-on-write edits.
  - `crypt` (0.9.0): PDF standard security handler (RC4 / AES).
  - `filters` (0.9.0): Stream filters (Flate, ASCII85, LZW, etc.).
  - `content` (0.9.0): Content stream operations.
  - `organize` (0.9.0): Page combinations, extraction, boxes, and document structure.
- Intentionally excluded:
  - `render` / `ui-egui` (no UI bloat)
  - `ocr` (no OCR engine bloat)
  - `js` / `xfa` / `sign` (no JS or signature engines)
  - `engine` / `apps` (standalone CLI/web applications)
