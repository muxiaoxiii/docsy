# Fix: Add 4 missing exports to fieldRowUtils.js

`TemplateBuildTab.vue` imports these from `fieldRowUtils.js` but they don't exist there. This breaks `npm run build`.

## What to add to `src/modules/template/composables/fieldRowUtils.js`

Add these 4 exported functions at the end of the file, BEFORE the last closing brace if any. They must accept `fieldRows` as a parameter (not use a global ref).

### 1. `referenceSuggestion(row, fieldRows)`

Port from TemplateView.vue lines 539-555. Change `fieldRows.value` → `fieldRows` parameter.

```js
export function referenceSuggestion(row, fieldRows) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  const text = normalizeComparableText(row.text)
  if (!text) return null
  const rowIndex = fieldRows.indexOf(row)
  if (rowIndex <= 0) return null
  const target = findReferenceTarget(row, text, rowIndex, fieldRows)
  if (!target) return null
  return {
    target: target.row,
    targetLabel: target.label,
    targetKind: target.kind,
    sourceIndex: target.sourceIndex,
    prefixRows: adjacentStructureRows(row, 'prefix', fieldRows),
    suffixRows: adjacentStructureRows(row, 'suffix', fieldRows),
  }
}
```

Also port these PRIVATE helpers (not exported, used only by referenceSuggestion):
- `findReferenceTarget(row, text, rowIndex, fieldRows)` — from TemplateView.vue ~line 557
- `partySourceTotalsBefore(rowIndex, fieldRows)` — from ~line 587
- `partySourceIndexForRow(row, rowIndex, totalsBeforeRow, fieldRows)` — from ~line 598
- `adjacentStructureRows(row, usage, fieldRows)` — from ~line 619

All of these use `fieldRows.value` internally — change to `fieldRows` parameter.

### 2. `referenceSourceOptions(row, fieldRows)`

This function doesn't exist in TemplateView.vue either! Check TemplateBuildTab.vue line 719 — it calls `referenceSourceOptionsFn(row, props.fieldRows)`. 

Look at how it's used in the template (line 299): `v-for="item in referenceSourceOptions(row)"`. Each item should have `{ value, label }` for a dropdown.

Based on the context, this should return the list of possible reference source fields that `row` could reference. Implementation: iterate `fieldRows`, find all field-type rows that appear before `row`, return `{ value: referenceSourceKey(...), label: ... }` for each.

### 3. `allReferenceSuggestions(fieldRows)`

Port from TemplateView.vue ~line 714:
```js
export function allReferenceSuggestions(fieldRows) {
  return fieldRows.filter((row) => referenceSuggestion(row, fieldRows))
}
```

### 4. `hasUnseenReferenceSuggestion(row, fieldRows)`

This doesn't exist in TemplateView.vue. Based on the template usage (`v-if="hasUnseenReferenceSuggestion(row)"`), it should return true when a row has a reference suggestion but `row.referenceHintSeen` is not set:
```js
export function hasUnseenReferenceSuggestion(row, fieldRows) {
  const suggestion = referenceSuggestion(row, fieldRows)
  return suggestion && !row.referenceHintSeen
}
```

## After adding

Run:
1. `npm run build` — must succeed (this was the blocking error)
2. `npm test` — 75 tests must pass
3. `cargo test --manifest-path src-tauri/Cargo.toml` — 150 tests must pass

## Important
- Do NOT remove any existing exports from fieldRowUtils.js
- Keep the existing normalizeComparableText export (it's already there)
- The private helpers (findReferenceTarget etc.) should NOT be exported
- All functions must accept fieldRows as a plain Array parameter, not a Vue ref
