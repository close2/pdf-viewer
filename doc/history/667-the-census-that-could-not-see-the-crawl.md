# 667 — The census that could not see the crawl

Six ledger negatives re-derived over `CC-MAIN-2021-31` and two of them false, and the reason four of
the six had never been checked is one sentence: **the census this project built for absence claims
had its own population hard-coded, and the crawl was not in it.** §12.5.6.7's row therefore read "no
document in **any population this project measures** states `/Cap true`" — a true sentence about
`witness_census` that read as a sentence about the world. Two crawled documents state it, on
twenty-two line annotations each.

Date: 2026-08-22.
ADR: [0493](../adr/0493-the-population-a-flag-could-not-reach.md).

Touched: `crates/pdf-model/examples/witness_census.rs`,
`crates/pdf-model/examples/luminosity_mask_census.rs`,
`crates/pdf-model/examples/spec_annotation_census.rs`, `doc/conformance/ledger.toml` (§10.7.5,
§11.3.5.2, §11.5.3, §12.5.3, §12.5.6.6, §12.5.6.7), `doc/todo/01-ledger-partial-rows.md`, the ADR
and this file.

## The order the instruments gave

**`--bin overstated` first**, 0.2 s and no source opened: **8 contradictions over 170 parent rows
asserting 127 terms, 7 marked** — unchanged from 657 and 663. Its one unmarked hit is §12.7's `/AP`
against §12.7.5.5's "Table 236's `/P` is deliberately not read here", read here rather than
inherited from either verdict, and it is **noise on a ground neither of them used**. 663 argued that
a child corroborating the parent in its first line cannot be contradicting it in its last, which is
right. The sharper reason is that the two sentences are not about the same thing at all: the parent
asserts **`/AP`**, Table 170's appearance dictionary; the child denies **`/P`**, Table 236's entry in
the *signature field lock* dictionary, whose own words — "absence of this key shall result in no
effect on signature validation rules" — make it §12.8.2.2's question about what invalidates a
signature rather than anything about an appearance. `Rung::Elsewhere` is the loosest rung the sweep
has and this is what it is for: a denial about a different entry of a different table answering a
different clause. It is unmarked because the parent asserts a bare key rather than a table, so there
is no table for the `[a table read in part]` mark to divide.

**Then the blame ordering**, re-derived (616's rule): 879 commits, **242** `partial`-or-`reported`
rows with a blamed note. **663's prediction came out exactly, for the third band running** —
§10.7.5 at rank 1, §7.6.4 and §7.6.4.4 at 2–3, §11.5.3 at 4, §11.3.4 at 5, then nine sharing 6–14.

**620's rule chose §11.5.3 from the band**, and the row it chose is where step 7 was waiting.

## The rows

**§11.5.3 — a negative in two homes with no population and no command behind either.** The row is
`partial` for two residues and the one it writes out is "a blend mode inside a `DeviceCMYK` group …
a report with no corpus member"; §11.3.5.2's row says the same thing in its own words. Neither named
a corpus and neither named a run. `luminosity_mask_census` now asks exactly what
`content/transparency.rs::note_blended_luminosity` fires on — a `/Luminosity` group whose space is
subtractive with more than one component and whose own resources state a non-`Normal` `/BM`,
`/BM`'s array form included — and the answer is **0 of 1126 curated and 0 of 65 703 crawled**,
against **41 and 21 834** `/DeviceCMYK` mask groups. Thirty-three seconds over the whole crawl
through `xargs -P 8`.

**The zero was planted against before it was believed** (645's rule). A hand-built PDF with a
`/DeviceCMYK` mask group whose ExtGState states `/BM /Multiply` is counted by the census *and*
reported by `interpret`, so the two conditions are the same condition; and the census prints the
blends it finds in **any** space beside those it finds in that one, which is what tells a real zero
from a walk that finds nothing.

**And the residues are ranked the wrong way round in the row's own prose.** The other one — a
CIE-based group space, answered with the grey of the sRGB this tree converts every such space to —
is the crawl's *majority* case: **28 972 mask groups declare an `[/ICCBased …]` space of three
components, 3417 of four, 228 of one**. The residue that is spelled out is asked for by nothing at
all; the residue that is a parenthesis is what most of the world's mask groups are.

**§12.5.3 — a negative that rested on a byte search, re-derived on a walk.** "No corpus document
sets bit 9, on a scan of every **uncompressed** `/F` in all 974" is a claim about a grep: an
annotation inside a §7.5.7 object stream has no uncompressed `/F` to scan. `spec_annotation_census`
had no flag counter at all and has one now, through the object model, and over **806 668
annotations in 66 829 documents — 343 591 of them stating an `/F` — not one sets `ToggleNoView`**.
The claim survives on fifty-nine times the population and on an instrument that could have
disproved it. `Print` 316 383, `ReadOnly` 28 818, `NoZoom` 8296, `NoRotate` 8286, `Locked` 5434,
`Hidden` 3192, `NoView` 206, `LockedContents` 73, `Invisible` 85.

One document of the 65 944 is 452 MB and takes four minutes to walk; its chunk was rerun apart with
a longer bound rather than dropped, which is what makes the crawl figure all 65 703 that open and
not 65 504.

**§12.5.6.7 — false, and the instrument is why nobody could see it.** `witness_census`'s population
is `doc/pdf.js`, `doc/corpora` and this project's fixtures, hard-coded; the crawl is 65 944
documents on the same disk and was in no scope it could ask. `--crawl` is a third scope on it now,
separate rather than added, because ADR 0490's control-and-growth pair needs the two answered apart.
Over the crawl **four documents state `/Cap` and two write `true`** — `1530/1530384.pdf` and
`2514/2514866.pdf`, twenty-two `/Subtype /Line` annotations apiece, each with the `/CO` pair this
clause places the caption by. The caption construction (ADR 0431) had hand-built fixtures alone and
has producers now.

**§12.5.6.6 — false in the same run, and it is the one that shows a name census's limit.**
"`free_text_census` counts 0 of 73 free text annotations stating a `/CL`" is right about the corpus;
over the crawl the same census finds **33 of 1724 over 270 documents**. `witness_census --crawl CL`
says **81 documents**, and a spot check of four found three where `/CL` is an `/XObject` or `/Font`
resource key — ADR 0403's own warning, paid again, and the reason the number written into the row
is the structural walk's rather than the name census's.

**§10.7.5 — rank 1, kept, and ranked for the first time.** The row is `partial` for the clause's
first requirement, declined as a departure of §10.7.4's family on an argument about what this device
could do better; its only figure was "49 corpus documents set the parameter true".
`witness_census --crawl SA` finds **19 211 of 65 703**. Nothing is implemented and 620's rule leaves
a refusal resting on a reading of two clauses where it is — what changed is that the refusal now has
a size, and it is about a third of the world.

**Step 5, checked and passed.** §12.5.6.7's `a_captioned_line_draws_its_contents_where_cp_states`
does reach its claim: it states `/Cap true` with `/CP /Top`, asserts the caption is drawn rather than
named, and measures ink in the band above the line and **none** in the band below it.

**Step 6: no row read here cited a price.** §11.5.3's `partial` rests on two arithmetic residues and
§10.7.5's on a reading of §10.7.1's NOTE, which are reasons rather than estimates. What replaced the
re-derivation is the ranking above.

## `spec-errata emit` before writing

Over all fourteen documents. **§11.5.3 gets no heading at all** — `emit` goes 11.4.8, 11.6.6 — which
is 657's lesson about filing by the page a heading opens, so nothing in that neighbourhood touches
the luminosity derivation. §11.3.5.2 carries only Issue #345, a NOTE saying Figures 72 and 73 are
not colour-precise, which moves no requirement. §12.5.3 carries Issues #23, #34 and #56, all already
in its row. **§12.5.6.7 gets no heading either**, and its errata are filed under §12.5.6.8's page:
Issue #513's EDITOR NOTE says Table 179's row height in the ISO PDF obscures the end of
`OpenArrow`'s sentence — which is exactly the damage `doc/md/` carries, the word *arrowhead* leaked
into `ClosedArrow`'s cell — and Issue #515 adds "filled with the annotation's interior colour, if
any." Both are named here rather than acted on: the conversion's damage is already legible in the
table and this round changed no artwork.

## The instrument, before and after

Twelve sweeps run before the edit and after it (ADR 0485). **Every hit count unchanged** —
`overstated` 8 with 7 marked, `counts` 4, `quotations` 1 diverging, `tables` 6 denials and 98
absent, `unread` 69 rows / 182 keys, `entries` 177 over 49 rows, `pointers` 118 absent and 13
undefined, `blockers`, `capabilities`, `inapplicable`, `callers` at their standing populations. The
levels that moved are this round's own sentences: `counts` 6761 → 6767, `quotations` 1765 → 1767,
`tables` 5805 → 5807, `pointers` 7045 → 7053.

**`owed` moved and needed reading**: 181 unnamed terms over 114 rows → 182 over 115, and the row
that left the reading list is §11.5.3's. The term is **`luminosity`**, which is not a debt at all —
it is the leading segment of `examples/luminosity_mask_census`, read as a `/Key` because the
extractor looks for a solidus followed by letters. **A standing shape rather than a new one**:
`examples/border_precedence_census` yields `border` the same way. So obeying `CLAUDE.md`'s "write
down the command" rule inside a `partial` note costs this sweep one phantom every time, and the
right response is to know it — dropping the citation would be the instrument choosing what the
ledger may say, and teaching the extractor about paths would be a guess about identical characters.

## Gates

The change reaches `crates/pdf-model` (three examples), so the map asks for everything, and
`tools/round.sh` calls this a fifth round besides. The whole of `doc/todo/02` §2 was run.

- `cargo fmt --all --check` — exit 0.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — exit 0.
- `cargo nextest run --workspace` — **2413 passed, 17 skipped**, 59 s.
- `cargo test --workspace --doc` — exit 0.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — exit 0.
- `cargo build --profile gates -p pdf-sandbox --bins` and `-p hayro-compare --bin pdfref-hayro` —
  both exit 0 (trap 10).
- **corpus** — exit 0.
- **oracle** — exit 0: agrees 908 (863 on pages called complete), contradicted 65, ambiguous 786,
  our geometry 2, reference geometry 2, not comparable 13, no render 18 — every one of the seven
  the same as 663's, which is what a round that moved no drawing code should see.
- **text extraction** — exit 0: 99.8% (14257/14281 words) against PDFBox in both orders, 4 below
  90%; the position gate 10969/11163 in bounds (98.26%), 486 of 508 documents fully in bounds.
- **selection census** — exit 0: 1000/1011 words selected (98.91%) over 453 documents.
- **accessibility census** — exit 0: 102 853 elements reached, 57 116 a caret can move through.
- **dates** — exit 0. **xmp** — exit 0. **jpeg2000** — exit 0. **fixed documents** — exit 0.
- **quorra corpus** — exit 0: 957 pages compared, 933 agree, 22 differ, 2 refused, 17 not
  comparable; median page 2.72× the CPU backend.
- `cargo test -p conformance` — exit 0. **875 rows**, breakdown unchanged at 436 implemented, 224
  partial, 18 reported, 76 inapplicable, 8 writer-side, 113 out-of-scope, 0 unreviewed, no `silent`
  row. No status moved, which is right: two rows lost a false sentence about the world and four
  gained the evidence they rested on.

The reference cache was **copied** rather than shared — `PDFREF_CACHE` points at this worktree's
own copy of the 2.2 GB `pdfref-cache` — so the oracle's 908 agreements are not a read of a
directory three neighbours are writing.

**§5's binaries were deliberately not installed**: this is a parallel round told not to push or
merge, `target/` is the *main* tree's, and putting an unmerged branch's binaries where a person
runs them is what §5 exists to prevent rather than to require. The merge round owns it.

## Overlap with the parallel rounds

665, 666 and 668 ran beside this one. Nothing written here is outside §10.7.5, §11.3.5.2, §11.5.3,
§12.5.3, §12.5.6.6 and §12.5.6.7, and no other row was reflowed.
