# 701 — The fifth tag, the clause number a sibling had right, and the report that never existed

§14.6's three `partial` rows read as a family, on ADR 0538's method for the third block running.
All three are `partial` and two of them state the same list — which marked-content tags this tree
acts on by name — and a claim held in duplicate is a claim with somewhere to disagree with itself.
It did, four times. One status moved, one test arrived, one erratum that changes a requirement was
recorded, and the instrument the six-hundred-and-ninety-seventh asked for was measured and declined.

Date: 2026-08-24.
ADR: [0560](../adr/0560-the-fifth-tag-and-the-report-that-never-existed.md).

Touched: `doc/conformance/ledger.toml` (§14.6, §14.6.1, §14.6.2),
`crates/pdf-model/tests/optional_content.rs` (one test, one fixture builder, one extracted helper),
`crates/pdf-model/src/content/report.rs` (`MissingResource`'s doc comment),
`doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## Why §14.6

The blame ordering was re-derived on this base rather than read out of `doc/todo/01` (616's rule):
`git blame --line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own
`note =` line, ranked by where its commit falls in `git log --reverse`. 943 commits, 242 such rows.
§7.6.4.4 is rank 1 — ADR 0538's family, which 691 did not finish — §11.3.4 is 2, seven rows share
3–9, and §14.6, §14.6.1 and §7.7 are 10–12.

§11.3's rows were read for shape first and left alone: they cross-refer heavily and consistently,
and §11.3.7's account of what keeps its two children `partial` agrees with both children's own.
§14.6's three were taken for the duplicated list.

## The four findings

- **§14.6 wrote §8.11.3.3 twice for §8.11.3.2's mechanism**, and §14.6.1's row one line below had
  the number right all along. §8.11.3.2 is *Optional content in content streams* — the `OC` tag on a
  `BDC`; §8.11.3.3 is *Optional content in XObjects and annotations* — the `/OC` entry, a different
  mechanism with an `implemented` row of its own.
- **"Four tags are read by name" is five, and the fifth is named by the same note.** §12.7.4.3
  requires a processor to replace an appearance stream "from / Tx BMC to the matching EMC", and
  `appearance::find_tx_marked_content` matches that tag by name — which §14.6's note says one
  sentence before it says four. The sixth failure shape, in the row that records the sixth failure
  shape: 697's rule that a corrected row is not a safe row, met again.
- **§14.6's reason for `partial` denied two whole clause families.** It read that §14.7's and
  §14.8's semantics are unimplemented; the ledger's own rows under both deny it. What keeps §14.6
  `partial` is §14.6.2's debt and nothing else.
- **§14.6.2 said twice inside one sentence that an undefined `/Properties` name is reported, and no
  such report has ever existed** — `note_missing_resource` fires for `/Pattern`, `/ExtGState` and
  `/XObject` and for nothing else, in any commit. **The code is right**: §8.11.3.2 makes a section
  optional content "only if the tag is OC and the dictionary operand is a valid optional content
  group", so a section whose operand nobody defined is ordinary content and is drawn. What such a
  name does cost is silent and is not a mark — §14.9's entries, §14.7.5.2's `/MCID`, an artifact's
  property list and §14.13.5's files all arrive through `content::marked::property_list` — and that
  is now half of what keeps the row `partial`, rather than a report written on the spot. **The
  comment beside the code had it right the whole time**, and was one turn behind in the other
  direction: `Unsupported::MissingResource`'s doc says a missing `Properties` list costs no mark and
  named three of the things it leaves out, written before four more readers arrived. It names them
  all now.

## The status that moved, and the test under it

§14.6.1 was `partial` because a tag that *is* a structure type goes unread. §14.7.5.2 says the tag
"is not directly related to the document's logical structure" and makes the sameness a `should` on
the producer, so it is nobody's requirement; the association a reader gets is the `/MCID`, through
§14.7.5.4's parent tree, and that row is `implemented`.

Every other modal in the amended clause binds a producer. The one that binds a reader is its last
sentence — a page's `/Contents`, "whether a single stream or an array of streams, is considered a
single stream with respect to marked-content sequences" — which holds because `ContentReader::for_page`
pumps Table 31's parts into one window, and which nothing in the tree asserted.
`optional_content.rs::a_marked_content_section_may_span_two_parts_of_the_contents_array` asserts it
now: the `BDC` is the whole of the first part, so a reader taking each part as its own stream paints
the square the second part hides. Calibrated per trap 13 by moving the `EMC` back into the first
part — it fails there with its own message — and restored.

`implemented`.

## Two errata, and one of them changes a requirement

`emit` files six annotations on the two pages §14.6.1 spans; four were recorded and two were not.

**Issue #302** adds two pairs to the properly-nested rule — the compatibility pair BX and EX and the
graphics state pair q and Q join BMC…EMC, BDC…EMC and BT…ET. Its whole strikeout is the word *or*,
one word under `check`'s four-word floor, so that instrument is blind to it by construction: the
seventh consecutive round in which a bare or nearly bare caret has been the find. It is a **licence**
rather than a debt — under the amended clause a conforming file may not close a `q` across the `EMC`
`appearance::spliced` cuts at, so the splice is balanced by the standard rather than by luck.

**Issue #301** is capitalisation in Table 352's `BMC` row. And `doc/errata-read.md`'s own §14.6.1
row claimed a stale `variable_text.rs` comment was "owed below" while the same document's settlement
paragraph records it as done — corrected, and it is the round's own subject in a second document.

## The instrument 697 asked for, measured and declined

The obvious construction is the eighteenth sweep with both sides inside one row, and every piece of
it is already public. Measured with one throwaway program, ADR 0481's method: **794 rows with a
note, 259 asserting a term, 930 assertions, 46 contradicted inside one note, 24 marked as a
correction quoting its retired wording — and all 22 unmarked are noise.** The reason not to build it
is structural rather than a vocabulary problem: ADR 0523 made it this project's rule that a
correction states the retired claim in words the sweep can still match, so a note repaired for a
self-contradiction *contains* the contradiction on purpose, and the population is defined to be
dominated by the notes somebody already fixed. It would not have printed either of this round's own
two, which are a cardinal against an enumeration and a `partial` reason against a modal verb. ADR
0560 §5 and `doc/todo/01` carry the numbers and the argument; the program was not committed.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
The full sequence was run rather than the documents-only column: the round adds a test under
`pdf-model`, and `doc/todo/02` §2's map is by crate. The machine carried three other rounds and
began at a load average of 36 on 24 cores, so the lines that spawn a reference renderer were held
until it fell — §2's rule that such a gate measures two programs and a loaded machine is a silent
third.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them after the final edit;
the whole sequence was run a second time after the `report.rs` comment landed. §5's binaries were
not rebuilt — `tools/round.sh` says this is not a fifth round, and nothing here measured a launch
path, a page turn, a frame or a high-water mark.

Thirteen sweeps run before the edits, after them, and a third time on the committed tree carrying
the ADR and `doc/todo/01`'s paragraphs, which are `SOURCE_ROOTS` too. The figures below are the
third run's, and `counts`, `tables` and `pointers` were run once more after this section was
written and printed the same three lines — so the levels a round records are levels the record
itself does not move. **One level moved
on purpose and one is this round's own noise**, and both are worth recording:

- `--bin tables`' absent list went **101 → 100**, which was predicted rather than discovered.
  §14.6's old sentence put `Table 363` in the middle of the tag list, so the sweep attributed
  `/ReversedChars` to it — the ninth sweep's nearest-table rule working exactly as documented. The
  new sentence gives the artifact's property list its own full stop, and the hit is gone.
- `--bin blockers`' ledger count went **23 → 24 with the expired 6 → 7**, on this round's own
  correction sentence naming three clause numbers. It carries `[history]`, which is the mark that
  demotes it, and it is the oldest false positive in this family: a correction quoting the wording
  it retired. Not rewritten to dodge it — ADR 0490 §6.

`--bin unread`'s keys fell **180 → 179**: §14.6.1's old closing sentence denied `/StructTreeRoot`
beside `/MCID`, and only the second survives the rewrite, on the same noise shape (a denial about a
*tag* standing next to a key the row's own code reads).

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Levels,
third run → before: `counts` 7276 ← 7242 sentences with 393 ← 391 attributed counts and **four
places counting one family twice both times**; `quotations` 5729 ← 5707 document spans with
**diverging unchanged at 34**, and 1875 ← 1872 ledger spans with **diverging unchanged at 2**;
`tables` 6156 ← 6145 sentences and 2296 ← 2297 key citations with **contradicted denials unchanged
at 6**; `pointers` 7669 ← 7656 with **absent unchanged at 130 and undefined at 13**; `owed` 3683 ←
3693 terms over 223 ← 224 `partial` rows with **175 unnamed over 111 rows unchanged**; `overtaken`
520 ← 519 decision records with **43 overtaken unchanged**; `entries`, `inapplicable`, `overstated`,
`capabilities` and `callers` all unmoved. `spec-errata check` is byte-identical before and after, and
`applied`'s three counts — 90 quoting a replacement, 10 matching both sides, 171 quoting what an
erratum struck — are unchanged.
