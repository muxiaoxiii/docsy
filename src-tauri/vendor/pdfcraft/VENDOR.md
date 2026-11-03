# PdfCraft Vendored Dependency

This directory contains vendored crates from `storytold/pdfcraft`.

- **Upstream Repository**: https://github.com/storytold/pdfcraft
- **Upstream Version**: `v0.18.0`
- **Upstream Commit**: `3936f93a1479366a951206fc6d3094680fe2d3dc`
- **Upstream License**: Dual licensed under `MIT` OR `Apache-2.0` (see `LICENSE-MIT` and `LICENSE-APACHE`)
- **Upstream Notice**: see `NOTICE`

## Vendored Crates

- `crates/cos`: PDF COS-level object parsing, document manipulation, serialization, and stream decoding.
- `crates/crypt`: Standard PDF security handler implementation (RC4 40/128-bit, AES-128, AES-256 R5/R6).
- `crates/filters`: PDF stream filters (Flate, LZW, predictors, ASCII85, ASCIIHex, RunLength, CCITT Group 3/4, JBIG2).
- `crates/content`: Content stream decoding and operator parsing.
- `crates/organize`: Document combining, page extraction/importing, bookmarks, and structural manipulation.
- `vendor/hayro-syntax`: Low-level PDF syntax reader vendored from upstream.
- `vendor/hayro-jbig2`: Pure-Rust memory-safe JBIG2 decoder vendored from upstream.

## Local Patches Applied for Docsy

1. **Public Page-Tree Walk (`crates/organize/src/lib.rs`)**:
   - Exposed `pub fn walk(doc: &Document)` for Docsy's page geometry extraction and layout traversal.

2. **Form Appearance State Preservation in Combiner (`crates/organize/src/import/fields.rs`)**:
   - Updated `register_fields` to preserve source document `/AcroForm << ... /NeedAppearances true >>` flag during page import and document combining, preventing silent baking of stale form appearances.

3. **Resource Protection During Unused XObject Pruning (`crates/organize/src/prune.rs`)**:
   - Added `protect_indirect_resources` to protect `Pattern`, `ExtGState`, `Shading`, and `Annots` from premature pruning, and handled empty/absent `/Contents`.

4. **CCITT Encoder Line Width Conversion (`crates/filters/src/ccitt.rs`)**:
   - Cast line width to `u32` for `encoder.encode_line`.

5. **Standalone Crate Manifests and Minimal Distribution**:
   - Replaced upstream workspace inheritance with explicit crate versions and path dependencies.
   - Kept the minimal Docsy README and prior NOTICE attributions; excluded unrelated applications, assets and other vendored crates.
   - Added `crates/organize/examples/docsy_probe.rs` for local backend verification.

*(Note: Upstream v0.18.0 has officially incorporated the previous patches for tolerant stream delimiter spacing and strict ISO 32000-1 `/UserUnit` non-inheritance).*
