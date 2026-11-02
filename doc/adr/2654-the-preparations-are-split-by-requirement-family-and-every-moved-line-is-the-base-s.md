# 2654 — The preparations are split by requirement family, and every moved line is the base's

Session 1910, slot 10 of batch 131, a docs round. Status: **accepted** and **built** in
`crates/pdf-transform/src/archive/prepare.rs` and `crates/pdf-transform/src/archive/prepare/`.
Context: the owner's code-quality slot; ADR 2634, whose method and families this follows; ADR 1023
(a moved comment carries no history). ADR 2655 and Q786 are unused.

## The family map

`prepare.rs` held, past 7 400 lines, every preparation a decision reads. It keeps what more than one
family reads — `Prepared` and `Prepared::of`, the one entry that fills it, `Owed`, `Spare` and
`MAX_SPARE_NUMBERS`, and the types and sentences two families share. Six modules under `prepare/`
hold the rest:

| module | what it prepares | rows it serves |
|---|---|---|
| `font_rewrites` | the font objects the Unicode requirements failed at, metric directions, recorded substitutions | `decision/fonts` |
| `colour_and_graphics` | the output intent, its destination profile, the `DeviceN` `/DefaultCMYK` | `decision/colour_and_graphics` |
| `files_and_structure` | the structure tree a `/Marked` flag promises | `decision/files_and_structure` |
| `annotations_and_attachments` | appearances written or restated, annotation removal, embedded files' outcomes, the kept XFA resource | `decision/annotations_and_attachments` |
| `metadata` | the XMP packets edited, replaced or written, extension schemas, the information dictionary moved into the packet, the header cut | `decision/metadata`, and the `file-structure/` information-dictionary rows whose remedy is the packet |
| `appended_pages` | the preserved pages, the statement page, attachments laid out as text, marks relocated onto the producer's page | the `preserve` remedy and the two authoring amendments' constructions |

There is no `content_stream_fences` module: no preparation is a fence's. The fonts module is not
`fonts` because the parent already reads `super::fonts`, the rewriter it feeds. The packet header's
cut sits with `metadata` because its requirement is `metadata/xmp-packet-header-attributes` and it
is the first of the three edits to one packet.

## The method

A script reads the base file, cuts it into items, seeds each family with the preparations named
above, and places every other item with the one family whose items read it; an item two families
read, or only the parent, stays. It writes each item's lines verbatim and then runs rustfmt. The
seven rewrites it may make to a moved line, and no other:

1. `super::` becomes `super::super::` (not inside the moved `tests` module);
2. `pub(super)` becomes `pub(in crate::archive)`, the same scope one level deeper;
3. a private item another file reads gains `pub(super)` on its head line, or `pub(in crate::archive)`
   where a file outside `prepare` reads it;
4. `include_bytes!("../` becomes `"../../`;
5. a rustdoc link to a name the file reads only in documentation gains its explicit target, since
   an import used only by a doc link is reported unused;
6. `ReplacedDestination::intent`, which the parent calls, gains `pub(super)`;
7. the fields of `PreparedFonts`, `Edited` and `Recording`, which the parent builds by literal,
   gain `pub(super)`.

The parent's `use` declarations lose the leaves only moved code read; each module's imports are
written from the base's declarations, a comment above one carried with it as its own group. The four
names `archive/mod.rs` re-exports publicly are `pub use`; every other name read outside `prepare` is
re-exported `pub(super) use`, so no caller's path changed.

## The proof

A line-for-line comparison, the rewrites undone: of the base's 7 318 non-blank lines (7 286 outside
`use` declarations), 7 272 match with multiplicity; the 14 others are lines rustfmt wrapped, whose
tokens are the generated 42 lines' but for three trailing commas and one pair of braces; 287 string
literals match with their multiplicity; every one of the base's 1 952 plain comment lines and 57
imported names is present. The gates are the round's record.

## What it costs a reader and buys them

A reader of one remedy's preparation opens its family's module and `prepare.rs` for the types it
fills. A family's preparation, sentences and helpers now sit together under the heading
`doc/pdfa-blocking-issues.md` uses, and a round working on one family touches one file its siblings
in the same batch do not. `metadata` and `prepare.rs` stay past 2 000 lines each; a further split
of either is its own round.
