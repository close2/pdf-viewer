# 579 — the oracle's last two unwatched verdicts, and the refusal that enters the vote as a page size

Date: 2026-08-18. ADR: [0414](../adr/0414-a-refusal-that-exits-zero-and-the-two-buckets-nobody-watched.md).

Touched `crates/pdf-model/tests/oracle.rs`, `doc/conformance/ledger.toml`,
`doc/oracle-and-corpus.md` (new §3e), `doc/todo/00-ambiguous-bucket.md`, `doc/todo/README.md` and
ADR 0410.

## The demand half — `not comparable` and `reference geometry`

ADR 0410 wrote down what it was leaving, and this round took it: the gate prints seven verdicts, and
after that round five were held. `reference geometry` (2 pages) and `not comparable` (13) were
printed and gated by nothing, on the reasoning that neither accuses this tree *by construction* —
which is a claim, and was equally true of `no render` for four hundred rounds.

All fifteen were asked on §3d's recipe, by hand. **The claim survives**: nothing in either class is a
page drawn wrong or a page not drawn, and both are held by name now, over *all* pages rather than the
complete ones. Two things came out of asking.

**On four of the fifteen the one reference that got to a page agrees with us**, which is a statement
about pages the gate has never judged. `poppler` and this tree get past §7.6's encryption on
`auth-event-ef-open.pdf` and `encrypted-attachment.pdf` while `mutool` and `gs` decline — 0.06 of 255
between our raster and `poppler`'s on both. `gs` alone rebuilds `issue9418.pdf`'s and
`poppler-67295-0.pdf`'s cross-reference tables — 3.15 and 0.14 of 255 from ours, the first on a
3024 × 2304 text page whose class bound is 5.00. And `mutool` alone draws `bug1978317.pdf`, 1.69 of
255 from ours over a 612 × 792 sheet.

**And `reference geometry` was wrong about both of its own members.** `pdftoppm` writes a **1 × 1
raster and exits 0** when it fails to create a page, so a refusal enters `reconcile` as an opinion
about the page's extent and outvotes the renderer that drew. On `bug1978317.pdf` it says why
(*Failed to create page*, an annotation array it calls too big); on `boundingBox_invalid.pdf` page 3
it says nothing at all, and the only tell is the size. Neither page has references disagreeing about
a size; both have one reference and two refusals. Trap 3 says to check what question a reference was
asked before reading its answer; this is the same rule one step earlier — **check whether it answered
at all, because a program can decline while returning success.**

The classifier is not changed for it, deliberately: "a 1 × 1 raster is a refusal" is a rule about one
program's output shape, and a page whose crop box really is a point square would be misread by it.
The verdict already prints every reference's size and the group's note now says to read them.

`boundingBox_invalid.pdf` page 3 is the construction ADR 0410 named and left — *Empty /CropBox and
/MediaBox intersection*, `/MediaBox [0 0 600 800]` against `/CropBox [600 800 1000 1000]`.
§14.11.2.1's `shall` is applied and the intersection encloses no area, which the clause states no
recovery for; Table 31 does, and this is where it parts company with ADR 0389's media box — the
fallback is to a rectangle **the file states**, not one this program invented, so it is right and it
is not reported, and those are one argument rather than two. We draw 600 × 800 at ink 1.502 and no
reference draws the page at all.

**And the bucket's artefacts used to be deleted.** `examine` called `remove_dir_all` on a page whose
references could not be reconciled, so the one bucket that cannot be diagnosed from disk was the one
bucket with no disk. Three of the thirteen have a reference raster worth seeing, and it had to be
re-rendered to see one. Kept now, with our own raster written beside it.

## The spec half — three sweeps, one row

`tables`, `entries` and `unread`, none of them run in the round before. `tables` and `unread` are
clean and the sharpest suspect in each was checked rather than assumed: `hostile_budgets.rs`'s "Table
87 gives an image mask no `/Mask`" reads as a denial the table contradicts and is exactly right, on
the entry's own "(Optional; **shall not be present for image masks**; PDF 1.3)"; §12.6.4.3's `/SD` is
one short key in two clauses, `destination.rs` reading Table 202's while the row denies Table 203's.

`entries` paid on its own discriminator — an entry the row's *note* does not name. **§12.11.1 is
`implemented`, its note enumerates Table 273's common entries, and it walks past `/RH`**, the fifth
of the five, sitting between the `/V` and the `/Penalty` it does discuss. The entry carries a `shall`
— a handler that "shall be disabled (not invoked) if the interactive PDF processor can check the
requirement specified in the S entry" — and the condition **fires**, because `Kind::unmet` answers
all twenty-five of Table 275's types. It is discharged by §12.11.5: such a handler is an ECMAScript,
which `CLAUDE.md` excludes, so every handler a file could name is disabled here whatever the file
says. §12.11.5's row has carried that reading since it was written; what was missing was the
neighbour's enumeration — `doc/todo/01`'s fifth failure shape between two clauses that do not cite
each other.

## Gates

Everything `doc/todo/02` §2 lists, all green, with the oracle run last: fmt clean, clippy silent
across the workspace (the `viewer-qt@0.1.0:` lines are gcc's about generated code, as that file
records), 2147 nextest tests, the doctests, the corpus gate at 974 documents with 66 incomplete, the
oracle at 1794 pages — 906 agree, 67 contradicted, 786 ambiguous, 2 our geometry, 2 reference
geometry, 13 not comparable, 18 no render, every bucket identical to the run this round opened with —
the text, date, XMP and JPEG 2000 gates, the quorra corpus at 957 pages, and `cargo test -p
conformance`. Release binaries and `libviewer_ffi.so` installed.

**No step 7.** This round moved no pixel: its whole diff under `crates/` is `tests/oracle.rs`, so a
before-and-after ink sweep would compare a file with itself — which is the four-hundred-and-sixth
session's own note about when that sweep is not owed.
