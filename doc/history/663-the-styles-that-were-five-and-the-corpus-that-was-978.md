# 663 — The styles that were five, and the corpus that was 978

Two defects, one shape: a cardinal whose denominator its sentence never named. One of them was
being shown to a person — `requirements::unmet` told a host that *five* of Table 164's transition
styles are reported by name, where four are and the fifth is the cut the table defines. The other
was a negative measured over 978 documents and written about the world; the crawl states 276 of the
thing it says nothing states, and hands four clauses their first real witness.

Date: 2026-08-22.
ADR: [0490](../adr/0490-the-denominator-a-sentence-does-not-name.md).

Touched: `crates/pdf-model/src/requirements.rs`, `crates/pdf-model/src/navigation.rs`,
`crates/pdf-model/examples/presentation_census.rs`,
`crates/pdf-model/examples/presentation_fixture.rs`, `crates/viewer-host/src/clock.rs`,
`doc/conformance/ledger.toml` (§8.6.5.7, §12.4.4, §12.4.4.1, §12.4.4.2, §12.6.4.11, §12.6.4.13,
§12.6.4.15, §12.11.2), `doc/adr/0462`, `doc/todo/01-ledger-partial-rows.md`, `doc/todo/32-presentation-player.md`,
the ADR and this file.

## The order the instruments gave

**`--bin overstated` first**, as `doc/todo/01` says: a fifth of a second, **8 contradictions over
170 parent rows asserting 127 terms, 7 of them marked**. Unchanged from 657, verdict for verdict.
Its one unmarked hit is §12.7's `/AP` against §12.7.5.5's "Table 236's `/P` is deliberately not
read here", read this round rather than inherited — and it is noise, on a reading that goes one
step further than 657's. The child does not merely deny a different entry: its **opening sentence
asserts the same one the parent does**, "[a] signature field is drawn from its `/AP` and from
nothing else". A row that corroborates the parent in its first line cannot be contradicting it in
its last; the sweep's own loosest rung, "the child's denial names another one of its kind", is
where it belongs.

**Then the blame ordering**, re-derived (616's rule): 869 commits, **242** `partial`-or-`reported`
rows with a blamed note. 657's five have moved off the top and its prediction holds — §10.7.5 at 1,
§8.6.5.7 at 2, §7.6.4 and §7.6.4.4 at 3–4, §11.5.3 at 5, §11.3.4 at 6, then nine sharing 7–15 on
`cb73428b`.

**Then step 7, which is new in this batch and which took the round somewhere else**: a negative
claim decays when the population grows, and nobody had swept the ledger for negatives measured
before the crawl. There are about sixty sentences of the form *no corpus document does X*. The one
read here is the loudest, because a whole clause family rests on it.

620's rule still chose one row: **§8.6.5.7, rank 2, which 657 passed over as a reading of the
standard.** It is that, and its *first sentence* is a claim about this codebase — and false.

## The rows

**§12.4.4.1 — the negative, re-derived rather than inherited.** The row says demand was measured in
the three-hundred-and-ninety-third over 978 documents and "not one states a `/Trans`, a `/Dur` or a
`/PresSteps`", so the witness is hand-built. Both halves of the re-derivation matter:

- the **control**, same instrument, curated corpora, now 1133 documents: still **0 / 0 / 0**. The
  old sentence was never wrong about its own population, which is why nothing could see it.
- the **crawl**, all 65 944 of `CC-MAIN-2021-31`, 65 703 of which open: **276 state a `/Trans`, 86 a
  `/Dur`, 1 a `/PresSteps`** — lower bounds, because `presentation_census` walks a document's first
  hundred pages.

Under a minute, `find | xargs -P 8 -n 200`, and the census's own module comment now carries that
command beside the pdf.js one.

**§12.4.4 — the debt is ranked for the first time.** This row is `partial` for the styles no frame
is shaped for, on the clause stating no quantity for them. Over the crawl, `Dissolve` is asked for
on **221 pages of 11 documents** and `Blinds` on **16 of 4**; **`Glitter` and `Fly` by nothing at
all**, against `Fade` on 596 pages, `Wipe` on 258 and `Push` on 162, all drawn. So the refusal is
one style wide in practice and two of the four are sentences nobody has ever read. Nothing is
implemented: the clause still states no quantity, and 620's rule leaves a refusal resting on the
standard where it is.

**§12.4.4.2 — a witness that is not a fixture, for the first time in the clause's life.** The row
said `PresSteps` and `NavNode` occur in no corpus document's bytes and on no page of the 978.
`cc-main-2021-31/7680/7680405.pdf` states `/PresSteps` on four of its 39 pages; Table 165's nodes
carry `/NA`, `/Next`, `/PA` and `/Prev`, and their actions are §12.6.4.13's `/SetOCGState` over
optional content groups — a slide build, which is exactly what `examples/presentation_fixture` was
hand-written to stand in for (trap 8). §12.6.4.13's row gains the same file: **36 dictionaries, all
performed.**

**§12.6.4.15 — the same file again, and the case the row never mentioned.**
`examples/refused_action_census` over the whole crawl finds **14 `/S /Trans` action dictionaries in
one document**, all performed, and every one states `/S /Blend` — a name Table 164 does not define.
So the thirteenth case this reader keeps on purpose, `Style::Unrecognised`, had a unit test and now
has a producer.

**§12.6.4.11 — the negative with the most behind it, because its own row says what it costs.** "No
corpus document states one: the sixty-second session walked every object of all 964 openable
documents and found no `/Hide` at all", three sentences after "**This one changes what is drawn**".
The control still holds — **zero over the curated 1133** — and the crawl states **2165 `/S /Hide`
actions over 8 documents**, every one performed: `0546966`, `1530266`, `2883425`, `4100004`,
`4359052`, `4359983`, `5343096` and `6942521`, the last of which states 1900 by itself. Nothing in
the code changed; what changed is that the clause has files behind it, and `doc/todo/03` is where a
chunk of them belongs.

**And §12.6.4.13's positive count was stale in the *curated* direction too**, which is the same
lesson with the sign reversed: the row said one corpus document states a `/SetOCGState`, measured
over 964, and the curated population is 1133 now and **two** of them do — the second arrived with a
submodule and no round re-counted. A population claim decays when the population grows whichever way
the claim points.

**§8.6.5.7 — the ledger's own sixth failure shape, in the row the blame ordering puts second.** The
note opened "[t]his device is an sRGB screen and every colour is converted to it at the moment it
is read, so there is no four-to-three-to-four round trip for the clause to save **and no place the
shortcut would apply**" — and three sentences later, since the four-hundred-and-thirty-sixth
session: "[t]he implicit conversion this clause describes is performed on a page that composites in
a press." Both cannot be true. The second is: `colour.rs`'s `Space::Icc` arm passes four components
through untouched where `press.identity == PressIdentity::Profile(profile.identity())`. A note
corrected by appending and never re-read whole, for 227 sessions.

## The defect that was being shown to a person

Table 164 has twelve styles. `viewer_core::transition` partitions them **twice** and the two
partitions differ by one: `frame` shapes seven and shapes nothing for five; `note` reports **four**.
`R` is in the second set and not the third, because the table defines it as the cut — so a cut is
what the file asked for and there is nothing to report.

The five-hundred-and-fifty-third session found "five are reported" and corrected it in §12.6.4.15's
row and in §12.4's, recording both in `doc/todo/01`'s band table. **Eight other homes went on saying
it.** §12.4.4.1's row, §12.11.2's row, `pdf_model::navigation`'s module comment,
`viewer_host::Clock::shapes`'s doc comment, that crate's own test's doc comment, ADR 0462,
`doc/todo/32`'s **status line** — which is the file a round working on this clause opens, so the
lesson landed everywhere except where it is used — and `requirements::unmet`'s `Kind::Transitions`
arm, which is **not a note but a sentence this program hands a host to show a person**. A file
asserting Table 275's `Transitions` requirement was told a false number by the reader that could not meet it. All eight now say four, and the arm cites the
function the number comes from.

Why it repeated so easily is the ADR's subject: **both numbers are true of something.** A round
writing "five" was counting the unshaped and describing the reported. One place had it right all
along and nothing could hear it: `viewer-core/tests/headless.rs`'s
`a_transition_this_reader_does_not_draw_is_named_rather_than_cut` says `Blinds` is "one of the four
left unshaped" — a test's doc comment disagreeing with eight other places for a hundred and ten
sessions.

**Step 5, checked and passed.** That test does reach its claim: it opens two pages, ticks the clock
past the first page's `/Dur`, and asserts both halves — an `Event::Reported` whose note names
`/Blinds` and mentions the page moved to, *and* that an `Event::Transition` is still emitted,
because a host that can draw it is not this one. Written into §12.4.4.1's row.

**Step 3, enumeration, bounded rather than paid.** Every caller of `navigation::transition`,
`display_duration` and `steps`: `action.rs`'s `/S /Trans`, `viewer_core::viewer`'s clock and
`presentation`, all three hosts' `arm_transition`, `viewer-ffi`'s `pdfv_event_transition` and
`viewer-confined`'s protocol codec. Nothing is unwired; the fifth sweep's shape is not here.

**Step 6, a price re-derived: none was cited by these rows.** §12.4.4's `partial` rests on a silence
in the standard rather than on an estimate, which is a reason and not a price. What replaces the
re-derivation is the ranking above — the four refusals now have populations, which is the same
question asked with the corpus instead of the clock.

## `spec-errata emit` before writing

Over all fourteen documents. **Nothing new under §12.4.4, §12.4.4.2, §12.6.4.13, §12.6.4.15 or
§8.6.5.7.** §12.4.4.1 carries issues #36 (`/Di` *number* → *integer*) and #75 (*upper case* →
*uppercase*), both already recorded in that row. §12.4.4.2's own errata are still filed under
`## 12.5.1`, which is 657's lesson about `emit` filing by the page a heading opens, and issue #304
is already in the row. §8.6.5.7 gets no heading at all — `emit` goes 8.6.5.5, 8.6.5.8 — so nothing
in that family's neighbourhood touches the implicit conversion.

§12.11.2 carries issues #187 and #656 renaming Table 274 and one of its columns; that row is
`implemented` and this round changed only its cardinal, so they are named here rather than acted on.

## The instrument, and one new noise shape

Twelve sweeps before the edit and after it, ADR 0485's habit, ledger-only for the reading. The
deltas are in ADR 0490 §5 and every one has a sentence. **`overstated` 8 contradictions with 7
marked, `counts` 4 places counting one family twice, `quotations` 1 diverging, `tables` 6 denials
the table contradicts and 97 absent, `unread` 68 rows / 181 keys, `entries` 182 reported over 49
rows, `pointers` 118 absent and 13 undefined, and `blockers`, `capabilities`, `inapplicable` and
`callers` at their standing populations — all unchanged**, which is the point of running it twice.

`counts` went 369 attributed counts to 371, `quotations` 1754 to 1757, `tables` 2175 key citations to
2187 with 2031 agreed, `entries` 282 rows to 283 and `pointers` 6971 paths to 6998 — every one of
those a sentence this round added, and every *hit* count beside them the same as before.

The one that moved and could not be waved past: **`owed` went 179 unnamed terms over 113 rows to 180
over 114, and the row that left its reading list is §12.6.4.15's** — because the term it gained is
`Blend`, a style name out of somebody's file, which this tree deliberately names nowhere. That is a
noise shape the fourteenth sweep did not have, and the obvious repair is the wrong one: a witness is
not a debt, and deleting the evidence to hold a level flat would be the instrument choosing what the
ledger may say.

## Gates

The change reaches `pdf-model`, so the map asks for everything and **the whole of `doc/todo/02`
§2 was run**, not a subset.

- `cargo fmt --all --check` — exit 0.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — exit 0.
- `cargo nextest run --workspace` — **2405 passed, 17 skipped**, 230 s.
- `cargo test --workspace --doc` — exit 0, every result line `ok`.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — exit 0.
- `cargo build --profile gates -p pdf-sandbox --bins` and `-p hayro-compare --bin pdfref-hayro` —
  both exit 0 (trap 10).
- **corpus** — exit 0.
- **oracle** — exit 0: agrees 908 (863 on pages called complete), contradicted 65, ambiguous 786,
  our geometry 2, reference geometry 2, not comparable 13, no render 18.
- **text extraction** — exit 0: 99.8% (14257/14281 words) against PDFBox in both orders, 4 below
  90%; the position gate 10969/11163 in bounds (98.26%), 486 of 508 documents fully in bounds.
- **selection census** — exit 0: 1000/1011 words selected (98.91%) over 453 documents.
- **accessibility census** — exit 0: 102 853 elements reached, 57 116 a caret can move through.
- **dates** — exit 0: 1514 of 1545 strings conform (97.99%).
- **xmp** — exit 0: 318 of 319 packets read, 3191 properties.
- **jpeg2000** — exit 0.
- **quorra corpus** — exit 0, every page agreeing with the CPU oracle; median page 3.48×.
- **fixed documents** — exit 0: 40 checked, 0 absent.
- `cargo test -p conformance` — exit 0. **875 rows**, breakdown unchanged at 436 implemented, 224
  partial, 18 reported, 76 inapplicable, 8 writer-side, 113 out-of-scope. **No `silent` row**, 0
  unreviewed. No status moved, which is right: every defect here is a sentence about work already
  done or a population that had grown.

**§5's binaries were deliberately not installed.** This is a parallel round told not to push or
merge, and `target/` is the *main* tree's — putting an unmerged branch's binaries where a person
runs them, with three other rounds building beside it, is what §5 exists to prevent rather than to
require. The merge round owns it.

The reference cache was **copied** rather than shared: `PDFREF_CACHE` points at this worktree's own
copy of the 2.2 GB `pdfref-cache`, so the oracle's 908 agreements are not a read of a directory
three neighbours are writing.

## Overlap with the parallel rounds

660, 661 and 662 ran beside this one, each briefed to touch a row or two. Nothing written here is
outside §8.6.5.7, §12.4.4, §12.4.4.1, §12.4.4.2, §12.6.4.13, §12.6.4.15 and §12.11.2, and no other
row was reflowed.
