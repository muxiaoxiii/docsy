# PdfCraft Vendored Dependency

This directory contains vendored crates from `storytold/pdfcraft`.

- **Upstream Repository**: https://github.com/storytold/pdfcraft
- **Upstream Commit**: `4db6ac4ae2448df6389d8bb0bc54db6ba175721c`
- **Upstream License**: Dual licensed under `MIT` OR `Apache-2.0` (see `LICENSE-MIT` and `LICENSE-APACHE`)
- **Upstream Notice**: see `NOTICE`

## Vendored Crates

- `crates/cos`: PDF COS-level object parsing, document manipulation, serialization, and stream decoding.
- `crates/crypt`: Standard PDF security handler implementation (RC4 40/128-bit, AES-128, AES-256 R5/R6).
- `crates/filters`: PDF stream filters (FlateDecode, ASCIIHexDecode, ASCII85Decode, RunLengthDecode).
- `crates/content`: Content stream decoding and operator parsing.
- `crates/organize`: Document combining, page extraction/importing, bookmarks, and structural manipulation.

## Local Patches Applied for Docsy

1. **Lenient Cross-Reference & Trailer Recovery (`crates/cos/src/parser.rs`)**:
   - Enhanced xref and trailer recovery for real-world PDFs with slightly miscalculated byte offsets or trailing garbage before EOF.

2. **Strict ISO 32000-1 Page Attribute Inheritance (`crates/organize/src/lib.rs`)**:
   - Removed `b"UserUnit"` from `INHERITABLE` table.
   - Per ISO 32000-1 Table 30, `/UserUnit` is strictly a leaf `/Page` attribute and must not be inherited from parent `/Pages` tree nodes.

3. **Form Appearance State Preservation in Combiner (`crates/organize/src/import.rs`)**:
   - Updated `register_fields` to preserve source document `/AcroForm << ... /NeedAppearances true >>` flag during page import and document combining, preventing silent baking of stale form appearances.
