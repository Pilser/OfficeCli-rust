# RFC: set-by-bookmark & batch MutationReport

**Status**: Draft  
**Author**: Internal  
**Target**: OfficeCli-rust v0.2.x

---

## 1. Problem Statement

When a caller performs mutations (set, add, remove, move) on an Office document, DOM
node indices shift. For example, splitting `/body/p[3]` into two paragraphs turns it
into `/body/p[3]` + `/body/p[4]`, pushing every subsequent `p[N]` index up by one.

An external IR (Intermediate Representation) that tracks elements by
`office_cli_path` becomes **stale** after every mutation. There is currently no
mechanism to map old paths to new paths, so the IR cannot efficiently locate
elements it previously referenced.

Two specific pain points:

1. **Path-based addressing is fragile** — bookmarks (`w:bookmarkStart`) are stable
   semantic anchors, but the CLI only supports path-based `set`. A caller must
   first `view --mode forms` or `query bookmarkStart` to discover the path, then
   call `set /body/p[N]/bookmarkStart[M] …` — but the path may shift between the
   query and the set.

2. **Batch has no path-mutation feedback** — `batch` returns `OK`/`ERROR` per
   command but does not report which paths were created, moved, or invalidated.
   The caller's IR has no way to update itself.

---

## 2. Proposal

### 2.1 `set-by-bookmark` command

A new top-level command (or a `set` sub-mode) that addresses elements by bookmark
name instead of DOM path.

```
officecli set doc.docx --bookmark DSN_S_123 --text "张**"
officecli set doc.docx --bookmark DSN_S_123 --replace "old text=new text"
```

**Semantics:**

1. Resolve `bookmark name → bookmarkStart node` by scanning the document body for
   `w:bookmarkStart[@w:name=<name>]`.
2. Find the **content range** between `bookmarkStart` and the matching
   `bookmarkEnd` (by `w:id` pairing).
3. Apply the mutation:
   - `--text <value>`: replace all text content within the bookmark range
   - `--replace <old=new>`: substring replacement within the bookmark range
4. Return a `MutationReport` (see §2.2).

**Why a separate command instead of overloading `set`:**

- `set` currently requires a `path` positional arg. Adding `--bookmark` as an
  alternative path resolver changes the semantics enough to warrant its own
  command surface.
- `set-by-bookmark` always operates on the **bookmark content range**, not the
  `bookmarkStart` element itself (which `set /body/p[1]/bookmarkStart[1]` would
  do). The distinction is important: the user wants to change the *text inside
  the bookmark*, not the bookmark's XML attributes.

**Handler implementation outline (docx-handler):**

```rust
pub fn set_by_bookmark(
    dom: &mut WordDom,
    bookmark_name: &str,
    properties: &HashMap<String, String>,
) -> Result<MutationReport, HandlerError> {
    // 1. Find bookmarkStart by name
    let (bm_start_path, bm_id) = find_bookmark_start_by_name(dom, bookmark_name)?;

    // 2. Find bookmarkEnd by matching id
    let bm_end_path = find_bookmark_end_by_id(dom, &bm_id)?;

    // 3. Collect old paths of elements in the range
    let old_paths = collect_content_paths(dom, &bm_start_path, &bm_end_path);

    // 4. Apply mutation (text replacement, etc.)
    apply_bookmark_mutation(dom, &bm_start_path, &bm_end_path, properties)?;

    // 5. Collect new paths (indices may have shifted)
    let new_paths = collect_content_paths(dom, &bm_start_path, &bm_end_path);

    Ok(MutationReport {
        bookmark_name: bookmark_name.to_string(),
        old_paths,
        new_paths,
        matched: true,
    })
}
```

### 2.2 `MutationReport` in batch responses

Extend the `batch` command's per-operation result to include path mutation info.

**Current `BatchResult`:**

```json
{
  "op": "set",
  "result": "OK"
}
```

**Proposed `BatchResult`:**

```json
{
  "op": "set-by-bookmark",
  "result": {
    "status": "ok",
    "bookmark_name": "DSN_S_123",
    "old_paths": ["/body/p[3]/r[1]/t[1]"],
    "new_paths": ["/body/p[3]/r[1]/t[1]"],
    "matched": true
  }
}
```

For regular path-based mutations:

```json
{
  "op": "add",
  "result": {
    "status": "ok",
    "old_paths": [],
    "new_paths": ["/body/p[4]"],
    "matched": true
  }
}
```

```json
{
  "op": "remove",
  "result": {
    "status": "ok",
    "old_paths": ["/body/p[3]"],
    "new_paths": [],
    "matched": true
  }
}
```

```json
{
  "op": "set",
  "result": {
    "status": "ok",
    "old_paths": ["/body/p[3]"],
    "new_paths": ["/body/p[3]", "/body/p[4]"],
    "matched": true
  }
}
```

**`MutationReport` schema:**

| Field | Type | Description |
|---|---|---|
| `status` | `"ok"` \| `"error"` | Operation result |
| `error` | `string?` | Error message if `status == "error"` |
| `bookmark_name` | `string?` | Bookmark name, if the operation was `set-by-bookmark` |
| `old_paths` | `string[]` | Paths of elements before mutation |
| `new_paths` | `string[]` | Paths of elements after mutation |
| `matched` | `bool` | Whether the target element was found |

**IR update algorithm (caller side):**

```
for each MutationReport in batch response:
  if report.matched:
    for (old, new) in zip(report.old_paths, report.new_paths):
      IR.update_path(old, new)

    // Elements after the mutation point shift
    last_old = max_index(report.old_paths)
    last_new = max_index(report.new_paths)
    delta = len(report.new_paths) - len(report.old_paths)
    if delta != 0:
      IR.shift_siblings_after(last_old, delta)
```

---

## 3. Batch JSON Schema Extension

New `set-by-bookmark` verb in batch:

```json
[
  {
    "command": "set-by-bookmark",
    "bookmark": "DSN_S_123",
    "text": "张**"
  },
  {
    "command": "set-by-bookmark",
    "bookmark": "DSN_S_456",
    "replace": "old value=new value"
  },
  {
    "command": "set",
    "path": "/body/p[1]",
    "properties": { "text": "Title" }
  }
]
```

Each entry in the batch response includes a `MutationReport`.

---

## 4. CLI Surface

```
officecli set-by-bookmark <file> --bookmark <name> --text <value>
officecli set-by-bookmark <file> --bookmark <name> --replace <old=new>
```

With `--json`:

```json
{
  "bookmark_name": "DSN_S_123",
  "old_paths": ["/body/p[3]/r[1]/t[1]"],
  "new_paths": ["/body/p[3]/r[1]/t[1]"],
  "matched": true
}
```

---

## 5. Affected Crates

| Crate | Change |
|---|---|
| `handler-common` | Add `MutationReport` struct; add `set_by_bookmark()` to `DocumentHandler` trait |
| `docx-handler` | Implement `set_by_bookmark()` — find bookmark range, apply mutation, compute old/new paths |
| `xlsx-handler` | Stub — return `UnsupportedMode` (Excel uses named ranges, not bookmarks) |
| `pptx-handler` | Stub — return `UnsupportedMode` |
| `pdf-handler` | Stub — return `UnsupportedMode` |
| `officecli` | New `SetByBookmarkCommand` + handler; extend `BatchResult` to include `MutationReport` |

---

## 6. Path Mutation Tracking — Design Decisions

### Q: Should `set` on a regular path also return `MutationReport`?

**Yes** — this is the key value proposition. Any mutation can cause path shifts:
- `set /body/p[3] text="line1\nline2"` may split a paragraph
- `remove /body/p[3]` shifts all subsequent indices
- `add /body paragraph` inserts a new element
- `move /body/p[3] --target /body` re-indexes both source and target

All mutation commands should return `MutationReport` in `--json` mode.

### Q: What about non-mutation commands (get, view, query)?

No. Read-only commands don't mutate the DOM, so no path shifts occur.

### Q: What about `raw-set`?

`raw-set` operates at the XML level and the handler cannot reliably track path
mutations. Return `MutationReport` with empty `old_paths`/`new_paths` and a
warning that path tracking is unavailable for raw operations.

### Q: How to handle cascading shifts?

When `add` inserts `/body/p[4]`, every `/body/p[N]` where N ≥ 4 shifts to N+1.
The `MutationReport` should include a `shift_hint` for efficient IR updates:

```json
{
  "status": "ok",
  "old_paths": [],
  "new_paths": ["/body/p[4]"],
  "shift_hint": {
    "base_path": "/body/p",
    "from_index": 4,
    "delta": 1
  },
  "matched": true
}
```

This allows the IR to do a single bulk shift instead of per-element updates.

---

## 7. Example: End-to-End Desensitization Flow

```bash
# 1. Discover bookmarks
officecli view report.docx --mode forms --json

# 2. Batch-desensitize by bookmark
officecli batch report.docx '[
  {"command":"set-by-bookmark","bookmark":"DSN_S_123","text":"张**"},
  {"command":"set-by-bookmark","bookmark":"DSN_S_456","text":"李**"},
  {"command":"set-by-bookmark","bookmark":"DSN_S_789","replace":"13800138000=138****8000"}
]' --json

# 3. Use MutationReport to update IR
# Each result tells the IR exactly which paths changed:
# {
#   "results": [
#     {
#       "op": "set-by-bookmark",
#       "result": {
#         "status": "ok",
#         "bookmark_name": "DSN_S_123",
#         "old_paths": ["/body/p[3]/r[1]/t[1]"],
#         "new_paths": ["/body/p[3]/r[1]/t[1]"],
#         "matched": true
#       }
#     },
#     ...
#   ]
# }

# 4. Save
officecli save report.docx
```

---

## 8. Implementation Priority

| Phase | Scope | Effort |
|---|---|---|
| **P1** | `MutationReport` struct in `handler-common` + `set_by_bookmark()` in `docx-handler` | 2-3 days |
| **P2** | `set-by-bookmark` CLI command + `--json` output | 1 day |
| **P3** | Extend `batch` to return `MutationReport` per operation | 1 day |
| **P4** | Add `MutationReport` to all mutation commands (`set`, `add`, `remove`, `move`) | 2-3 days |
| **P5** | `shift_hint` for efficient IR bulk updates | 1 day |

---

## 9. Open Questions

1. **Named ranges in xlsx** — Excel uses defined names (`_xlnm.Print_Area`, custom names)
   rather than bookmarks. Should `set-by-name` be a separate command, or should
   `set-by-bookmark` be renamed to `set-by-name` with format-specific resolution?

2. **Cross-run bookmarking** — After a `set-by-bookmark` that splits a paragraph,
   should OfficeCLI auto-repair the bookmark range to cover both new paragraphs?
   Or should the bookmark stay on the first paragraph only?

3. **Transaction semantics** — Should `batch` support rollback-on-failure? Currently
   operations are applied sequentially with no rollback. For desensitization,
   partial application may be worse than no application.

4. **Streaming MutationReport** — For large batches, should `batch --json` stream
   results (one JSON object per line) instead of returning a single JSON array?
   This would align with the plugin protocol's JSONL pattern.
