# 473 — The seam between two marks that abut, and whose defect it is

**Finding.** The project owner's report — a dark rule shining through the polygons drawn over it on
a 50 MB Inkscape cross-section, gone on magnification — reproduces exactly and is **not a defect of
this program, and not quorra's**. The page states 58 003 opaque filled paths with no clipping path
at all; §11.3.7.3 unites each mark's anti-aliased coverage with what is already there by an
"inverted multiplication", so two marks covering half a boundary pixel each leave a quarter of
whatever is beneath, and *n* equal shares rise towards `1/e`. `render-cpu` leaves 0.2510,
`render-quorra` 0.2471 and `render-gpu` 0.2510 against the union's 0.2500; `mutool` leaves 0.2510
beside them. The two references that appear not to have it are the two not anti-aliasing the shape,
and a control fixture says so.

**Date.** 2026-08-13.
**ADR.** [0308](../adr/0308-the-seam-between-two-marks-that-abut.md).
**Touched.** `crates/render-quorra/tests/abutting_marks.rs` (new, three tests),
`crates/pdf-model/examples/uncovered_share.rs` (new instrument),
`doc/conformance/ledger.toml` (§10.7.4, §11.3.7.3),
`doc/todo/11-shapes-that-still-disappear.md` (item 5),
`doc/todo/_scan-conversion.md` (departure (2) has a witness), `doc/todo/README.md` (one line, which
said the case was only a tiling cell's), `doc/adr/0308-*`, this file. **No rendering code changed**,
deliberately: the round's outcome is a measurement, a gate on the arithmetic and a priced decision,
not a fix.

## The answer to the owner, in plain words

The line is real, it is drawn *under* the coloured tiles, and the tiles do not quite cover it. Each
tile is a separate filled polygon and this program draws smooth edges, so a tile that covers half a
screen pixel is painted at half strength there. Two tiles meeting inside one pixel therefore paint
`1 − ½ × ½ = ¾` of it between them and a quarter of the line underneath still shows. On this
drawing four to six tiles meet in a pixel at page size, which is why it is a fifth of the line
rather than a hundredth. Magnify and each tile covers many whole pixels, the boundaries thin out,
and the line goes: measured at 0.1937 of the layer beneath at page scale, 0.0673 at 4× and 0.0156
at 8×.

It is not something one of the three rasterisers is doing wrong — all three of ours agree to a
level of 255, and so does `mupdf`. `ghostscript` with anti-aliasing switched off shows nothing,
because ISO 32000-2 §10.7.4's own scan-conversion rule is aliased and paints whole pixels; that is
the clause working rather than a renderer being clever, and it is the reason the rule reads the way
it does. `pdftoppm` also shows nothing on the obvious test and **that is a trap**: poppler snaps an
axis-aligned rectangle's edge to whole pixels, so it is not anti-aliasing that shape at all. Turn
the seam a few degrees and poppler leaks with everyone else.

Removing it means resolving the marks against one another before compositing either. ADR 0308 and
`doc/todo/11` item 5 price the two ways of doing that; neither is started, and the reason is
written down rather than left implicit.

## Two things about the instruments, for the next round

1. **`uncovered_share` counts only interior pixels, and that is the whole of why it means
   anything.** A pixel on the *outer* edge of what a page's marks cover is supposed to be partly
   painted — that is anti-aliasing working — so a page-wide average of "what did not get covered"
   is dominated by legitimate edges and ranks a page of eight-point text as the worst thing in the
   corpus. The first version of the example did exactly that and read 0.52 on page 100 of ISO
   32000-2, which is a number about type size and nothing else.
2. **A display list may not be rebuilt command by command.** The first version of the splice built
   a fresh `DisplayList` and pushed every command into it; clips and soft masks are referred to by
   index into tables the new list does not have, so the first clipped command came back
   `UnknownClip(ClipId(0))`. `split_off_commands` cuts the list and keeps both tables, which is
   what the example does now.

## Gates

`fmt`, `clippy --workspace --all-targets` (silent), `nextest --workspace` **1700 passed, 11
skipped**, doctests, the sandbox worker build, `pdfref-hayro`, both text gates, dates, xmp,
jpeg2000 and `conformance` — and the three that count a population, every one of them on the
number the four-hundred-and-seventy-second recorded:

```text
  corpus     974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless,
             65 incomplete, 0 slow
  oracle     906 agrees (861 complete), 67 contradicted (66), 786 ambiguous (754);
             reference cache 99.8%
  quorra     956 pages: 918 agree, 37 differ, 1 refused, 18 not comparable
```

Nothing that draws was touched, so no gate could move and none did; the run is the claim rather
than the change.

`doc/todo/00` step 7's ink sweep is **not** owed: the round changes no pixel on any page, which
`git status` says directly — no file under `crates/` outside a new test and a new example.

The quorra gate's second coverage lane is not owed either: `doc/todo/02` §2 asks for it from a
round that takes a quorra release or changes the zoom path, and this round did neither.

## A note on this worktree

It was created from a branch **98 commits behind `main`**, at `aea1228` from 6 August, where the
document in question still drew as a fraction of itself. Every measurement above was taken after
fast-forwarding onto `main`; the first half hour of the round was spent measuring the wrong tree
and finding a rule that was not there. **A worktree round's first command should be
`git rev-list --count HEAD..main`.**
