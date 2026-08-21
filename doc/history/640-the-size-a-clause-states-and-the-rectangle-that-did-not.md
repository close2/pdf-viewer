# 640 — The size a clause states, and the rectangle that did not

`doc/todo/03`'s chunk again, over the SafeDocs crawl, for the eighth round running. Ten whole
archives, a negative head shallower than any before it, and **one defect — a derivation this tree
had already made and then written under a condition nothing in the clause states**. A text
annotation's icon took its size from `/Rect`; §12.5.6.4, §12.5.3 and Table 166 each say it does not.

Date: 2026-08-21.
ADR: [0471](../adr/0471-the-size-a-clause-states-and-the-rectangle-that-did-not.md).

Touched: `crates/pdf-model/src/annotation.rs` (`anchored_icon`, `ICON_SIZE`),
`crates/pdf-model/src/appearance.rs` (`text_icon`'s and `largest_square_within`'s comments),
`crates/pdf-model/src/icon.rs` (the module comment's unit-square paragraph),
`crates/pdf-model/tests/annotations.rs` (three tests),
`doc/conformance/ledger.toml` (§12.5.6.4, §12.5.3), `doc/errata-read.md` (one section),
`doc/checks/fixed-documents.toml` (two rows), `doc/todo/03` §26, `doc/todo/11` (one successor),
the ADR and this file.

## The chunk

**`2100`, `3990`, `4100`, `4605`, `6081`, `6100`, `6942`, `7065`, `7188`, `7434` — 10 000
documents**, none of the fifty-two archives sessions 603, 613, 615, 619, 625, 631 and 636 ranked,
and every remaining thousand-document archive but two. An archive is a hash bucket (ADR 0261), so
any set is unbiased. 603's instrument reused rather than rewritten, at **14 minutes 12 seconds**
for the ten thousand on fourteen workers, at a load average between 5 and 25 rising to 120 later in
the round while three other rounds compiled. **9957 rows produce a number; 43 do not.**

**Checked before it was trusted.** Both binaries built (619's lesson), `target/release/examples/`
confirmed to hold no worker of its own (624's), and §20's own check run first:
`cargo test --profile gates -p pdf-model --test fixed_documents -- --ignored` — **31 checked, 0
absent, 31 rows, green**. Six documents named by ADRs 0459, 0464 and 0468 were re-measured against
the four-renderer instrument before anything was read and reproduce to the thousandth.

## The head, and the shape it has taken

**The negative head is the shallowest any chunk has produced, and that is now a sequence rather
than an event**: −8.860 here, against 636's −10.174, 631's −43.503, 625's −112.626, 619's −84.152
and 613's −20.341. 25 rows of the 9957 are below −3 and 48 below −2.

**What sits at the top of it is this tree's own scan conversion, read from the artefacts.**
`6942935.pdf` −8.691 is a hymn sheet whose rules the producer draws as **twelve abutting strokes of
`.06 w`**, which §11.3.7.3's union composites to 0.52 of a pixel where their area is 0.72 — ADR
0308's conflation, `doc/todo/11` item 5, on a document nobody had to construct. `7434231.pdf`
−2.271, with the three references inside **0.018** of each other, is the same subclause's
anti-aliasing departure on a TeX double box thinner than a pixel. `6081615.pdf` −4.127 and
`4100532.pdf` +7.452 are `Image::area_averaged` against a decimating filter, one in each direction.
`4100873.pdf` −7.922 and `7188835.pdf` −5.129 are trap 9's family with its evidence: four-component
`ICCBased` `DCTDecode` photographs, three references inside 0.3 of one another, one colour library
between them, and the file stating `/Intent /RelativeColorimetric` — Table 51's default and ours.

**The positive head above +16 is 613's `poppler`-draws-nothing note and almost nothing else**: 39
of the 49 rows above +10 have `poppler` under a third of the heaviest reference while `mutool`, `gs`
and this tree agree. The exception is **`7188579.pdf` +19.856** and it is the opposite of a defect —
a linearised file declaring `/L 2236960` that is 310 952 bytes long, which `poppler` reduces to a
1×1 raster and `mutool` and `gs` refuse, while this tree draws the part of the scan the file carries
and says so in §7.3.8.2's own words.

## A second instrument, because ink had gone quiet

A ranking by ink cannot see a page wrong in a way that costs no levels, so the ten thousand were
asked a different question — **what does this tree report?** — with `examples/open_one` over every
one of them. **101 documents of 10 000 report anything at all on page one**, and two hold nine
tenths of the reports; both are damaged files where the references do worse than we do. It is that
sweep which turned up the successor below, and it is worth keeping as a habit: the two questions
have different blind spots.

## The defect

**`1407194.pdf` −6.304**, silent, seven commands — §25's own open lead. A book cover with a pale
yellow sticky note **250 units square** over its top-left quarter. `<< /Subtype /Text /C [1 1 0.5]
/Rect [0 542 400 792] /Contents (…) >>`: no `/AP`, no `/Name`, so Table 175's default `Note`, and
this tree inscribed the icon in the largest square inside a 400 × 250 rectangle.

**The derivation that says it should not was already in the tree, one condition too narrow.**
`anchored_icon` was written in the two-hundred-and-sixty-fifth session for `rc_annotation.pdf` and
its doc comment said "a fixed size, which is by definition not `/Rect`'s" — under
`if subtype != b"Text" || !is_empty(rect)`. §12.5.6.4 says a text annotation is "attached to a point"
and that text annotations "shall behave as if the NoZoom and NoRotate annotation flags … were
always set"; §12.5.3 says a `NoZoom` annotation "shall always maintain the same fixed size on the
screen"; Table 166 gives `/Rect` no size at all — it is "defining the location of the annotation on
the page in default user space units" — and what turns `/Rect` into a size is §12.5.5's algorithm,
which maps a **stored** appearance's `/BBox` and has nothing to map here. Not one of those sentences
mentions the rectangle's area. **A derivation and the condition it is written under are two claims,
and only the first had been checked.** → **−6.304 → +0.032**.

§12.5.6.15's file attachment and §12.5.6.16's sound keep the old arithmetic, because neither clause
states either sentence — and the test that used to pin inscribing through a *text* annotation now
pins it through a file attachment, which is the subtype whose clause states it.

## The erratum, which `check` could not see

`spec-errata emit` over all fourteen documents before writing. **§12.5.6.4 carries no annotation and
§12.5.5 carries none**; §12.5.3 carries **Issue #34, `Review/Completed`**, whose second half had
never been read here because the first half already had a verdict in `doc/errata-read.md`:

> When an appearance dictionary is not present, the rendered appearance will be implementation
> dependent.

A pure addition, invisible to `check` by construction. It does not change a behaviour; it turns
this tree's inference from silence — everything constructed for an annotation with no `/AP` — into a
citation.

## The population, measured before the change

Trap 11, with an instrument that is not this tree's (trap 8): a hand-written scanner over each
file's bytes and over every Flate stream in it, tracking `<< >>` depth while skipping `(…)` strings,
`<…>` hex strings and `%` comments. Over **67 193 files** — the crawl, `doc/corpora` and
`doc/pdf.js` — **185 `/Text` annotation records in 67 documents; 80 of them in 18 documents state
no `/AP`; 7 in 6 documents state no `/AP` and a `/Rect` with a side over twenty units.** **The
curated corpora carry not one of the seven**: every `/Text` annotation in `doc/pdf.js` and
`doc/corpora` states an `/AP` except `rc_annotation.pdf`, whose rectangle is degenerate and which
the old condition already caught. So no gate in this tree could show the defect and none moves for
the fix.

**The census was wrong twice before it was believed**, which is trap 1 one directory over: its
first version matched dictionaries with a regular expression and missed `rc_annotation.pdf`, whose
`/RC` holds `<p>Hello World!</p>`; its second blanked stream data with a pattern that also matched
the `stream` inside `endstream` and so blanked whole objects, losing `pr12564.pdf`'s thirteen. Both
were caught by asking it for documents whose answer was already known.

## What moved

**Four rows of the 62 009 differ, and no row differs for any other reason** — the machine was
quiet enough that no render lost a budget, which is the failure mode 631 measured and 633 met.

| document | ours before → after | references | gap before → after |
|---|---|---|---|
| `1407/1407194.pdf` | 39.468 → 45.804 | 46.307 / 45.932 / 45.772 | **−6.304 → +0.032** |
| `6573/6573247.pdf` | 11.241 → 2.805 | 2.995 / 3.010 / 2.977 | **+8.264 → −0.172** |
| `7557/7557734.pdf` | 27.993 → 27.414 | 27.855 / 27.389 / 28.651 | +0.604 → +0.025 |
| `2145/2145632.pdf` | 141.6065 → 141.6074 | 141.963 / 138.047 / 141.558 | +3.559 → +3.560 |

**Three of the four are in archives an earlier chunk took** — `1407` is 636's and it is §25's own
open lead, `6573` and `2145` are 631's — which is the **eighth round running** that a fix has
reached back into an earlier chunk. The fourth is in an archive no chunk has ranked and reached the
panel only because the census named it.

`6573247.pdf` is the sharper of the two visible ones and was on the **positive** side: the same
producer's note over a nearly blank page, where 250 units of pale yellow was most of the ink, at
+8.264 against three references agreeing within 0.04. It is a row 631 did not name because it sat
well below that chunk's head.

**`2145632.pdf` is the only one that does not move toward the lightest reference**, by nine
ten-thousandths of a level. Its twenty-seven text annotations state no `/AP` and rectangles at or
under twenty units, so each icon grows a little; the lightest reference there is `mutool` at 138.047
against `poppler`'s 141.963 and `gs`'s 141.558, and this tree sits between the other two. It is the
row that says what the change costs where a rectangle was already about the right size: nothing
that can be seen.

**`2100517.pdf` does not move at all**, and the reason is worth recording because the census named
it as a witness: its `/Text` annotation with `/Rect [0 0 100 100]` is object 2, and no page in the
file has an `/Annots` array at all — so it was never drawn, before or after.

**`6696835.pdf` does not move either**, and its reason is the third different one: its four notes
each state `/CA 0`, which Table 166 makes the opacity of "all visible elements of the annotation in
its closed state", so they draw nothing at any size. Three witnesses out of six that the census
named turn out not to reach the page at all — which is the difference between a population and a
reach, and the reason the second is measured rather than inferred from the first.

## What the head still holds

`doc/todo/03` §26 has it in full, and one item is new work rather than a lead: on `4605705.pdf`'s
garbage content stream `render-cpu` refuses the **whole raster** because `page_to_path` cannot
invert one singular transform, so 293 commands that did draw are lost with it. Whether a paint that
cannot be positioned costs its own mark or the page is a question about `Rasterizer`'s contract, and
it is `doc/todo/11`'s now.

## Gates

The full §2 sequence, because the change is in `pdf-model` and because `tools/round.sh` says this
is a fifth round. `RUSTFLAGS="-D warnings"` on the clippy line, which caught one lint this round's
own writing introduced — a `doc_markdown` on the word `XObject` inside a paraphrase of §12.5.5,
answered by quoting the half of the sentence that carries the rule and paraphrasing the rest without
the noun.

- `cargo fmt --all --check` silent; `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`
  silent after that one fix.
- `cargo nextest run --workspace`: **2355 run, 2355 passed, 17 skipped**, 35.1 s. Doctests: 0.
- corpus gate green; **oracle 907 agree, 66 contradicted, 786 ambiguous, 13 not comparable**, 102.7 s
  — the figures `doc/todo/02` records for a *quiet* machine, and the machine was quiet (load average
  18 falling to 3, against 120 at the middle of the round).
- `text_extraction` 4 passed; `selection_census` **1000/1011 words (98.91%) over 453 documents**;
  `accessibility_census` green; `dates`, `xmp`, `jpeg2000` green.
- `render-quorra` corpus: **957 pages, 932 agree, 23 differ, 2 refused, 17 not comparable**, 29.5 s.
- `fixed_documents`: **33 checked, 0 absent, 33 rows**, green — the two new rows included.
- `cargo test -p conformance`: green, which is what checks the four new blockquotes against
  `doc/md/` and the two ledger rows this round rewrote.

**§5's binaries were built and installed**, because this is a fifth round: the six programs plus
`libviewer_ffi.so`, 2 m 34 s for the fat link. They land in *this worktree's* `target/`, which is not
the directory a person's shell looks at — the merge owns that copy — but the link itself is a check
that the release profile still builds what a person runs, and it does.
