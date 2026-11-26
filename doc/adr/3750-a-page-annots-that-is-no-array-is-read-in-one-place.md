# 3750 — A page `/Annots` that is no array is read in one place

Session 2262. Status: **accepted**. Context: ADR 3477, ADR 3543, ADR 3738; ISO 32000-2 §7.7.3.3
Table 31.

## Context

ADR 3738 held `script_sites.rs`'s last site: a page `/Annots` that is no array was read as listing
nothing, and none of the thirty-three readers of the entry in `pdf-model` logged it.

## Decision

- `crate::annotation::page_annots` is the one read of a page's `/Annots`: an array or an absent entry
  is returned as it is, and a value of another type is read as absent (`Object::Null`) and logged as
  the new `model/page-annots-not-an-array` at the file, from a `#[cold]` function. Every reader of
  the entry in `pdf-model` — the interpreter's annotation pass, the views, forms, structure, links,
  actions, tab order, the XFA package, submission — goes through it; each reader's own handling of
  the answer is unchanged.
- The class breaches section 5.1 alone. It is a page entry, so ADR 3543's answer holds: the rewrites
  that restate a page and the output's own check, which refuses by the class's name.

## Consequences

- `script_sites.rs` held 1 → 0.
- The readers outside the reader crate (`pdf-transform`'s rewrites, `pdf-archive`'s survey,
  `pdf-signature`, `viewer-core`) still read the entry themselves; a reader there that draws or
  validates is the next to route through a public form of this function.
