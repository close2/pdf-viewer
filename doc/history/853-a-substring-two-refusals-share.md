# 853 — A substring two refusals share

2026-09-01. The `PARTIAL_FILE_ONLY_EVIDENCE_CEILING` reading list, continued: eight rows off it,
13 → 5. The reading found two false sentences about `crate::file_spec`'s callers, and one of the
plants found a named test that passed with the clause it is named after deleted — which is trap 27.

No ADR: nothing here is a decision, and the one lesson that generalises is in
`doc/traps/instruments-and-reports.md` beside the traps it belongs with.

## The rows, and what each named test was calibrated against

| row | new evidence | plant | what failed |
|---|---|---|---|
| §8.10 Form XObjects | three tests, one per child | the form `/BBox` clip taken off | `a_forms_bounding_box_clips_what_it_draws`, 1 of that file's 39 |
| | | a `/Group` of any subtype accepted | `a_form_becomes_a_group_only_for_the_transparency_subtype`, 1 of 39 |
| §8.10.4 Reference XObjects | both new tests | (below) | |
| §8.10.4.1 General | `a_proxy_carrying_ref_is_drawn_as_an_ordinary_form_xobject` | a `/Ref` refused at the `Do` | that test |
| §8.10.4.3 Special considerations | `no_content_of_the_target_page_reaches_the_containing_page` | a naive import: the proxy's content stream replaced by the target page's | both new tests |
| §12.6 Actions | `every_action_type_table_201_names_is_performed_read_or_refused_by_name` | `/Launch` losing its arm of `refused` | that test — "produced 0 actions" |
| §12.6.4 Action types | the same | `/GoToE` dispatched to `/GoTo`'s handler | that test — the *variant* check, which a test that only counted would not have |
| §7.6 Encryption | three of `encryption.rs`'s tests | Algorithm 5 step (e)'s nineteen passes dropped | 6 of that file's 13 |
| §7.6.4 Standard security handler | the same three | **revision 5 accepted outright** | **nothing at all** — see below |

## The plant that found a hole

`an_unspecified_revision_is_refused_by_name` is what §7.6.4.2's `implemented` status rests on: Table
21 says of revision 5 "[s]hall not be used" and states no algorithm for it, so refusing it by name
is the clause met rather than a debt. The test opened `issue21579.pdf` and asserted the refusal's
sentence contained `"/R 5"`.

Delete the refusal — `2..=4 | 5 | 6 => {}` — and **every test in `crates/pdf-syntax` passed**.
`crypt_filters` declines the same document a few lines later for §7.6.4.1's method pairing, and its
sentence begins *"/R 5 with a crypt filter method"*. The substring is there either way. Two ledger
rows rested on a test that could not have failed.

The expected value had been assembled from the *input* rather than from the answer, which is the
generalisation and is now trap 27. The test names Table 21's own words now, and carries the arm
nothing exercised beside it: one byte of `issue21579.pdf` makes a `/R 7` of it — `/R` is a direct
integer, so every offset and every other entry is unchanged — which is §7.6.4.2's *other* refusal,
a revision the table does not list.

## The weakest shape on the list, and why renaming could not fix it

The §8.10.4 family's three rows all named `tests/corpus.rs`. That is a **gate's** file, over a
corpus that holds not one reference `XObject` — so it passes for reasons that have nothing to do
with the clause, and there was no test in the tree to promote the rows to.

`crates/pdf-model/tests/reference_xobjects.rs` is the fixture pair the absence forces, the same
reasoning §8.10.3's row records for `/Group << /S /Softness >>`. It holds §8.10.4.1's permission at
its strongest reading — the same proxy with and without `/Ref` produces the same display list
command for command, is clipped by its own `/BBox` as any form is, and reports nothing, which is the
clause's doing rather than an oversight — and §8.10.4.3's argument that neither of its two
considerations arises, against a target page that is *reachable*: page 1 of the same file, filled,
carrying a `Square` annotation whose appearance is a second colour, named by Table 95's `/F` and
`/Page`. Under the import plant the target's fill reaches the containing page and its annotation
appearance does not, which is §8.10.4.3's first consideration missed in the exact way the clause
describes.

## Two false sentences, found by reading the clause beside the code

`file_spec.rs`'s module comment said "[e]very refusal in this tree that names a file — §7.3.8's
external stream data, §8.10.4's reference `XObject`, §12.6.4.6's launch action, §12.6.4.4's embedded
go-to — names it out of one of these". **Three of the four name no file at all**: §7.3.8's is a bare
`StreamRefusal::External` decided in a crate that cannot depend on `pdf-model`; §12.6.4.6's launch is
a fixed `&'static str`, as `GoToR`'s and `Thread`'s are; and §8.10.4's reference `XObject` reads
nothing, which is the whole of why §8.10.4.1's permission applies. §7.11's ledger row carried the
same claim in a weaker form and was wrong about two of its five. Both are corrected to what is true
— §12.5.6.15's file attachment annotation, §7.11.4's embedded files and §14.13's associated ones,
§12.6.4.4's embedded go-to, §12.7.6.4's import — and what would make the retired sentence true is
written down as what it is: a refusal carrying an owned string, which is a change to `action.rs`'s
boundary that no clause owes.

## The demand-side half: the population, re-asked over the crawl

§8.10.4's row said "no document on this disk states a reference `XObject`" over the curated 1251,
and the new tests' whole justification rests on it — a claim of absence over a population that has
grown by two orders of magnitude since the sentence was written, which is the twenty-third sweep's
subject.

`examples/absence_audit` carries the claim now, asked as §8.10.4.1 states it: a form `XObject` whose
form dictionary holds a `/Ref` **dictionary**, the subtype being part of the condition rather than a
refinement of it — which is what tells Table 93's entry from Table 355's *array* on a `/TOCI`
structure element, the over-report a name census cannot see past.

| population | witnesses |
|---|---|
| curated (1251) | none |
| `SafeDocs` `CC-MAIN-2021-31` (65 944) | none |

67 195 of the 67 460 PDFs on this disk; `corpus-cache/openpreserve`'s 267 are outside either scope
of that example and the prose says so. **Calibrated rather than believed**: pointed at `/Group`
instead, the same block names 75 of `doc/pdf.js`'s documents, so the zero is a measurement and not a
blind spot. Two earlier calibrations failed to show anything and are worth recording — relaxing the
subtype filter named nothing either, because Table 355's `/Ref` is an *array* and never reaches
`dict_of`.

## Table 201, enumerated rather than counted

§12.6's and §12.6.4's rows both summarise this tree as a count, and both had carried one that five
rounds left behind. The new test walks the table's twenty names with the entries each subclause
makes required and classifies each into **performed** (a variant of its own), **read but not acted
on** (`/URI`), or **refused by its own name** — with `/Thread` on both sides, which is the clause's
distinction rather than the function's. The fourth possibility is what it exists to catch: a type
that produces nothing at all, which is indistinguishable from a document that asked for nothing.
Ten, one and nine over the twenty; eight, one and eight within §12.6.4's seventeen. Both rows'
counts survived the enumeration.

## Gates

§2 whole, on a quiet machine, because `pdf-model` and `pdf-syntax` are in the first row of the
change→gate map: formatting and clippy under `RUSTFLAGS="-D warnings"` for both workspaces,
`nextest --workspace`, the doctests, and every gate line plus `cargo test -p conformance`. All
green. `--bin undenominated` was run because this round wrote a count over a corpus, and it is what
caught the 265 documents the sentence had not measured; `--bin pointers` for the moved documents,
which named nothing new.

§5's binaries were not rebuilt: not a fifth round, and nothing here was measured against an
installed binary.

## What is left

Five rows still name a file: §7.7, §8.6.6, §8.6.6.5, §11.7.5, §14.9. §8.6.6 is the pair that
started this — the backwards fold-over rule under `tests/colour_paths.rs` — so it is the one with a
reading already written against it.
