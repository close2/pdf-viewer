# 2192 — The C session wrappers are grouped by the command they serve

Session 1678. Status: accepted. Context: ADR 2005 and `doc/todo/74` entry 13.
No behavior changes; ADR 2193 is unused.

## The boundary

The census found one `Session` block of 1 291 lines and 97 methods. The three larger blocks
belong to the interpreter work active in this batch. `Session` wraps `viewer-core`'s vocabulary;
its methods already divide by the command or answer they carry and need no new abstraction.

`session.rs` keeps the state, the constructor, `reads` and the single command router `handle`.
It also keeps the free helpers, public frame description and existing tests. Nine child modules
hold the methods by concern:

| module | concern |
|---|---|
| `documents` | opening, closing, focus and deferred preparation |
| `navigation` | page movement, search, pointer, layout and layers |
| `frames` | completed rendering and frame readback |
| `printing` | the print operation and retained raster |
| `editing` | selection, widgets, annotations, attachments and saving |
| `policy` | restrictions, trust and rendering policy |
| `scripts` | clocks, script answers and keyboard decisions |
| `reports` | findings, viewer preferences and structured readback |
| `archiving` | the conversion command and retained result |

No state is duplicated. Children can read their parent's private fields and call `handle`, so
no visibility needs widening. Public methods retain their signatures. The root's raster helper
is named explicitly in the moved printing documentation. Five ledger code-path lists and the
selection teaching walk now point to the concerns that hold their operations.

## Evidence

The saved pre-split file and the final modules have identical token sequences for all 97
methods, including string and character contents. The state, free helpers and tests also have
identical token sequences. Only imports, module declarations, implementation wrappers and
module documentation differ. This is stronger than a multiset comparison of lines.

`cargo tree -i viewer-ffi` prints only this crate. The forward graphs of `render-raster`,
`render-cpu`, `pdf-render` and `pdf-sandbox` contain no `viewer-ffi`; this split cannot enter
those corpus binaries. Pixel arms are therefore not owed for this move. The crate tests,
C callers, header agreement, format check and warnings-denied lint are recorded in session 1678.
