# 979 — A construction refused by its own neighbour

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `9fac224b`. Stream: transparency,
ISO 32000-2 clause 11. Four sibling rounds were live in the same working tree throughout (980 in
colour, 981 in fonts, 982 in `pdf-archive`, 983 in `pdf-syntax` and `tools/conformance`).

Argued in **ADR 1000**.

## What the round chose, and why

The brief named five `partial` rows of clause 11 and asked for the one with the largest pixel
consequence. Four of the five are *reported* today with zero corpus witnesses each — §11.4.4's
blend mode at the `Do`, §11.3.7.2's shape channel, §11.7.5.3's stated black generation, §11.7.2's
conversion between two presses — and the fifth, §11.7.4.4, was the one row whose witnesses are
drawn **wrong** rather than refused: `issue17215.pdf` and `issue14438.pdf`, the corpus gate's two
`CompositedInParts` documents, each a `B` under a blend mode drawn flat with its stroke composited
over its own fill. A page drawn wrong outranks a page reported, so that was the row.

## What the clause says and what the code did

§11.7.4.4's second bullet establishes "a non-isolated knockout group" in which "the fill and
stroke shall be performed with their respective prevailing alpha constants and the prevailing
blend mode", composited "using an alpha value of 1.0 and the Normal blend mode"; §9.3.8 says the
same of a text object under `Tk`. `transparency::knockout_group_elements`, asked by both callers,
refused a part under a soft mask and any part that blends. The first refusal was recorded in
§11.6.2's row as deliberate — while `knockout_elements`, forty lines above in the same file, had
stated a masked element's shape for a form's knockout group since ADR 0234. The second was true of
a group drawn on transparency and answered by nothing.

## What changed

- `crates/pdf-model/src/content/transparency.rs` — `implicit_knockout_group` and
  `ImplicitKnockout`: three constructions, each exact (ADR 1000 §4). Nothing blends: the parts on
  transparency, masked ones as `Command::Shaped`. One shared blend mode that is affine in its
  source (Multiply, Screen, Overlay, Exclusion) or one colour: the mode moved to the group's `Do`,
  drawable by every backend. Otherwise §11.4.6's own backdrop (ADR 0327's construction), not
  inside a knockout group. `knockout_group_elements` keeps its signature and is the new function
  with the two answers its callers cannot state filtered off, so both callers take the masked case
  today. Four unit tests hold each route against §11.4.6's arithmetic and against the flat drawing.
- `doc/conformance/ledger.toml` — rows §11.7.4.4, §11.6.2 and §11.4.6.
- `doc/todo/23-transparency-departures.md` — one section.
- `doc/adr/1000-…`, this file.

**Not changed, and owed**: `crates/pdf-model/src/content/path.rs` and `content/text.rs` still
ask `knockout_group_elements`. The three-line switch at each of their three sites is written out
in ADR 1000 §5; `text.rs` was the font round's file and `path.rs` was assigned to nobody, so
neither was this round's to edit. The `path.rs` form was applied here **temporarily, to measure,
and reverted** — `git status` shows the file clean.

## What it measures, with the `path.rs` switch applied

- `issue17215.pdf` at scale 2: 197 pixels changed; the black line inside the yellow shape (the
  stroke's `Difference` against the fill) is gone. `mupdf` and `poppler` draw the shape blue —
  `Difference` against white — which is the standing §11.4.7 reading in §11.3.6's row, not
  reopened here.
- `issue14438.pdf` at scale 2: 125 pixels changed, the rim of one `Square` annotation under
  `Multiply`; at a rim pixel `(255, 255, 116)` → `(255, 255, 132)`, and `mutool` draws
  `(255, 255, 132)` there.
- Corpus gate (`--test corpus`): `974 documents in 12.5s: 0 unopenable, 9 locked, 1 encrypted
  beyond us, 5 pageless, 61 incomplete, 0 slow`, exit 0; the neighbouring round's run of the
  unpatched tree the same evening printed 63 incomplete with the two witnesses in it.
- Oracle: `agrees 990 total, contradicted 62, ambiguous 835, not comparable 47, no render 17`,
  **exit 101** — `1 page(s) newly ambiguous without a diagnosis: ["issue14438.pdf page 1"]`, at
  `mean 6.12 worst tile 48.61 differing 11.84% ssim 0.8871` against a bound of `5.00 / 40.00 /
  5.00% / 0.9000`. The page was outside the judged set while it carried a report; the side-by-side
  shows the five renderers parting over its four `Ink` annotations with no `/Rect`
  (`poppler.log`: "Bad bounding box for annotation", "Bad Annot Ink List", four times) and
  `hayro` leaving the title's highlight without its text — nothing on the `B`'s box. Whoever
  applies the switch owes that page a line in the oracle's `AMBIGUOUS_*` groups
  (`crates/pdf-model/tests/oracle.rs`, `tests/annotations.rs:288` already describes the file).
  `issue17215.pdf` is listed nowhere in the run, which is what an agreeing page looks like.

## Gates on the final tree

The whole §2 sequence, this being a `pdf-model` change. Its lines and numbers are in the section
below, written after the run.

### The run

Every line of `doc/todo/02` §2, on this tree with the callers unswitched, four siblings editing
beside it. Exit statuses read off the run, not off a grep:

| line | exit | what it printed |
|---|---|---|
| `cargo fmt --all --check` | 1 | one diff, `crates/pdf-syntax/src/lexer.rs:1253` — a sibling's file mid-edit |
| `clippy --workspace --all-targets`, `-D warnings` | 101 | one lint, `crates/pdf-syntax/src/lexer.rs:727` (`single_match`) — the same file; `fuzz/`'s fmt and clippy both 0 |
| `nextest run --workspace --no-fail-fast` | 100 | `4370 tests run: 4368 passed (4 slow), 2 failed, 35 skipped` — `pdf-syntax::encryption a_document_with_a_password_opens_with_it_and_not_without` ("issue3371.pdf has no first page") and `pdf-model::outlines a_document_with_an_outline_produces_its_items` (174 documents state an `/Outlines`, expected 176); the four new tests in `transparency.rs` pass |
| `test --workspace --doc` | 0 | |
| corpus | 101 in the chain, **0 alone** | in the chain `63 incomplete, 1 slow` — `ContentStreamCycleType3insideType3.pdf` at 31.42 s against the 30 s wall clock, straight after the workspace test build, which is §2's own documented false positive; alone: `974 documents in 16.9s: 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 63 incomplete, 0 slow` |
| oracle | 0 | `1956 pages in 35.7s (1853 we call complete, 103 incomplete)`; `agrees 990, contradicted 62, ambiguous 835, not comparable 47, no render 17` |
| text_extraction, selection_census, accessibility_census | 0, 0, 0 | |
| launch_path (`--release`) | 0 | the counted half, no `PDFVIEWER_LAUNCH_CLOCKS` |
| dates | 101 | `1472 date strings parse, down from 1514 — the ratchet only rises` |
| xmp | 101 | `297 packets read, down from 318 — the ratchet only rises` |
| jpeg2000 | 0 | |
| render-raster corpus | 0 | refused and differing lists unchanged |
| fixed_documents | 0 | |
| transform gate, writer/split/merge/pages/optimize/foreign corpus | 0 ×7 | |
| pdf-vfs write_corpus, read_corpus | 0, 0 | |
| conformance | 0 | |

**The four red lines that are not the slow one share a cause and it is not this round's.** The
encryption test, the outline count, the date-string ratchet and the metadata-packet ratchet all
count documents the corpus opens, and all four fell by the encrypted documents in the same run in
which `pdf-syntax`'s lexer was being rewritten by the round that owns it; nothing in this change
reaches `Document::open`. They are reported rather than repaired, per
`doc/habits/tests-gates-and-reports.md`'s first habit.

`cargo run --release -p conformance --bin quotations` prints nothing about ADR 1000 or the three
rows.
