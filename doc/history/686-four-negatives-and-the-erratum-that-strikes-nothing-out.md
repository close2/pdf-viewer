# 686 — Four negatives, the halves that survived, and an erratum that strikes nothing out

Four more of `doc/todo/01`'s owed negatives re-derived over the SafeDocs crawl: **three false, one
holds** — and in two of the three a *sharper* claim survived the falsification, which is worth more
than the counts. One of the four was settled by an instrument the queue said did not exist.

Date: 2026-08-23.
ADR: [0523](../adr/0523-four-negatives-and-the-halves-that-survived.md).

Touched: `crates/pdf-model/examples/border_precedence_census.rs`,
`crates/pdf-model/examples/hollow_glyph_census.rs`,
`crates/pdf-model/examples/long_mitre_census.rs`,
`crates/pdf-model/examples/witness_census.rs` (module comment only),
`doc/conformance/ledger.toml` (§8.4.3.5, §9.7.4.2, §12.5.4, §14.8.2.5.3), `doc/errata-read.md`,
`doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## What the queue said

`doc/todo/01`'s own script, run in this worktree before the edit, printed **26 done and 20 owed**,
which is what the briefing quoted and what the merged tree holds. After the edit it prints
**30 and 16** over the same population of 46.

**Two rows nearly left the population rather than moving across it.** The first draft of the
§9.7.4.2 and §14.8.2.5.3 corrections deleted the retired sentence outright, and one of them wrote
the quotation in this project's house style — `\"[n]o corpus document …\"` — which the script's own
regular expression cannot match. The population fell to 44 with nothing to show for it. Both
corrections now state the retired claim in words a grep sees, which is `doc/habits.md`'s "a retired
claim is a string, and strings are greppable" read from the other end: **a correction has to leave
the sentence findable, or the sweep loses the row instead of counting it.**

## The four, both populations

Curated is 1251 documents (1239 open), `CC-MAIN-2021-31` is 65 944 (65 703 open), stated apart per
ADR 0490. The pdf.js column is there because three of these sentences were measured over it.

| clause | claim | pdf.js | curated | crawl |
|---|---|---|---|---|
| §9.7.4.2 | a `/CIDToGIDMap` stream over a wholly hollow program | 0 | 0 | **3** |
| §9.7.4.2, the symptom the fixture is for | a code landing on a blank glyph in one | 0 | 0 | **0** |
| §12.5.4 | Table 168's `B` or `I` on a constructed border | 0 | 0 | **170** |
| §12.5.4 | a discarded `/Border` radius on a subtype whose `/BS` is a border | 0 (6 on `/Ink`) | 0 | **3, all `/Link`** |
| §14.8.2.5.3 | a `/ReversedChars` marked-content sequence | — | **1** | **2** (a third states the name, and is not the tag) |
| §8.4.3.5 | a long mitre that is dashed or at or under a device pixel | 0 | 0 | **0 — holds** |
| §8.4.3.5 | a long mitre at all, against 116 pages stating a limit that admits one | 2 fixtures | 2 fixtures | **0 of 65 659** |

**The one that held is worth as much as the three that fell.** The last three rounds on this queue
found ten of eighteen and then seven of eight false, which is a rate at which a round stops
expecting the other answer. Not one crawled first page has a mitre join whose geometry reaches the
ratio `tiny-skia` bevels — while 116 of them state a *limit* that would admit one, over 4419
strokes. A large `M` is ordinary; a long mitre is not, and `pdf-differences`' two files are still
the whole of the world ADR 0398's construction draws differently on. That converts its
justification from "no file we have does this" into "no file in sixty-six thousand does".

## Three things worth carrying forward

**Run the census against the sentence's own population first.** Two of the three censuses reprinted
the row's old figures to the digit — 33 781 constructed borders with one `U`, one `D` and no `B` or
`I`; 42 streams in 30 documents with 214 of 221 programs partly hollow. That is the planted-witness
rule pointed backwards, and it costs one extra run: an instrument that had drifted under its row
would have printed something else, and there would have been nothing to say about the crawl until
that was explained.

**A false negative can leave a sharper claim standing, and it did twice.** §9.7.4.2's three crawled
witnesses embed a hollow program and never show it, so `codes_reaching_a_blank_glyph` is zero on
every page of all three: the structure has witnesses and ADR 0350's *symptom* still has none.
§12.5.4's six pdf.js radii sit on ink annotations, which this clause says a `/BS` is not a border
for — the crawl's three sit on links, which it says it is. Writing only "false" would have thrown
both away.

**A name census has a third column, and it is a content-stream census.** `witness_census` searches
every stream's *decoded* data, so a marked-content tag and a CMap operator are both in reach — which
is what settled §14.8.2.5.3, a row the queue had moved into the group needing "a content-stream
census, which nothing in this tree has". The same run printed its own discriminator: the one crawled
hit scored *as a name* rather than only in a stream is `/S /ReversedChars` under a `/RoleMap` to
`/Span`, a structure element type and not a tag on any page.

## The new witness immediately asked a question

`issue19971.pdf` is the first real document this tree has for §14.8.2.5.3, and its Arabic page reads
back in the **opposite direction** from `pdftotext`'s — the longest run the two share is 8 characters
against poppler's text reversed and 3 against it forwards. Every code on that page sits in a `/Span`
whose §14.9.4 `/ActualText` names one character, inside the `/ReversedChars` sequence, and this
clause's `shall` is about "the sequence of the characters as found in the show string operator",
which an `/ActualText` is not. Exactly one of the two implementations is reversing that page. The
row now carries the question; nothing was changed on a guess.

**And the old sentence had decayed twice rather than once.** It was measured over *first pages*, and
the curated witness writes its tag on page 6 — a narrowness no amount of corpus growth would have
exposed, only a re-reading of the instrument.

## Also: Issue #154, and the fourth insertion-only erratum

`spec-errata emit` on every clause this round touched found **Errata Collection 3's Issue #154 on
§8.4.3.5, recorded nowhere in this tree**. A bare Caret, `Review/Completed`, whose `/Rect` lands
between "limit" and "shall" on the line `pdftotext -bbox` puts at 441.5 of page 177, inserting
"shall be a number greater than or equal to 1.0 and". `spec-errata check` cannot see it, because it
compares quotations against *struck* text and this strikes nothing — the same blind spot ADR 0502
found at Issue #131 and ADR 0516 at Issue #536, now three instances. It vindicates the code and
replaces its argument: the clamp at 1 was justified by inferring the floor from the clause's own
ratio, and the clause states it.

## Gates

The change reaches `crates/pdf-model` (four examples), so the map asks for everything, and
`round.sh` called this a fifth round besides. The whole of `doc/todo/02` §2 was run, every line
exit 0.

- `cargo fmt --all --check`, `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`,
  `cargo nextest run --workspace` (**2452 passed, 17 skipped**, 130 s), `cargo test --workspace
  --doc`, `cargo check --manifest-path fuzz/Cargo.toml --bins`.
- Both trap-10 builds — `pdf-sandbox --bins` and `hayro-compare --bin pdfref-hayro`.
- **corpus** — 974 documents in 21.8 s, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless,
  68 incomplete, 0 slow.
- **oracle** — 1794 pages in 119 s: agrees 902 (861 on pages we call complete), contradicted 60,
  ambiguous 768, our geometry 2, reference geometry 2, not comparable 42, no render 18. **The
  not-comparable figure is the machine rather than the tree and is worth saying so**: this round ran
  beside three parallel ones on a box at load 40–130, and `doc/todo/02` §2's own warning is that a
  reference renderer loses a wall-clock budget under load and its failure reads as a regression in
  the thing being measured. 50 pages had a reference return one colour and 29 of those are left with
  fewer than two readings. Nothing in this round can move a pixel — no crate outside `examples/`
  was touched — and the gate's own ratchets passed.
- **text extraction** — 99.2% (22834/23013 words) against `pdftotext` with 22 below 90%, and 99.8%
  (14257/14281) against PDFBox's frozen extraction in both orders with 4 below 90%; the position
  gate 508 of 974 judged, 22 documents not fully in bounds. `issue19971.pdf` is on none of those
  lists — it appears only as an **ambiguous** oracle page, 5 and 6, which is a rendering verdict and
  not a reading of its `/ReversedChars` text.
- **selection census**, **accessibility census** (988 documents), **dates**, **xmp** (319 documents
  carry §14.3.2's stream), **jpeg2000**, **fixed documents** (40 checked, 0 absent) — all exit 0.
- **quorra corpus** — 957 pages compared in 45 s, 933 agree, 22 differ, 2 refused, 17 not comparable.
- `cargo test -p conformance` — **875 rows**: 443 implemented, 224 partial, 18 reported, 69
  inapplicable, 8 writer-side, 113 out-of-scope, **0 unreviewed**, 10 498 citations. **No status
  moved**: all four rows this round re-derived keep the status they had, which is the point of §3
  below.

The reference cache was **copied** into this worktree's own target directory rather than shared with
three neighbours (99.8% hit rate, 15 renders produced). §5's binaries were deliberately not
installed: this is a parallel round told not to merge, and `target/` is the main tree's.

## Sweeps

Fourteen sweeps plus `spec-errata check` and `applied`, both runs with `cargo run` from this
worktree rather than from a build directory (trap 15). **Seven moved and every delta is this round's
own prose.**

- `counts` 7005 → 7024 sentences and 381 → 382 attributed, the one new attribution being under a
  clause with no rows below it, which is that sweep's own documented noise shape.
- `capabilities` (source) 182 → 183 sentences and 110 → 111 about the program.
- `owed` 3591 → 3592 terms, with its 182 unnamed over 114 rows **unchanged** — no phantom key this
  round, because the citations added (`examples/hollow_glyph_census`, `examples/witness_census`) have
  leading segments that are ordinary words the sources themselves name (ADR 0493's shape).
- `pointers` 7333 → 7346 paths with **absent unmoved at 123**. Two categories moved inside the
  non-failing ones and both are this round's: "a form" 170 → 167, because three module comments
  stopped naming `doc/pdf.js/test/pdfs/*.pdf` as an invocation and now say `--pdfjs`; "not carried"
  392 → 393, because `hollow_glyph_census` now names `corpus-cache/safedocs/cc-main-2021-31`, which
  is machine-local and untracked by design.
- `quotations` 5505 → 5521 over the documents in 838 → 840 (the ADR and this file) with verbatim
  2385 → 2387 and **diverging unmoved at 34**; in the ledger 1838 → 1841 with verbatim 1407 → 1408
  and **diverging unmoved at 2**. The two new unrelated ledger quotations are Issue #154's added
  words, which `doc/md/` does not carry because they are an insertion — which is the whole finding.
- `tables` 5961 → 5966 sentences with citations, absent and the denial count all unmoved.
- `spec-errata applied` 50 898 → 50 984 places with 540 → 545 naming an erratum, and 1531 → 1535
  comparisons with its 90 / 10 / 171 split unchanged.

`blockers`, `callers`, `entries`, `inapplicable`, `ledger`, `overstated`, `retired`, `unread` and
`spec-errata check` did not move at all.

Run a third time on the finished tree (ADR 0485), `pointers`, `quotations` and `counts` print
**exactly** the second run's figures — and that is explained rather than suspicious: this file is the
only thing edited between the two, and `pointers::a_rounds_own_record_is_not_swept` says in as many
words that `doc/history/` is read by nothing there, "however many dead pointers it carries".
