# 901 — Two merges, and a ranking head that is a clause working: `batch5/ocrmypdf` walked, and §10.7.5's second requirement gets a page

Date: 2026-09-03.
ADR: [0844](../adr/0844-a-ranking-head-that-is-the-clause-working-and-a-witness-for-stroke-adjustment.md).
0845 was allocated to this round and not needed: one decision, one ADR.
Touched: `doc/conformance/ledger.toml` (§7.6.4.2's merge resolution, then §9.3.6 and §10.7.5),
`doc/checks/fixed-documents.toml` (one row), `doc/todo/03-more-corpora.md` (§46),
`doc/oracle-and-corpus.md` (§3d's third rule), one ADR, this file; and the two merge commits
before this round's own. **No code changed**, and §46 says why.

## The merges

**`round-892` (5ffa8a6b) is on `main` as `119736db`**, `--no-ff`, on top of round 898. It fetched
the Adobe Supplement to ISO 32000-1, ExtensionLevel 3, from the Internet Archive — the document ADR
0820 had recorded as unobtainable — and settled that revision 5 *does* require SASLprep, so the code
was right and both pdf.js and PDFBox are narrower than the document they implement; it verified the
owner-password branch against a `/R` 5 file qpdf publishes with its password; and it fixed a real
defect where a stream whose `/Crypt` names a filter the document's `/CF` never defines was read as
`Identity`, leaving every such stream silently encrypted.

**One textual conflict**, in `doc/conformance/ledger.toml`'s §7.6.4.2 row, and it is two sessions
writing into one note rather than a disagreement. `main`'s round 886 had appended the bit-11
sentence (ADR 0818) to the end of the note; 892 amended a clause in its *middle* — "ADR 0820 marks
which steps rest on a normative sentence and which on evidence, **including the one this tree
cannot check against a copy of the supplement**" — because the supplement has now been checked, and
put two sentences of its own there. Resolved with 892's amended middle and main's bit-11 sentence
kept whole at the end, which is the order the note's sessions are in. The row's other three ledger
edits (§7.6.4.3.3, §7.6.5, §7.6.6) auto-merged and `main` wrote none of them;
`crates/pdf-syntax/src/crypt.rs` merged with no conflict at all.

**`round-896` (71aeb0d8) is on `main` as `d2d3846a`**, `--no-ff`, on top of that. §7.4.4.1 names two
RFCs and this tree had one error for failing either, so a deflate stream whole under a disagreeing
Adler-32 was called a truncation; `Damage::CheckValue` tells the two apart, and the decision that
goes with it is that the new class is **not** admitted for font programs, with the cost written
down. Its tracker half is `batch5/DSS`, 243 documents in 0.5 s, nothing owed.

**Git found no textual conflict in any of the three files where one was expected**, which is worth
recording rather than glossing. `doc/checks/fixed-documents.toml` is an append and `main` added no
row after the branch left it — 67 rows on main, 68 on the branch, 68 merged, sessions still in
order. `doc/conformance/ledger.toml` auto-merged although both sides wrote to it, and
`git diff main HEAD -- doc/conformance/ledger.toml | grep '^-'` prints nothing, which is the check
that main lost no line rather than a hope about three-way merging. `doc/todo/03-more-corpora.md`
took the branch's §45 whole.

**The whole `doc/todo/02` §2 sequence then ran on the merged `main`, all twenty-five lines, and
every one is green.** Highlights, verbatim from the run: `Summary [69.216s] 3140 tests run: 3140
passed (1 slow), 26 skipped`; the quorra gate `958 pages compared in 29.7s: 929 agree, 22 differ, 7
refused, 16 not comparable`; `fixed-documents: 68 checked, 0 absent, 68 rows`; the oracle's
`our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; the four transform
walks and the foreign readback at 58.78 s, 59.34 s, 89.87 s and 88.89 s. `tools/worktree.sh close
892 896` then took both checkouts and their build directories away, both branches having been
verified ancestors of `main` first. `r867` and `r899` are neighbours' and were left alone.

## The chunk

`batch5/ocrmypdf`, 205 documents and the largest unwalked tracker, surveyed whole under the four
rules of 2026-09-02 — twelve rayon threads, `--data 8 --tree 12`, **2.9 s and a 0.85 GiB peak**.
`doc/todo/03` §46 has the line, the four unusable files (none of them a PDF at all), the four
incomplete and their reports. **1.95% incomplete** is the second-lowest rate of any tracker, and
all four reports name populations this project has already argued and priced — §9.9's
closed-by-decision case twice, §7.4.8's contradicted frame, an unclosed `BT`.

The four incomplete pages rank within 0.4 of a reference, so the round ranked **all 201 openable
documents** rather than the four — which is what §44's and §45's rounds did and is where both their
findings came from. That has one head and it is **dark**: `ocrmypdf-99-0.zip-0.pdf`, ours 9.9448
against `poppler` 8.6045 and `mupdf` 7.4452, where the next row in either direction is 0.28. The
page reports nothing.

## The finding, which is that there is no defect

**It is §10.7.5 working.** ADR 0844 has the three measurements in the order they were run and the
third is the one that settles it.

The resolution ladder converges from a 2.50-level spread at 72 dpi to **0.02 at 576** — ours
7.2727, `poppler` 7.2921, `mupdf` 7.2762 — so three renderers are drawing the same shapes and
nothing here is geometry. Replacing `2 Tr` with `0 Tr` in a `qpdf --qdf` copy puts all three within
0.04, so the whole disagreement is the stroke half of §9.3.6's rendering mode 2. And the page's
`/ExtGState` states `/SA true`: renaming it to `/S1` in place takes ours to 7.2840, the converged
value, and leaves **both references byte-identical**, which is ADR 0688's finding confirmed a second
time, on a different document and a different measure — neither poppler nor mupdf reads Table 57's
entry at all.

The content stream is 108 runs of one embedded `SimSun` subset at `2 Tr` under `0.240226 w`,
`0.3203 w` and `0.426 w`, with an outer `0.24 0 0 0.24 0 0 cm` and a `4.16667 0 0 4.16667 0 0 cm`
inside every `q`, so the CTM at each stroke is unity and those widths *are* the device widths. Under
half a pixel at 72 dpi, all of them; which is why §10.7.5's "the stroke shall be rendered as a
single-pixel line" fires on every glyph, and why the ladder's disagreement survives 144 dpi and dies
at 288, where 0.961 crosses the threshold.

So the head is this tree obeying a `shall` two references miss, and it is held. What was recorded
instead of a change is three things, because a decision with no artefact is a memory: a
`doc/checks/fixed-documents.toml` row pinning the page at 8.945 .. 10.945 with `reports = []` —
**the first thing in this tree that fails if the promotion is withdrawn on a document somebody
wrote**, where until now the requirement was held by a fixture and by an oracle page whose
promotion costs no ink at all; the §10.7.5 and §9.3.6 ledger rows, neither of which changes status;
and the instrument rule below.

## The instrument rule, which is the transferable half

`doc/oracle-and-corpus.md` §3d's ink ranking is what every corpus chunk since `doc/todo/03` §8 has
used to choose the document it opens. Round 876 recorded two ways to be wrong with it —
`pdftoppm` without `-cropbox`, and averaging our alpha channel in — and both are operating errors
that make *every* page wrong at once, so they announce themselves.

This is a third and it is not an operating error. **At 72 dpi the ranking can put this tree at the
head of a directory for obeying a clause neither reference reads**, and it makes one page wrong in
a way that looks exactly like a finding: the page is boldly, visibly different beside the
references over every glyph, so trap 1's "look at the page" *confirms* the false reading instead of
breaking it. Two instruments break it and both are cheap — the ladder, four renders, and one grep
for `/SA true` over `qpdf --qdf` output followed by renaming the entry and re-measuring. The rule
is in §3d beside the other two.

**The condition is not the entry on its own**, and that was measured rather than assumed: 13 of the
201 documents state `/SA true` and exactly one is displaced by it, the other twelve spread from
+0.17 to −33.59 of the darkest reference. What displaces a page is `/SA true` **and** a stroke the
CTM puts under half a device pixel.

Three other rows of the ranking were laddered and all three converge by 288 dpi
(`ocrmypdf-144-0.pdf`, `ocrmypdf-605-1.pdf`, `ocrmypdf-LINK-490-0.pdf`), so this tracker holds no
defect of ours. A tracker that gives a round nothing is a result and §45 recorded one the same way;
what this one gives instead is a witness for a clause that had none and a rule for the instrument
seven chunks have used.

## Gates

The merge is a round of its own, so the whole `doc/todo/02` §2 sequence ran on the merged `main`
before this round's work — all twenty-five lines, green, figures above. **After the round's own
work only the lines it can reach were re-run**, and what makes that sufficient is that the sequence
had just run whole on the same tree: the round changed **no Rust at all**, so the crate graph the
map is a claim about did not move. What it touched is `doc/checks/fixed-documents.toml`, which is
`--test fixed_documents`'s own input and nothing else's, and four documents, which
`cargo test -p conformance` reads for citations, quotations and pointers. Both ran, with the six
core lines in front of them, which the map requires whatever a round touched.

## What is left

`batch5`'s other eighteen trackers are a long tail — the seven walked are its seven biggest and
nothing left is above two hundred documents, `cairo` (166), `pdfminer.six` (123) and `qpdf` (111)
the largest. §10.7.5's *first* requirement, the grid-fitting of a stroke's coordinates, is still
not implemented and is what keeps that row `partial`; this round found nothing bearing on it, and
ADR 0688's measurement of what it would buy is unchanged.
