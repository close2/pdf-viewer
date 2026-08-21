# 648 — The refusal three rows had already been taught to prove

The three rows 641 left, plus one from below the gap. Four read properly, two defects, two
confirmations, and one count corrected that was not in the ledger at all. The round's own
contribution is an instrument and the correction of its own first draft.

Date: 2026-08-22.
ADR: [0478](../adr/0478-the-refusal-three-rows-had-already-been-taught-to-prove.md).

Touched: `crates/pdf-model/examples/refused_action_census.rs` (new),
`crates/pdf-model/examples/signature_algorithm_census.rs`, `crates/pdf-model/src/action.rs`,
`crates/pdf-model/src/der.rs`, `crates/viewer-core/tests/headless.rs`,
`doc/conformance/ledger.toml` (§7.6.3.3, §7.6.4.1, §12.6.4.3, §12.7.6.2, §12.8.2.2, §12.8.3.4.2),
`doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## How the band was ordered

Re-derived rather than taken, which is 616's lesson: `git blame --line-porcelain
doc/conformance/ledger.toml`, each `partial` or `reported` row's own `note = ` line, ranked by where
its commit falls in `git log --reverse`. This base has **843 commits** and 240 `partial`-or-
`reported` rows with a blamed note.

641 named eleven rows at ranks 513–534 and read four. **Three were left, and the ranks had moved
under them**: §12.7.6.2 at 513, §12.8.2.2 and §12.8.3.4.2 both at 517, and the gap above is now
**fifty-nine** commits rather than forty-two — §10.4.2.4 and §10.4.2.5 begin at 576. That is 5's
warning paying: the numbers in a document are a property of the base, not of the ledger.

All three were read, and a fourth from just below the gap, §7.6.4.1 at rank 578. 620's rule chose
the work for the sixth time — all four state a reason that is a claim about this codebase rather
than about the standard, and three of the four end in a number over the corpus.

## §12.7.6.2 and §12.6.4.3: the defect an earlier round had already fixed for three siblings

620's third shape — *the row is right and its evidence is not* — for the seventh round running, and
the sharpest instance yet because the fix was already in the tree.

§12.7.6.2 is `reported`, a status whose whole content is "a person is told which action declined
and why", and it cited `action.rs::a_name_the_table_does_not_hold_is_not_an_action` — which asserts
that `/Teleport` yields *no* action and so is the one path that never calls `action::refused`.

**626 found that exact citation false for §12.6.4.6, §12.6.4.9 and §12.6.4.10** and wrote
`headless.rs::a_click_on_an_action_this_program_will_not_perform_says_which_and_why`, which drives a
real click through `interact::perform` to `Event::Reported`. It did not ask which other rows rested
on the same function.

**Enumerating `action::refused`'s ten arms against the ledger found the second survivor.** Five of
the ten types carry a `reported` row and four carry an `out-of-scope` one, which owes nothing.
Three of the five were covered; §12.7.6.2 and **§12.6.4.3** were not, and both kept citing the dead
test, twenty-two rounds on. §12.6.4.3 could not have been found by the ordering — its note has been
rewritten since and ranks nowhere near the top.

Both rows now cite the click test, which gains `GoToR` and `SubmitForm` with their tables' required
entries so that the refusal is the clause's rather than a malformed dictionary's, and both `code`
arrays name the whole path — §12.6.4.3's had pointed at `pdf-viewer.rs` where `dispatch.rs` is what
prints. Mutation-checked: with both arms made to return `None` the test fails naming the action.

## The instrument, and the bound that was a finding

`examples/refused_action_census` counts Table 201's twenty types over a population and prints what
`action::read` answered for each — the standard's table on one side, the reader's own verdict on
the other, so neither side is a copy of the other.

**Its first draft walked only the objects the cross-reference table lists**, which is
`structure_destination_census`'s bound and its argument, and it reported **zero** `/S /GoToR` and
**zero** `/S /SubmitForm` — which would have said neither of the rows being fixed describes anything
a reader meets. A `grep -l` over the corpus's raw bytes contradicted it in one command: two files
hold `/GoToR` and one holds `/SubmitForm`, all three written *directly* inside their annotation or
outline item, where there is no object number to be found by. The census now walks each numbered
object's body as well, never through a `Reference`.

The same bound was behind a wrong count in 626's own comment: "of the 974 corpus documents exactly
one states a `/S /Launch` action" is **two** — `externalLink.pdf` writes its action inline.

Over the 1249 documents of `doc/pdf.js` and `doc/corpora`, 1237 opening: `/S /GoToR` 2 in 2,
`/S /SubmitForm` 1 in 1, `/S /Launch` 2 in 2, `/S /Sound` and `/S /Movie` **0** — so the two
absences 626 recorded hold and the presence it recorded did not. The largest refused population by
far is `/S /JavaScript`, 253 dictionaries in 60 documents, whose row is `out-of-scope` and asserts
nothing.

## §12.8.2.2 and §12.8.3.4.2: two counts, both right, one denominator reconciled

Both end in a number over the corpus and neither had a command, which is 641's rule. Two counters in
`signature_algorithm_census`, on the walk that already opens every document and reads every
`SignedData`, plus one small public accessor.

§12.8.2.2's "the corpus's one certification signature states `/P 2`" **holds**: exactly one
`/Perms /DocMDP`, in `xfa_filled_imm1344e.pdf`, at level 2 — a file the row had never named.
Counted from the permissions dictionary rather than from a signature, because §12.8.6's
`/Perms /DocMDP` is what makes the transform binding.

§12.8.3.4.2's "four corpus documents write [indefinite lengths]" **holds**, and so does `der.rs`'s
differently-denominated "four of the ten signature values" — they coincide because each of
`160F-2019.pdf`, `issue16553.pdf`, `prefilled_f1040.pdf` and `xfa_filled_imm1344e.pdf` holds exactly
one such value. `der::Value::had_indefinite_length` is what made the tolerance measurable at all;
the census walks the whole encoding rather than its outermost value, because Adobe's `30 80` is an
observed shape rather than the only legal one.

## §7.6.4.1: the count whose command already existed

"Eight corpus documents reach it" **holds** — the corpus gate prints every locked file and
`MAX_LOCKED` ratchets the total. The row simply never said so, for the four hundred and sixteen
sessions the sentence has stood. **A note whose count already has a command owes the command's
name, not a new census**, and this round nearly wrote a fourth instrument before finding the gate
that had been printing the number all along. In `doc/todo/01` with the walk rule.

## Two findings beside the rows

**`action::refused`'s doc comment said "Table 201's other seventeen types."** No reading of the
table produces seventeen: ISO 32000-2's Table 201 lists **twenty**, `one` performs eleven, nine are
left with no arm, and the function names ten because `Thread` sits on both sides. Counted from
`doc/md/` and corrected with the arithmetic written down.

**`spec-errata emit` filed an erratum two clauses from where it belongs.** Issue #469 — `shall be`
struck, `is` inserted — printed under `## 7.6.4.1 General`, which is a row this round was reading.
The strikeout's `/Rect` against page 91's text boxes puts it on "[t]he number of bytes to be
encrypted or decrypted **shall be** given by the Length entry in the stream dictionary", which is
**§7.6.3.3's**. `emit` files a note under the clause the outline puts its *page* in, and page 91
opens inside §7.6.3.3 before beginning §7.6.4.1. Recorded in §7.6.3.3's row, whose note already
quotes the sentence beside it; the change costs this reader nothing, since `/Length` is how much
stream there is whether or not a `shall` says so. The rest of the families' errata: §12.7.6.2 has
Issue #122, striking "inheritable" from Table 239's `/Flags` and `/CharSet`, which changes nothing
while the action is declined whole; nothing at all touches §12.8.2.2 or §12.8.3.4.2.

## Gates

`pdf-model` is the change→gate map's first row and `tools/round.sh` called this a fifth round, so
the whole sequence ran; every line exit 0.

`fmt --all --check` clean. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0
— it caught three `arithmetic_side_effects` in the new census first, fixed with a saturating helper
rather than allowed. `cargo nextest run --workspace` **2364 passed / 17 skipped**. Doctests clean.
`RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` exit 0.

Corpus **974 documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 68 incomplete, 0
slow** — 68 unchanged, which is the number that says trap 11 was not sprung. Oracle **1794 pages:
907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2 reference geometry, 13 not comparable,
18 no render** — identical to 641's, verdict for verdict, which is what a round that moves no pixel
should print. Text extraction 10969/11163 matched words in bounds (98.26%) over 508 documents;
PDFBox lane 99.8% (14257/14281); pdftotext lane 99.2% (22834/23013). `selection_census` 1000/1011
words (98.91%) over 453 documents; `accessibility_census` 104 documents with structure, 90 tagged,
0 pages answering with structure they do not state; `dates` 1545 strings, 1514 conforming; `xmp` 319
documents, 318 read; `jpeg2000` green. `render-quorra` 957 pages: 932 agree, 23 differ, 2 refused,
17 not comparable. `fixed_documents` 33 checked, 0 absent. `cargo test -p conformance` green —
**875 rows, 0 unreviewed**, and the status breakdown is unchanged at 436 implemented, 222 partial,
18 reported, 78 inapplicable, 8 writer-side, 113 out-of-scope. **No `silent` row.** No status moved,
which is the right outcome: every row read here keeps what it claimed and gains evidence that
reaches the claim.

Sweeps run because the ledger moved. `quotations` — 1715 ledger quotations, 1 diverging, and that
one is §8.9.5's and was there before; `pointers`, `counts`, `tables`, `entries`, `unread`,
`blockers`, `capabilities`, `callers`, `inapplicable` and `owed` printed their standing false
positives and no new hit. §5's binaries rebuilt and installed.

**Overlap with the parallel rounds: none.** 645's new parent-versus-children sweep named §7.3.8,
§9.9, §12.8.2, §14.9.2, §12.11 and §12.7; the two near misses are §12.8.2 and §12.7, whose
*children* §12.8.2.2 and §12.7.6.2 are this round's. Different rows, different notes, no shared
line.
