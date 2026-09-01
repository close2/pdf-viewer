# 852 — A file that passes whatever it contains

2026-09-01. An instruments round: the shape round 851 was bitten by, made mechanical and then
walked, with the population it names read against the clauses and the code.

## The instrument

`FILE_ONLY_EVIDENCE_CEILING` has counted `implemented` rows since the four-hundred-and-twenty-first
session and nothing else. Its argument — *a row naming `file.rs` names something that passes
whatever it contains, so the claim is held by nothing* — is about **evidence**, and a `partial`
row's `test` is evidence for the half of the clause that *is* executed; `check_evidence` already
demands one for exactly that reason. Nobody had joined the two, so 222 rows, a third of the ledger,
sat outside the only instrument that reads the shape. `doc/todo/01` records the toleration twice,
approvingly, and never argues it.

`PARTIAL_FILE_ONLY_EVIDENCE_CEILING` is the same count over `partial` rows, ratcheted downward and
printed with its row list so it is a reading list as well as a gate.

**Calibrated rather than believed** (trap 13), and both directions were run:

| the ledger | the gate says |
|---|---|
| as it stood | 23, the hand-derived list to the row |
| §11.4.4's four named tests truncated to their two files | 24, and it fails |

The hand-derived list was itself a claim about the tree (trap 25); the point of running the
instrument at 23 first was that a mechanical population and a hand-written one agreeing is what
makes either believable.

## Rows moved, and what each named test was calibrated against

Ten rows, 23 → 13. Every named test was calibrated by planting the defect it is supposed to guard
and watching it fail; the plants are listed with what they cost, because a plant that fails the
wrong set of tests is the finding.

| row | plant | what failed |
|---|---|---|
| §8.9.6 Masked images | `/ImageMask` never true | 7 of `image_masks.rs`'s 17 |
| | `/Mask` never read | 7, including the explicit and colour-key tests |
| | the colour-key test on the samples switched off | exactly the 2 colour-key tests |
| §8.11 Optional content | the `/ON`/`/OFF` arrays never applied | 11 of `optional_content.rs`'s 27 |
| | `OptionalContent::is_locked` answers `false` | **nothing at all** — see below |
| §8.11.1 General | membership `/P` forced to hold | 2 |
| | an XObject's own `/OC` never read | 3 |
| | `/BaseState /OFF` read as `ON` | 1 |
| §8.11.4 Configuring | `read` tolerates a missing `/OCProperties` **and** an undeclared group hides | 2 |
| | `/AS` never consulted | 4 |
| §8.11.4.1 General | as above | as above |
| §11.3.7 Shape and opacity | the graphics state's soft mask never reaches the state | 1 + 5 |
| | `knockout_elements` emits the bare command | 4 |
| §11.3.7.2 Source shape and opacity | `ca` and `CA` never read | 4 of `transparency.rs`'s 5 |
| | `/AIS` never read | exactly the 2 tests the row's `/AIS` sentence is about |
| §11.4 Transparency groups | a form's `/Group` unrecognised at the `Do` | 23 of 39 |
| §11.6 Specifying transparency | `/BM` never read | exactly 1 |
| | an image's own `/SMask` never read | 5 |
| §11.6.5.2 Soft-mask images | `/Matte` never undone | exactly 1 |

## The hole a plant found, and the test that closes it

§8.11's row claims a panel "throws the switch through `Command::SetGroup` — with Table 99's
`/Locked` refusing the change, which is the clause's own sentence". With
`OptionalContent::is_locked` planted to answer `false`, **every test in `viewer-core`, `viewer-ui`
and `viewer-ffi` passed.** `pdf-model`'s `a_locked_group_refuses_a_panel_and_not_an_action` pins
`ViewState::set_group`; `viewer-ui`'s `a_layer_switch_throws_unless_the_document_locked_it`
hand-builds the `locked` flag of the rows it draws. Nothing held the path from the document's array
to the flag a host is told — `viewer.rs`'s `locked: content.is_locked(*group)` could have been
`false` and no gate would have said so.

`crates/viewer-core/tests/headless.rs::a_locked_group_reaches_a_host_and_its_switch_is_refused` is
the fixture that holds both halves, and it was calibrated on each: the flag lost (`is_locked`
false), and the refusal lost (`set_group` dropping the lock from its condition).

## Three things about planting, worth carrying

- **A defect can be refused twice, and then one plant fails nothing.**
  `a_document_with_no_properties_dictionary_draws_everything` survives a `read` that tolerates a
  missing `/OCProperties` *and* survives a `state_of` that hides an undeclared group. §8.11.4.2's
  "shall ignore" is enforced at two independent points, so it takes both plants at once. A test
  needing a conjunction is still a test — but a round that names it must know that, or it reads one
  clean plant as an empty test.
- **The plant has to be at the site the row's sentence names.** A form `/Group` unrecognised at
  `transparency.rs:240` fails three tests; the same thing unrecognised at the `Do` fails
  twenty-three. Only the second is what §11.4's row means by a group.
- **A named test can be riding another entry's plant.** `/Matte` was calibrated a second time on
  its own entry, because a test that fails only when a *different* entry is broken is evidence
  about that entry.

## The stale claim in the code

`optional_content.rs`'s *What is deliberately not here* listed five things — `/AS`, `/RBGroups`,
`/Locked`, `/Order`, `/Configs` — as absent because they "describe an interactive processor
offering the user a layer panel", and ended "[w]hen a layer panel exists, this is the module it
attaches to". The panel arrived in the hundred-and-sixty-seventh session, four of the five are
implemented in this very file, and the sentence stayed: `doc/todo/02` §1's "a capability that
arrived and announced nothing". Corrected to the one thing genuinely absent — Table 98's `/Configs`
and a configuration's `/Name` and `/Creator`, all three of which exist so a person may choose
*between* configurations, which nothing in this tree offers.

## The demand-side half: nine witnesses, and a denominator I got wrong first

The new test's doc comment said a fixture was the witness "because no corpus document this tree has
states a `/Locked` array". That is a claim of absence over an unnamed population, which is the
twenty-third sweep's whole subject — and a grep cannot settle it, because a configuration
dictionary usually lives in an object stream. So `examples/oc_usage_census` now counts Table 99's
`/Locked` and Table 98's `/Configs` beside the usage categories it already read.

| corpus | open | `/OCProperties` | `/Locked` | `/Configs` |
|---|---|---|---|---|
| `doc/pdf.js` | 964 | 31 | 0 | 0 |
| `doc/corpora` (four) | 273 | 13 | 0 | 0 |
| `corpus-cache/openpreserve` | 264 | 13 | 0 | 0 |
| `corpus-cache/safedocs` `CC-MAIN-2021-31` | 65 703 | 2409 | **9** | **1** |

The claim of absence was false, and the first figure written down was wrong a second way: a single
`find corpus-cache` walk folded `openpreserve` into the crawl and reported 65 967. Both are
corrected in the row and in the test.

**Taken through to a verdict**: of the nine, five present an `/Order` as well, and on every one of
the five this reader refuses the switch on exactly the groups the array names and accepts it on
their neighbours — 21 rows in `0300227.pdf`, 5 in `5097719.pdf`, 2 in `5343400.pdf`, 48 in
`6942736.pdf`, and `1530801.pdf` among the four that lock groups they do not present, which
§8.11.4.3 settles: "[a]ny groups not listed in this array shall not be presented in any user
interface that uses the configuration". No defect. The probe was a throwaway example, deleted.

## Gates

§2 whole, on a quiet machine: formatting (both workspaces), clippy under `RUSTFLAGS="-D warnings"`
(both workspaces), `nextest --workspace`, the doctests, and all ten gate lines plus
`cargo test -p conformance`. All green. `--bin undenominated` was run because this round wrote a
count over a corpus, and it is what caught the folded denominator above.

## What is left

Thirteen rows still name a file: §7.6, §7.6.4, §7.7, §8.6.6, §8.6.6.5, §8.10, §8.10.4, §8.10.4.1,
§8.10.4.3, §11.7.5, §12.6, §12.6.4, §14.9. The §8.10.4 reference-XObject cluster is three of them
and its evidence is `tests/corpus.rs` — a *gate*'s file, which is the weakest shape on the list,
because that file passes for every document in the corpus and says nothing about reference
XObjects at all.
