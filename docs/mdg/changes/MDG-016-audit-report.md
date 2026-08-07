# MDG-016 Deep Code Audit Report

## 1. Bug Analysis — Confirm / Correct

### Bug 1: File list vs rendered data inconsistency — **CONFIRMED**

**Root cause**: Two separate code paths with different data sources.

| Path | Function | Data source |
|------|----------|-------------|
| Rendering (overlay PDF) | `buildHeaderTextForGroup(file, index, group, rules)` | `group.mode` |
| File list display | `buildHeaderText(file, index, rules)` | `rules.headerMode` |

- **Rendering**: `buildHeaderFooterItems` (useEvidencePdfSession.js:507) calls `buildHeaderTextForGroup` with the resolved group's mode.
- **Display**: `displayRowHeader` → `rowHeaderPreview` → `buildHeaderText` (useEvidencePdfExistingEditing.js:213) reads `rules.headerMode`.
- When `rules.headerMode` and `headerGroup.mode` diverge (e.g., after switching modes without updating both), the display shows stale data.

**Fix location**: `useEvidencePdfExistingEditing.js:213` — `rowHeaderPreview` should use `buildHeaderTextForGroup` with the file's selected group instead of `buildHeaderText`.

---

### Bug 2: "Per evidence list" mode shows filename — **CONFIRMED**

**Root cause**: Initialization race condition in `startHeaderEdit`.

**Lifecycle of `file.header`:**
1. `createEvidenceFile` (useEvidencePdfSession.js:130): `header: stripPdf(name)` ← initialized to filename
2. `startHeaderEdit` (useEvidencePdfExistingEditing.js:34): `row.header = rowHeaderPreview(row, index) || stripPdf(row.name)` ← if mode isn't `per_file` yet, `rowHeaderPreview` returns filename-based text
3. `headerBaseTextForGroup` (useEvidencePdfSession.js:356): `per_file` → `file.header ?? stripPdf(file.name)` ← reads the filename-initialized value

**Sequence**: Mode switch to `per_file` happens at `startHeaderEdit:33` (`headerMode.value = 'per_file'`), but `file.header` is initialized at line 34 BEFORE the group mode is updated. The `rowHeaderPreview` call at line 34 uses the OLD mode.

**Fix**: In `headerBaseTextForGroup`, when `group.mode === 'per_file'`, DON'T fall back to `file.header`. Instead, generate `证据${index + 1}` directly, bypassing `file.header` entirely.

---

### Bug 3: Sequence always starts from 1 — **CONFIRMED**

**Location**: `splitFileName.js:47`
```js
const value = String(Math.max(1, Number(index || 0) + 1))
```

`index` is the file's array position (0-based), always starting from 0. No way to set a custom start.

**All callers of `formatSequenceToken`**: Only `expandSplitNameTokens` at `splitFileName.js:36`.

**All callers of `expandSplitNameTokens`**: `formatSplitFileName`, `decorateHeaderTextForGroup`, `decorateHeaderText`, `resolveTextTemplate` — all pass the array index directly.

**Fix**: Add a `startIndex` parameter (default 1) to `formatSequenceToken` and propagate it from the rules config.

---

### Bug 4: Deletion failure — **CONFIRMED with nuanced root cause**

See Section 2 for complete deep audit.

---

## 2. Bug 4 Deep Audit — Complete Failure Path Analysis

### 2.1 Data Flow Trace: Detection → Deletion

```
DETECTION (detection.rs: detect())
  ├── pdftotext -bbox → renders PDF, extracts text + bbox (render-level, sees ALL text)
  ├── Artifact inspection → content-stream-level, finds BDC Artifact markers
  └── Returns: candidates with {text, normalizedText, bbox, source:'artifact'|'content-text'}

FRONTEND (useEvidencePdfDetection.js: applyDetectionResultToFile)
  ├── Stores candidates as file.existingElements
  └── Each element has: {kind, detectedText, normalizedText, bbox, source, decision}

USER ACTION: marks element.decision = 'delete'

JOB CONSTRUCTION (useEvidencePdfSession.js: buildHeaderFooterItems)
  ├── hasArtifactDecision() → checks element.source === 'artifact'
  │   └── If false → headerEnabled = false → artifact path skipped
  ├── buildPlainTextTargets() → for content-text 'delete' elements
  │   └── Returns: [{text, normalizedText, pageStart, pageEnd, bbox}]
  └── Result: cleanup.plainHeaderTargets = [...], cleanup.headerEnabled = false

BACKEND PROCESSING (header_footer.rs: process_job)
  ├── Step 1: edit_or_delete_standard_artifacts_if_requested()
  │   └── SKIPPED because headerEnabled = false (no artifact elements marked)
  ├── Step 2: delete_confirmed_plain_text_header_footer_if_requested()
  │   └── Calls content_text::delete_plain_header_footer_to_temp()
  │       ├── For each page:
  │       │   ├── filter_page_operations() — top-level content stream
  │       │   └── filter_referenced_form_text() — Form XObjects
  │       └── Returns removed count
  ├── Step 3: semantic_removed = artifact_removed + plain_removed
  └── If semantic_removed == 0 → WARNING: "没有找到可安全删除的匹配内容"
```

### 2.2 `filter_page_operations` Deep Audit (content_text.rs:287-319)

**Does it recurse into Form XObjects?** 

**NO — `filter_page_operations` itself does NOT recurse.** But the CALLER `delete_plain_header_footer_file` (line 109-123) explicitly handles Form XObjects separately:

```rust
// Line 109: filter_page_operations on top-level content
let (operations, page_result) = filter_page_operations(&content.operations, &page_plan);

// Line 112-123: separate Form XObject processing
let xobjects = super::artifacts::page_xobjects(&doc, page_id);
if !xobjects.is_empty() {
    let nested_result = filter_referenced_form_text(
        &mut doc, &content.operations, &xobjects, &page_plan, &mut HashSet::new(),
    )?;
}
```

`filter_referenced_form_text` (line 144-232):
1. Finds all `Do` operations in the content stream
2. Looks up each XObject name in the page's XObject dictionary
3. Checks it's a Form subtype
4. Decodes the form's content stream
5. Calls `filter_page_operations` on the form's operations
6. **Recursively** processes nested XObjects within the form
7. Writes back modified content if changes were made

**Conclusion**: The brief's claim that "Plain text 路径不递归进入 Form XObject" is **INCORRECT**. The code DOES recurse into Form XObjects. There's even a unit test confirming this (`deletes_confirmed_plain_header_inside_form_xobject`, line 760).

### 2.3 `edit_referenced_form_artifacts` Deep Audit (artifacts.rs:823-892)

**What exactly does it search for in Form XObjects?**

It searches for **Artifact marked-content ranges** (BDC/EMC) with Subtype Header or Footer:

```rust
fn target_artifact_region(operation, targets, properties) → Option<ArtifactRegion> {
    // Must be BDC operator
    // First operand must be Name "Artifact"
    // Second operand must resolve to a dict with Subtype "Header" or "Footer"
}
```

For each `Do` operation:
1. Resolves XObject reference
2. Checks Subtype === Form
3. Decodes content stream
4. Calls `edit_target_artifact_ranges` which looks for BDC Artifact/Header or BDC Artifact/Footer
5. If found, either replaces the text (if replacement configured) or removes the range
6. Recursively processes nested XObjects

**Key difference from plain text path**: Artifact path only looks for BDC markers. It doesn't look for bare text by position or content.

### 2.4 ALL Possible Failure Paths for Deletion

#### Path A: Zone check failure (MOST LIKELY for Bug 4)
- **Location**: `content_text.rs:300-305`, `is_in_header_zone` / `is_in_footer_zone`
- **Cause**: `state.y` (tracked from Tm/Td/TD operators) is outside the expected zone
- **When**: Text in Form XObject with non-page-aligned BBox, or text slightly outside the zone
- **Zone definition**: `y >= max_y - zone_pt && y <= max_y + 24.0` (header)
- **Default zone**: 18mm ≈ 51pt from page edge
- **Critical**: Zone check is a GATE — if it fails, `matches_any_target` is never called

#### Path B: Text encoding mismatch
- **Location**: `content_text.rs:377-399`, `shown_text`; line 401-413, `object_text`
- **Cause**: CID-encoded text (glyph indices) decoded as `String::from_utf8_lossy` produces garbled bytes
- **When**: PDF uses CID fonts (common with Chinese text), text stored as GIDs not Unicode
- **Effect**: `target_matches` fails because decoded text ≠ pdftotext's decoded text
- **BBox fallback**: `target_bbox_matches` would still work IF zone check passes

#### Path C: BBox coordinate mismatch
- **Location**: `content_text.rs:449-467`, `target_bbox_matches`
- **Cause**: pdftotext bbox (render-level) doesn't align with content stream Tm coordinates
- **Conversion**: `pdf_y0 = height - bbox.y1; pdf_y1 = height - bbox.y0` (top-down → bottom-up)
- **When**: Page has CropBox with non-zero origin, or form has transformation matrix
- **Tolerance**: 18pt padding on each axis

#### Path D: Form XObject coordinate space mismatch
- **Location**: `content_text.rs:197`, `filter_page_operations(&content.operations, plan)`
- **Cause**: `plan.page_box` uses page MediaBox/CropBox, but form content uses form-local coordinates
- **When**: Form BBox ≠ page MediaBox (e.g., form BBox is [0, 0, 595, 80] for header area)
- **Effect**: Zone check fails because form y values don't match page zone

#### Path E: No targets constructed
- **Location**: `useEvidencePdfSession.js:664-685`, `buildPlainTextTargets`
- **Cause**: Element's `decision` is not 'delete' or 'edit', or element has no text
- **When**: Frontend state not properly updated before job construction

#### Path F: Page box unavailable
- **Location**: `content_text.rs:96-98`
- **Cause**: `page_box(&doc, page_id)` returns None (no CropBox/MediaBox found)
- **Effect**: Entire page skipped silently

#### Path G: Content stream decode failure
- **Location**: `content_text.rs:92-95`, `Content::decode` at line 194
- **Cause**: Corrupt or unsupported content stream encoding
- **Effect**: Page or form skipped silently

### 2.5 Can BBox-Based Deletion Work Reliability?

**Partially yes, but with caveats:**

1. **Current implementation has a critical flaw**: The zone check (`is_in_header_zone`) is applied BEFORE the bbox check. If the text's Tm y-coordinate doesn't fall within the zone, the bbox is never consulted.

2. **When bbox matching works well**:
   - Text is in the page content stream (not in a Form XObject)
   - Page coordinates match pdftotext coordinates (standard case)
   - Zone check passes (text is within the header/footer zone)

3. **When bbox matching fails**:
   - Text in Form XObject with different coordinate space
   - Non-standard page CropBox origin
   - Zone is set too narrowly

4. **Recommended fix**: When a bbox is available, bypass the zone check entirely. The bbox already encodes the position, so the zone check is redundant and harmful.

---

## 3. Code Architecture Assessment

### Strengths
1. **Dual-path deletion**: Both Artifact and Plain Text paths are well-structured
2. **Form XObject recursion**: Both paths properly recurse into nested Form XObjects
3. **Cycle detection**: Uses `visited` sets to prevent infinite recursion
4. **BBox fallback**: `target_bbox_matches` provides a fallback when text can't be decoded
5. **Comprehensive detection**: pdftotext + Artifact inspection covers most cases

### Weaknesses
1. **Zone check as gate**: The zone check blocks bbox-based matching, which is the primary fallback for CID-encoded text
2. **No CTM tracking for forms**: `filter_page_operations` doesn't account for Form XObject coordinate transformations
3. **Silent failures**: Many failure paths (decode errors, missing page box) just `continue` without logging
4. **Coordinate system inconsistency**: Detection uses pdftotext (render-level, top-down), deletion uses content stream (operator-level, bottom-up). The y-axis conversion in `target_bbox_matches` is correct but fragile.

---

## 4. Requirement Feasibility Evaluation

### Req 1: Detection-Deletion Unification
**Feasibility**: Medium-High
- **BBox-based deletion can work** if the zone check is removed as a gate when bbox is available
- **Recommended approach**: In `filter_page_operations`, check bbox match FIRST (before zone check) when `target.bbox.is_some()`. If bbox matches, skip zone check.
- **Alternative**: Add Form XObject CTM tracking to properly transform coordinates

### Req 2: "Per evidence list" UI Refactoring
**Feasibility**: High
- Remove dependency on `file.header` in `headerBaseTextForGroup`
- Add dedicated settings: prefix dropdown, sequence format, start number, step
- All backend support exists (`format_page_number`, `number_style`, `number_offset`)

### Req 3: Placeholder Enhancement (`[#, start]`, `[#，start，step]`)
**Feasibility**: High
- Modify `expandSplitNameTokens` in `splitFileName.js:34-43`
- Parse `[#start]` or `[#start,step]` format
- Pass start/step to `formatSequenceToken`

### Req 4: Sequence Format Options
**Feasibility**: High
- `pdfPageNumberRules.js` already defines `PAGE_NUMBER_STYLES`
- Backend `format_page_number` supports: arabic, chinese, roman-upper, roman-lower, circled, dingbat
- Frontend needs UI to select style for header/footer sequences

### Req 5: Odd/Even Page Support
**Feasibility**: Medium
- Backend `expand_config_placeholders` already handles per-page expansion
- Need to add `first_page_different` and `odd_even_different` to `OverlayTextConfig`
- `build_overlay_pdf` would need to select different configs based on page parity
- Frontend needs new fields in group config

### Req 6: "分段" → "显示范围" Label Change
**Feasibility**: Trivial (UI-only)

### Req 7: Large File Detection Optimization
**Feasibility**: High
- `MAX_SPLIT_ANALYSIS_PAGES` already limits to 600 pages
- Add user-facing threshold check before calling `detect()`
- Show confirmation dialog for files exceeding threshold

### Req 8: Merge Completion Notification
**Feasibility**: Trivial
- Replace `ElMessage.success` with `ElNotification` in the merge completion handler

---

## 5. Recommended Fixes

### Fix 1: Bug 4 — Remove zone gate for bbox-based matching (CRITICAL)
**File**: `src-tauri/src/pdf/content_text.rs`
**Location**: `filter_page_operations` (line 295-315)

**Change**: When a target has a bbox, bypass the zone check:
```rust
if let Some(text) = shown_text.as_deref() {
    // If ANY target has a bbox, allow bbox-only matching without zone gate
    let has_bbox_targets = plan.header_targets.iter().any(|t| t.bbox.is_some());
    if (has_bbox_targets && matches_any_target(text, &plan.header_targets, &state))
        || (!has_bbox_targets && is_in_header_zone(state.y, plan)
            && matches_any_target(text, &plan.header_targets, &state))
    {
        remove_region = Some(TextRegion::Header);
    }
    // Similar for footer...
}
```

Or more precisely, modify `matches_any_target` to check bbox match without zone gate:
```rust
fn matches_any_target(text: &str, targets: &[&PlainTextTarget], state: &TextState, in_zone: bool) -> bool {
    targets.iter().any(|target| {
        if target_matches(text, target) && in_zone { return true; }
        if target.bbox.is_some() && target_bbox_matches(state, target) { return true; }
        false
    })
}
```

### Fix 2: Bug 1 — Unify display and render code paths
**File**: `src/modules/pdf-tools/composables/useEvidencePdfExistingEditing.js`
**Location**: `rowHeaderPreview` (line 213)

**Change**: Use `buildHeaderTextForGroup` with the file's selected group:
```js
function rowHeaderPreview(row, index) {
    const group = selectedGroupFor(row, 'header')
    if (group) return buildHeaderTextForGroup(row, index, group, currentRules.value)
    return buildHeaderText(row, index, currentRules.value)
}
```

### Fix 3: Bug 2 — Generate evidence label directly in per_file mode
**File**: `src/modules/pdf-tools/composables/useEvidencePdfSession.js`
**Location**: `headerBaseTextForGroup` (line 355-361)

**Change**: For `per_file` mode, use index-based label instead of `file.header`:
```js
function headerBaseTextForGroup(file, index, group, _rules) {
    if (group.mode === 'per_file') return file.header ?? `证据${index + 1}`
    // ... rest unchanged
}
```

### Fix 4: Bug 3 — Add configurable start index
**File**: `src/modules/pdf-tools/composables/splitFileName.js`
**Location**: `formatSequenceToken` (line 46-49)

**Change**:
```js
export function formatSequenceToken(token, index = 0, startIndex = 1) {
    const value = String(Math.max(1, startIndex + Number(index || 0)))
    return value.padStart(token.length, '0')
}
```

Propagate `startIndex` from rules config through `expandSplitNameTokens`.

### Fix 5: Improve CID text matching
**File**: `src-tauri/src/pdf/content_text.rs`
**Location**: `object_text` (line 401-413)

**Change**: Try `decode_text_string` (lopdf's PDF string decoder) before falling back to `from_utf8_lossy`:
```rust
fn object_text(object: &Object) -> Option<String> {
    let Object::String(bytes, _) = object else { return None; };
    if bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16 BE handling (existing)
    }
    // Try lopdf's text string decoder first
    super::artifacts::decode_pdf_string(object)
        .filter(|s| !s.trim().is_empty())
}
```

### Fix 6: Add diagnostic logging for deletion failures
**File**: `src-tauri/src/pdf/content_text.rs`
**Location**: `filter_page_operations` (line 295-315)

**Change**: Add `log::trace!` when zone check fails but text is non-empty:
```rust
if let Some(text) = shown_text.as_deref() {
    if !text.trim().is_empty() && !is_in_header_zone(state.y, plan) && !is_in_footer_zone(state.y, plan) {
        log::trace!("Text '{text}' at y={:.1} outside zones (header {:.1}-{:.1}, footer {:.1}-{:.1})",
            state.y, plan.page_box.max_y - plan.header_zone_pt, plan.page_box.max_y + 24.0,
            plan.page_box.min_y - 24.0, plan.page_box.min_y + plan.footer_zone_pt);
    }
}
```

---

## 6. Summary

| Bug | Status | Severity | Root Cause |
|-----|--------|----------|------------|
| Bug 1 | Confirmed | Medium | Two code paths with different data sources |
| Bug 2 | Confirmed | Medium | `file.header` initialization race |
| Bug 3 | Confirmed | Low | Hardcoded `Math.max(1, index + 1)` |
| Bug 4 | Confirmed | **High** | Zone check gates bbox matching; CID encoding breaks text matching |

**Critical finding**: The brief's claim that "plain text path doesn't recurse into Form XObjects" is **incorrect**. The code does recurse (`filter_referenced_form_text`). The actual root cause of Bug 4 is the **zone check acting as a gate before bbox matching**, combined with CID font encoding issues that prevent text matching.

**Priority fix**: Modify `filter_page_operations` to allow bbox-based matching without zone gate when bbox is available. This single change would fix the majority of Bug 4 cases.
