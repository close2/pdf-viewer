# 2032 — The redaction walk is written in one module per concern

Session 1598, slot 7 of batch ninety-four, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 3. Status: **accepted** and **built** in `crates/pdf-transform/src/redact.rs` and `redact/`.
Context: the owner's words of 2026-10-19, "Refactor god classes,..." and "Do not fix any bugs.";
ADRs 2004 and 2018 (`Viewer` and `ViewState`, split the same way) and ADR 2005 (the ranking).
Clauses: none; no behaviour moves. ADR 2033 is unused.

## 1. What the census found

`Walk`, the one walk that rewrites a content stream under a redaction region (ISO 32000-2
§12.5.6.23), was a 2 504-line `impl` of 64 methods in a 6 052-line `redact.rs`, beside six modules
(`forms`, `located`, `mesh`, `paths`, `shading`, `type3`) that already held an `impl Walk<'_>` or the
constructions it calls.

## 2. The decision

`redact.rs` stays the root. It keeps the state (`Walk` and the types it holds), the entry points
(`run`, `Walk::new`, `Walk::run`, `run_stream`, `operator`) and everything that plans and writes a page.
Seven new modules each hold an `impl Walk<'_>` for one concern:

| module | concern |
|---|---|
| `text` | a `Tf`'s font, a string's codes, the showing operators and the rewrite that deletes codes |
| `painting` | path construction (§8.5.2), the painting operators, the cut, a stroke's outline and its restated state |
| `frames` | `Do`, entering a form, the frame saved and restored around it, the recorded form edit |
| `images` | the plans that clear an image's samples: packed, codec, `JPXDecode`, masks, ownership |
| `inline` | the inline image splice (§8.9.7) |
| `graphics_state` | `gs`, soft masks and whether an object may be replaced in place |
| `regions` | the region's boxes and the unit square in the stream's space |

`doc/todo/74` named `text`, `paths`, `images` and `forms`. `paths` and `forms` already exist as
constructions (the geometry of a cut, the splitting of a form drawn twice), so the walk's side of
them is `painting` and `frames`; the inline splice and the `gs` operator are each a clause of their
own (§8.9.7, §8.4.5) and are `inline` and `graphics_state`; and the region's boxes are asked by text,
paths, images and shadings alike, so they are `regions`.

**Visibility is what a sibling reads.** A method is `pub(super)` only where the compiler named a
sibling or the root that calls it (22 of them); every other method stayed private to its module.
`Walk`'s fields stay private to `redact` and its children, so no field was widened. The free
functions and the types stayed in the root: the children name them through `super`, and moving them
is a different split (see §4).

## 3. The proof that nothing moved

The multiset of non-blank lines, whitespace stripped (habit 93): the old root and six modules
against the new root and thirteen modules. Removed-only: 24 lines, which are 22 signatures that gained
`pub(super)` or were re-wrapped by `rustfmt`, the old `ImageError` import and the module table line.
Added-only: the seven modules' doc comments, `use` blocks and `impl Walk<'_> {` wrappers, the root's
module list and its paragraph of module doc, and the 22 `pub(super)` signatures. `cargo test -p
pdf-transform` passes unchanged and clippy with `-D warnings` is clean over all targets. Twenty-five
ledger rows named `redact.rs`; each now also names the new module that cites its clause.

## 4. What this costs

`redact.rs` is 3 837 lines still: about 2 000 of them are the page's planning and writing
(`write_document`, `build_page`, `Carried`, the sample arithmetic and the inline-image builder) and the
types they pass, which are a different type's methods and a different split. A reader looking for
`Walk::plan_codec_clear` finds it in `redact/images.rs`.
