# 3797 — A `/Group` that states no subtype is logged, and the census's `?` blind spot is read

Session 2274, the silent-repair campaign. Status: **accepted**. Context: ADR 3477, ADR 3521, ADR
3792; ISO 32000-2 §8.10.3 Table 94, §11.6.6, Table 31, Table 93.

## Context

ADR 3792 left one site the census cannot see: `transparency_group` read a `/Group` that is no
dictionary, or one whose `/S` is absent, as no group, through `?` — and the census counts
`unwrap_or…`, `.ok()` and `.clamp(` only. The page's own `/Group` was read the same way, five
times over, in `blending_space.rs`, `press.rs` and `knockout.rs`.

## Decision

- **A class, `model/group-attributes-unusable` (§8.10.3)**, mapped to section 5.1 alone in
  `pdf-archive/src/repairs.rs` and listed in the content grammar's `COVERAGE` as the interpreter's.
  §8.10.3 has "[t]he value of this entry shall be a subsidiary group attributes dictionary", and
  Table 94 makes `/S` "(Required)": an entry that is no dictionary, or one whose `/S` is absent or no
  name, states no subtype, and is read as stating no group. A `/S` naming another subtype is §11.6.6's
  ordinary form and no repair; a null is the entry's absence (§7.3.9).
- **One reader, `transparency::transparency_attributes`**, which every `/Group` read in the content
  walk goes through: forms, annotation appearances, soft mask groups and imported pages at the
  content that invoked them (a form's group is now read before the stream becomes the form's, as its
  `/Resources` is), and the page's own group, through `page_group_attributes`, at the page object, as
  the page tree's entries are.
- **The sweep.** `?` on an `Option` read in `pdf-model/src/content`, one line at a time, finds
  fifteen. The two of `transparency.rs` and the one of `blending_space.rs` are this class. The others
  are not repairs: `/Alternates` (`image.rs`) and `/StructParents` (`marked.rs`) are optional entries;
  `grammar::components` answers "cannot count" to a caller that judges the space itself;
  `resources.rs`'s two are a name looked up in a category whose wrong type
  `model/resource-category-not-a-dictionary` already logs; `tiling.rs`'s is the dispatch on a
  `/PatternType 1` object being a stream; `press.rs`'s and `blending_space.rs`'s page reads now go
  through the class; `operands.rs`'s two are operand reads, whose shortfall and type the grammar's
  own classes log.

## Consequences

- The census still does not count `?`; the sweep is a reading, done once, and its command is
  `grep -rnE '\.as_(dict|name|array|number|integer|stream)\(\)\?' crates/pdf-model/src/content`.
- A malformed page `/Group` is now in the log a validator reads, and a converter that restates the
  page knows the group it dropped.
