# 732 — The codec a boundary waits on

ADR 0607's decision, carried out: a whole `pdf_render::DisplayList` crosses the confinement as
bytes, `Arc` identity preserved across four interned tables, with the two deferred producers
refused **by name** into the raster arm and a fuzz target beside `confined_wire`. ADR 0626.
Date 2026-08-25.

## What was built

`crates/viewer-confined/src/protocol/display_list.rs`, reachable as
`viewer_confined::wire::{encode_display_list, display_list, crossing}`.

The format's shape is one decision repeated: **the tables precede the body**. Paths, image
samples, shading kinds and shadings are written first; then the clips, the soft masks and the
commands that index into them. So every identifier the *host* reads is bounds-checked against a
length the decoder established by reading a table, never against a length the confined side
asserted. It costs the encoder one deferred buffer, because the interning that fills the tables
happens while the commands are being written.

Three other bounds, each a place a length could have become an allocation or an index: a count is
checked against the bytes that could hold it **per table, at the smallest record that table
admits** (tighter than `Reader::list`'s one-byte-an-element assumption, and it refuses 2^28 clips
in a nine-byte message before anything is reserved); nesting is bounded by
`pdf_render::MAX_GROUP_DEPTH`, which is what every backend already refuses past rather than an
invented number; and an image whose stated dimensions its samples do not fill is refused, which is
the same invariant `confined_wire` already asserts of a `Raster`.

`Arc<ShadingKind>` is a table of its own, which ADR 0607's accounting did not separate. It is what
turns `bug1721218_reduced.pdf`'s 3576 mesh paints back into three geometries.

## The two things a smaller design would have got wrong

**The whole clip table crosses, not only the reachable part.** `examples/list_against_raster`
priced only the clips something references, which is right for a cost estimate and wrong for a
codec: a `ClipId` is an index, so dropping an unreferenced entry renumbers everything after it.

**And the decoder checks that rebuilding the table reproduces the message's own numbering.**
`add_clip` hands back the identifier of a region *already there* — ADR 0132's whole point — so a
message stating one region twice would silently renumber every identifier after it and every
command naming one would clip by the wrong region, in the unconfined process. A parent is
separately required to sit strictly before its child, which is what keeps `clip_bounds` from
walking a cycle. Both are `ProtocolError::Unbuildable`, a new variant whose subject is a message
that is well formed and describes a value the type does not admit.

## The price, re-derived rather than quoted

`examples/list_over_the_wire` runs the encoder over 958 first pages of `doc/pdf.js` — byte counts,
so load-immune — and decodes and compares every one before printing its number. ADR 0607's
predicted column and this one, side by side, are in ADR 0626 section 5. The short of it: **median 0.0368
against a predicted 0.034 at 72 dpi, 0.0207 against 0.019 at a window's scale, p90 1.0002 against
1.000, worst 1101.09× against 1101×.** The prediction held to about a tenth, and the aggregate is
0.4225 against 0.37 because a real format has tables, tags and length prefixes.

954 of 954 encodable pages round-tripped to an equal list. The four that cannot be encoded are
named rather than counted: `function_based_shading.pdf`, `function_based_shading_cmyk.pdf`,
`issue16263.pdf` and `issue19517.pdf`.

## What the fuzz target covers

`fuzz/fuzz_targets/display_list.rs`, seeded from real pages by `examples/list_over_the_wire
--seeds`. Beyond "nothing panics" it asserts three things, so that deleting a check in the decoder
fails the target: every identifier a decoded list holds points at something, every decoded image's
samples fill its stated dimensions, and **anything this decoder accepts this encoder can write,
reading back the same list** — which is what catches the two halves of a codec drifting.

The seeder is an example rather than a Python script, because producing a display list means
running the interpreter, and it bounds what it writes at 256 KiB: unbounded, the pdf.js corpus is
841 MB of seeds and almost all of it is four scanned documents' pixels, which state nothing about
this format and are paid for in every merge.

**It found something on its first run, at 750 executions, and it was the target rather than the
codec.** `one message decoded two ways` — on an input stating NaN, because `DisplayList`'s
`PartialEq` is ultimately `f32`'s and `f32`'s is not reflexive. The decoder deliberately does not
refuse a non-finite number (ADR 0626 section 7: the confined path must draw the page the in-process path
draws, so a value the interpreter can produce may not be refused at the transport), so what
changed is the assertion: the target compares the bytes of a re-encoding, which is total where
equality is not and which asserts the stronger property besides — that the encoding is a
**canonical form**. Clean afterwards at **4 175 795 runs in 901 s**, 2709 edges, no artefact.

## What was deliberately not done

The codec is **not wired into `Answer::Frame`**. That is the tier change, it breaks every consumer
of `Reply::Frame`, it decides which host rasterises, and it bumps the frame protocol's `MAGIC` —
three arguments rather than one, and ADR 0626 section 6 states each. `doc/todo/15` holds them now.

## The gates

Whole: a `pdf-render` change is under everything, and `ClipId::new` is a `pdf-render` change.
Both workers built first. The fuzz run was taken before the sequence rather than beside it.

## Ledger

Untouched. This is `CLAUDE.md` principle 3 against principle 2 and cites no clause.
