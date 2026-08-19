# 600 — The two censuses that are one walk

Both tracks: `doc/todo/54`'s last two residues, which close the file, and one `partial` ledger row
read against the code it describes.

## The demand-driven half

`doc/todo/54` was the list quorra's own account of where it stands left this tree. Two items were
open, both recorded as ours in `doc/QUORRA_FEEDBACK.md` since §25, and both standing for the same
reason: each is a question about a corpus and nobody had spent the walk.

**The censuses are one instrument** — `crates/render-quorra/examples/rect_and_residue_census.rs`,
which walks the pdf.js corpus's first pages at 1×, the population and scale `tests/corpus.rs`
renders. It reports what it matched before any share is taken of anything, which is trap 11 in the
shape a census needs it: 955 pages drawn, 3 refused by the device, 16 not readable as a first page,
223 532 fills reached. Thirteen seconds. **Run twice, byte-for-byte identical** — five times, in
the end — the five-hundred-and-eighty-first session found a census whose answer moved between runs,
and the rule that came out of it is the reason the second run happened.

**One fill moved between two builds of this session**, and it is in ADR 0435 rather than here
because it is an argument: not this round's code (measured again with `collection.rs` reverted), and
a glyph is a `Command::Fill`, so the suspect is a font substituted from the system between the two
measurements. A page's display list is then not a function of the file alone — named, not chased.

The numbers are in `doc/QUORRA_FEEDBACK.md` §34, which is where they go, and the argument is in ADR
0435. Three sentences of it belong here because they are what the round learned:

- 925 of 955 pages report `(0, 0)` for `(clip_residue_regions, clip_residue_tiles)`. Upstream's
  divided encode gains 6.6× on a drawing and 1.2× on their `artwork` archetype, and residue-clipped
  marks are the difference; **this corpus is 97% drawing**. That also says dividing the residue
  rasterisation would move thirty pages here, which is this tree's reason not to ask for it.
- 6.30% of the fills are one axis-aligned rectangle and nothing else; 5.81% reach the device as
  one, the gap being 1 095 rectangles under a transform that does not preserve the axes. Two rows
  rather than one, because a fact about a path is not a claim about a device.
- **The fifth-frame tile-cache loss is not a repack and no longer reproduces.** ADR 0368 left it
  open between the atlas's capacity and a transform nobody looked at; `atlas_repacked` is false on
  every frame of the five-frame session, and frames 4 and 5 cost about 92 ms against 465–727 for
  the first three. Run twice.

**What the round added to reach them**, and each is one item: `pdf_render::crop::whole_rectangle`
made public, so the census asks the shipping predicate rather than a copy of it;
`QuorraRasterizer::last_clip_residue`, which is deliberately *not* a `FrameCost` field for the
reason §25.3 gave upstream; and `ZOOM_FRAME_SEQUENCE` on `examples/zoom_frame.rs`, which is its
two-frame pair generalised to a session's magnifications, with `repacked` on the frame line. The
default sequence is the pair at the same two targets, so every table taken with that example before
today is still comparable.

`doc/todo/54` is deleted and its line is out of the index.

## The spec-driven half

**§12.3.5.2, `partial`** — collection hierarchical folders. The row's own note said the clause
defines a file name "by five requirements" and then listed four. The one it dropped is the one the
code never applied:

> The number of characters in the string shall be between 1 and 255 inclusive.

`collection::is_file_name` tested `!name.is_empty()` — the 1 of that bound and not the 255 — under a
doc line claiming the clause's five rules, beside a test named
`a_folder_name_is_a_file_name_by_the_clauses_five_rules` that asserted four.

**The refusal shape is `doc/habits.md`'s wrong diagnosis**: the reason written beside the omission
was that "truncating one would rename a file", which is a reason not to truncate, and
`is_file_name` is a `bool` predicate that truncates nothing and stores nothing. Answering `false`
*is* the answer, and the clause's own next sentence is what makes answering safe — "[a]n
interactive PDF processor may choose to support invalid names or not". The bound counts characters
rather than bytes, as the sentence says, so it is `chars().count()` and a test asserts that 255
two-byte characters are a valid name.

The row, the doc comment and the test now say five and mean five.

## The gates

The full sequence, this being a fifth round, and §5's binaries rebuilt and installed — `round.sh`
had flagged `target/pdf-sandbox-worker` as older than `HEAD` two rounds ago and §5 had not run
since. `tools/state.sh` is what prints the numbers; nothing about them is written here.

## Files

`crates/render-quorra/examples/rect_and_residue_census.rs` (new), `crates/render-quorra/src/lib.rs`,
`crates/render-quorra/examples/zoom_frame.rs`, `crates/pdf-render/src/crop.rs`,
`crates/pdf-model/src/collection.rs`, `doc/conformance/ledger.toml`, `doc/QUORRA_FEEDBACK.md` (§34,
and two earlier sections marked closed), `doc/adr/0435-…`, `doc/todo/54-…` (deleted),
`doc/todo/README.md`, this file.
