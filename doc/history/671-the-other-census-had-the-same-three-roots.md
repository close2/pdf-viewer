# 671 — The other census had the same three roots

Ten ledger negatives re-derived over `CC-MAIN-2021-31` and **five of them false**, and the reason
none of the ten could have been checked before is the same sentence ADR 0493 wrote about the *other*
instrument: **an instrument has a population too.** The sixteenth sweep has two halves and ADR 0403
insists both be run — `witness_census` for a name, `absence_audit` for the structure a name only
suggests. 667 put `--crawl` on the first. The second still had `doc/pdf.js`, `doc/corpora` and this
project's fixtures hard-coded, so a round could re-derive any claim about a *name* and no claim about
a *construction* — which is what the remaining negatives are mostly about.

Date: 2026-08-22.
ADR: [0496](../adr/0496-the-other-census-had-the-same-three-roots.md).

Touched: `crates/pdf-model/examples/absence_audit.rs`, `doc/conformance/ledger.toml` (§7.11.4.2,
§8.11.1, §8.11.4.5, §10.7.2, §12.2, §12.5.6.21, §12.6.3, §12.6.4.7, §12.7.5.5, §12.9, §12.9.2,
§12.11.1, §14.11.6.2), `doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## The order the instruments gave

**`--bin overstated` first**, and no source opened: **8 contradictions over 170 parent rows asserting
127 terms, 7 marked** — unchanged from 657, 663 and 667. Its one unmarked hit is §12.7's `/AP`
against §12.7.5.5's "Table 236's `/P` is deliberately not read here", and a fourth round has now read
it and agrees it is noise. My ground is the second of 667's, checked in the text rather than
inherited: the parent's sentence asserts Table 170's **appearance dictionary** and the child's denies
Table 236's `/P` in a **signature field lock** dictionary, whose own words — "absence of this key
shall result in no effect on signature validation rules" — put it under §12.8.2.2. Two entries, two
tables, two clauses. It stays unmarked because the parent asserts a bare key, so there is no table
for the `[a table read in part]` mark to divide, and that is a property of the mark rather than a
defect in the hit.

**Then the blame ordering**, re-derived (616's rule): 887 commits, **242** `partial`-or-`reported`
rows with a blamed note. **667's prediction came out exactly, for the fourth band running** — §7.6.4
and §7.6.4.4 at ranks 1–2, §11.3.4 at 3, then the cluster of nine sharing one commit at 4–12, then
§14.6, §14.6.1, §7.6 and §7.7 at 13–16.

**And this round read none of them, which is the third time running that step 7 has outbid the band.**
That is a statement about the two instruments rather than about laziness: the blame list ranks a row
by when it was last *written*, and step 7 ranks a claim by whether the world moved underneath it —
and only the second of those has a population that grew fifty-three times. The band is where it was.

## The ten, and the five that were false

Curated is 1251 files; the crawl is 65 944, of which this tree opens 65 703. Both runs, every time.

| row | curated | crawl | |
|---|---|---|---|
| §12.2's four Table 147 boundary entries | 0 | **96** name them, 95 inside a `/ViewerPreferences` | **false** |
| §10.7.2's `/FL` in a graphics state parameter dictionary | 0 | **88** | **false** |
| §12.6.3's `/PV` and `/PI` | 0 | **5** | **false** |
| §12.7.5.5's `/Lock` on a signed signature field | 0 | **90** | **false** |
| §12.9.2's rectilinear measure | 0 | **127** of 277 stating a `/VP` | **false** |
| §7.11.4.2's `/RF` on a file specification | 0 | **0** | holds |
| §12.11.1's `/Requirements` | 0 | **0** | holds |
| §12.5.6.21's and §14.11.6.2's `/TrapNet` | 0 | **0** | holds |
| §12.6.4.7's thread action | 0 | **0** | holds |
| §8.11.1's and §8.11.4.5's `Zoom`, `User`, `Language` | 0 | **0** of the 475 stating a usage application | holds |

**The last one owed a gate rather than a census** (641's rule), and had one: §8.11.4.5's row already
names `examples/oc_usage_census`, so the re-derivation is that program over a wider argument list —
`find … -print0 | xargs -0 -P 8 -n 400`, four minutes — reporting `View` 758, `Print` 648, `Export`
530 and the three processor-facing categories not once. §12.6.3's count has a gate too,
`actions.rs::the_corpus_states_these_page_scoped_triggers`, which is why the new block asks that
gate's question instead of inventing one.

## The defect this round wrote and the planted file caught

**A hand-built witness stating all seven new constructs**, dropped into `doc/corpora-own` for one run
and deleted, scored **zero for the thread action** — because the first draft asked only the objects
the cross-reference table names, and the file writes its action inline inside the annotation's `/AA`.
That is the six-hundred-and-forty-eighth session's finding, reproduced by a round that had read it
that morning. §10.7.2's resource route was invisible for the same reason, sitting one level under
`/Resources`. `visit` recurses into each object's own structure now, depth-bounded and following no
reference. **Two of seven blocks would have written a false zero into the ledger.**

## Three things worth more than the counts

**A negative can be false with its sharper half intact, and that is a third row rather than either.**
§12.2's sentence was two claims in one: "none states any of the four boundary entries" is false, and
"the half of the clause that can change a pixel has no corpus witness" is **true** — every one of the
96 states `/ViewArea` and `/ViewClip` as `/CropBox`, Table 147's own default for both, and the single
document naming a box the table does not default to states it as `/PrintArea /MediaBox` and
`/PrintClip /MediaBox`, the pair nothing here prints. Writing only *false* would have deleted a true
sentence and lost the reason the row is calm.

**Where the two instruments disagree, the direction is the finding.** §7.11.4.2's `/RF`: **55 710**
crawled documents' raw bytes contain the token, **32 192** documents' decoded streams do, **one**
states it as a name in an object, and **none** carries it on a file specification. §12.11.1's
`/Requirements`: **411** documents' streams and no catalog at all — a census of English prose. A byte
search would have called both clauses well witnessed and both rows wrong.

**And `spec-errata emit` before writing found a live erratum on a clause this round was editing.**
Issue #371 strikes §10.7.2's "It shall be a positive number" and writes the 0-to-100 range in its
place. It moves nothing here — a processor exercising the clause's permission never reads the number
— and the finding is *where it already was*: `doc/errata-read.md` carries it, read correctly and
disposed of correctly, and the row never heard. The seventeenth sweep asks whether a place recording
an erratum applied it; this is the mirror, a place that read one and left the conclusion in its own
document.

## The instrument, before and after

Thirteen sweeps run before the edit, after it, and a third time on the committed tree (ADR 0485).
**Two hit lists moved** — `overstated` 8 contradicted with 7 marked, `counts` 4, `quotations` 2
diverging in the ledger and 34 in the documents, `entries` 177 over 49 rows, `pointers` 118 absent
and 13 undefined, `owed` 181 unnamed terms over 114 rows, and `blockers`, `capabilities`,
`inapplicable`, `callers` and `overtaken` at their standing populations.

**`--bin tables`' absent list went 99 → 100 and the hit is this round's own prose**, found only
because the sweeps were run a third time on the committed tree instead of on the ledger alone: an
ADR, a history file and a todo edit are `SOURCE_ROOTS` too. §12.2's finding turns on a default, so the
sentence carrying it names a page-boundary *value* beside the number of the table whose entry takes
it; the sweep reads the pair as a key citation, prints the right answer itself (`stated by: Table 31,
Table 396`) and marks the hit `[correction]`, which demotes it. It is the second of the three noise
shapes its own closing paragraph names. **It briefly read 102**, because the first drafts of the ADR
section and of this one each repeated the pairing they were describing — documenting this shape
instantiates it, once per place. The finding's sentence is not rewritten to dodge that (ADR 0490 §6);
its description gives the example once, and the level settles at 100.

**`--bin unread` is the one that moved, and it is ADR 0493's noise shape one sweep over.** Confirmed
46 → 44, quoted 136 → 138, both on the single key **`/FL`** under §8.4.5's row and §10.7.2's — because
the census written *to measure how many documents state `/FL`* names the string `"FL"`, and that
sweep asks whether any source quotes a key a row calls unread. A round that measures an unread entry
makes its own row look wrong. The repair is neither to drop the census nor to teach the sweep about
examples: the sweep's read-first list is the keys named by *the row's own `code` array*, and that
number did not move (68 both runs).

The levels that moved are this round's own sentences, measured on the committed tree against what
each was before: `counts` 6835 → 6877, `quotations` 1769 → 1772 in the ledger with all three new ones
verbatim and 5316 → 5335 over the documents, `tables` 5836 → 5861 sentences and 2213 → 2217 key
citations, `pointers` 7115 → 7153, `owed` 3530 → 3553 terms, `entries` 283 → 285 rows, `overtaken`
493 → 494 decision records, and `overstated`'s corroborations 55 → 56 — that last being §12.9's row
now asserting a term §12.9.2's corroborates, which is the direction a parent and a child should move
in.

**`owed` gained no phantom this time**, and the reason is worth the sentence: the citations added are
`examples/absence_audit` and `examples/witness_census`, whose leading segments are `absence` and
`witness` — ordinary English words occurring in those very files — so the extractor's phantom key is
*named by a source* and never reaches the unnamed list. ADR 0493's shape costs a round one phantom
only when the invented noun is invented.

## Where the sweep stands

**It is not finished, and the honest form of that is a list rather than a number.** `doc/todo/01`
carries the command that splits the population by whether a row names the crawl, and the reading
beside it. Of the **45** rows carrying such a sentence, 10 named the crawl before this round and 11
more do now, leaving **24** — and they are not twenty-four more runs of `witness_census`. They sort
into four groups that add up: **5** need a content-stream census nobody has written (§8.5.2.1,
§9.4.2, §9.7.5.4, §9.7.6.2, §11.6.7); **10** need a structural block of the kind this round added
eight of (§7.6.5, §7.9.2.2.2, §8.9.5.2, §8.10.3, §11.6.5.2, §12.3.2.2, §12.4.2, §12.5.1, §12.8.2.2.1,
§14.8.2.5.3); **6** are not claims about a corpus at all, two of which owe an existing census a
`--crawl` argument rather than a reading; and **3** are this population's own noise — a correction
quoting the negative it retired, already repaired, nothing owed.

§12.8.2.2.1 is the next false one waiting, and it can be named in advance because half of it is
already measured: `witness_census --crawl` says **144** crawled documents name a `/DocMDP` against
the corpus's one, and only its `/P` values are unmeasured.

What *is* finished is the part a name census can answer.

## Gates

The change reaches `crates/pdf-model` (one example), so the map asks for everything, and the whole of
`doc/todo/02` §2 was run.

- `cargo fmt --all --check` — exit 0.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — exit 0.
- `cargo nextest run --workspace` — **2426 passed, 17 skipped**, 76 s.
- `cargo test --workspace --doc` — exit 0.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — exit 0.
- `cargo build --profile gates -p pdf-sandbox --bins` and `-p hayro-compare --bin pdfref-hayro` —
  both exit 0 (trap 10).
- **corpus** — exit 0: 974 documents in 3.5 s, 0 unopenable, 8 locked, 2 encrypted beyond us, 6
  pageless, 68 incomplete, 0 slow.
- **oracle** — exit 0: agrees 908 (863 on pages called complete), contradicted 65, ambiguous 786, our
  geometry 2, reference geometry 2, not comparable 13, no render 18 — every one of the seven the same
  as 667's, which is what a round that moved no drawing code should see.
- **text extraction** — exit 0: 99.8% (14257/14281 words) against PDFBox in both orders with 4 below
  90%, 99.2% (22834/23013) against `pdftotext` with 22 below 90%, and the position gate 10969/11163
  in bounds (98.26%), 486 of 508 documents fully in bounds.
- **selection census** — exit 0: 1000/1011 words selected (98.91%) over 453 documents.
- **accessibility census** — exit 0: 102 853 elements reached, 57 116 a caret can move through.
- **dates** — exit 0. **xmp** — exit 0. **jpeg2000** — exit 0. **fixed documents** — exit 0: 40
  checked, 0 absent.
- **quorra corpus** — exit 0: 957 pages compared, 933 agree, 22 differ, 2 refused, 17 not comparable;
  median page 2.66× the CPU backend.
- `cargo test -p conformance` — exit 0. **875 rows**, breakdown unchanged at 436 implemented, 224
  partial, 18 reported, 76 inapplicable, 8 writer-side, 113 out-of-scope, 0 unreviewed, no `silent`
  row. **No status moved**, which is right: five rows lost a false sentence about the world and five
  gained the evidence they were resting on.

The reference cache was **copied** rather than shared — `pdfref-cache` under this worktree's own
target directory — so the oracle's 908 agreements are not a read of a directory three neighbours are
writing.

**§5's binaries were deliberately not installed**: this is a parallel round told not to push or merge,
`target/` is the *main* tree's, and putting an unmerged branch's binaries where a person runs them is
what §5 exists to prevent. The merge round owns it.

## Overlap with the parallel rounds

670 and 672 ran beside this one. Nothing written here is outside the thirteen rows named above, and
no other row was reflowed.
