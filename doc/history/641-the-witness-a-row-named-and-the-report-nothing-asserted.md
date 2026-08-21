# 641 — The witness a row named, and the report nothing asserted

The eleven rows 637 left, read with the rule 620 derived. Four read properly, three defects and one
confirmation, and the round's own contribution is an instrument: two ledger claims that were counts
without a command now have one, which is what told the wrong one from the right one.

Date: 2026-08-21.
ADR: [0472](../adr/0472-the-witness-a-row-named-and-the-report-nothing-asserted.md).

Touched: `crates/pdf-model/src/requirements.rs`, `crates/pdf-model/src/signature.rs`,
`crates/pdf-model/examples/signature_algorithm_census.rs`, `crates/viewer-core/src/notes.rs`,
`doc/conformance/ledger.toml` (§12.8.3.3.2, §12.8.3.4.1, §12.8.3.4.4, §12.8.3.4.6, §12.8.3.4.7,
§12.8.3.4.8, §12.8.5, §12.11, §12.11.2), `doc/todo/01-ledger-partial-rows.md`, the ADR and this
file.

## How the band was ordered

Re-derived rather than taken, which is 616's lesson and every round's since: `git blame
--line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own `note = `
line, ranked by where its commit falls in `git log --reverse`. This base has **833 commits** and
240 `partial`-or-`reported` rows with a blamed note.

It agrees with 637: eleven rows at ranks 513 to 534, and the next row is at 576 — the same
forty-two-commit gap, one row shorter because 637 took §12.7.4 off the top. Nine of the eleven are
§12.8's signature rows and they share one paragraph of boilerplate five times over; the other two
are §12.7.6.2 and §12.11.

620's rule chose the work for the fifth time. Four of the eleven state a reason that is a claim
about this codebase rather than about the standard, and all four were read.

## §12.8.3.3.2 and §12.8.5: two counts, one instrument, one of them wrong

Both rows end in a number over the corpus and neither had a command behind it. §12.8.3.3.2:
"`issue17069.pdf` is the corpus's one witness". §12.8.5: "**No corpus document carries a document
timestamp**, so the witness is a fixture".

`examples/signature_algorithm_census` grew two counters — about twenty lines, on a walk that
already opens every document and reads every `SignedData`. Over the 974: **964 opened, 9 carry a
signature dictionary, 10 dictionaries between them, 0 of them a `/Type /DocTimeStamp`, and 3
signature values carry `adbe-revocationInfoArchival`** — `issue17069.pdf`, `issue6127.pdf` and
`xfa_filled_imm1344e.pdf`. Over `doc/corpora`'s 275: nothing at all.

So §12.8.5 holds and §12.8.3.3.2 was wrong by a factor of three, and **nothing in the tree could
have told them apart**. That is the round's rule, now in `doc/todo/01`: a note stating a count over
the corpus names the command that produces it, or the round writing the count adds one. The same
"one witness" sentence stood in `notes.rs` beside the code, where the gate does not read.

## The evidence gap: 620's third shape, for the sixth round running

Five `reported` rows — §12.8.3.3.2, §12.8.3.4.4, .6, .7, .8 — rest entirely on the third question
being *asked out loud*, and that sentence is `viewer_core::notes`'s. All five cited the same three
tests and all three are `pdf-model`'s. A status whose whole content is "we report this" had no test
in the tree that could fail if the report stopped.

Half of it was covered without being cited:
`notes.rs::a_document_whose_signed_bytes_moved_says_so_and_claims_nothing_more` asserts the
three-questions paragraph including "no certificate store and makes no network request", so the
four revocation rows were right and pointed elsewhere. §12.8.3.3.2's sentence had nothing at all,
and now has `a_signature_carrying_revocation_information_says_so_and_claims_no_more` on
`issue6127.pdf`, with `bug854315.pdf` — signed, no signed attributes — as the silent half.
Mutation-checked: pointing the lookup at another object identifier fails it.

**A sixth row of the same shape, found by reading arrays rather than by the rule.** §12.8.3.4.1's
note ends "which is what a test asserts alongside the one that finds them" and its array named no
PAdES test; the test is `a_pades_signature_is_held_to_the_rules_that_need_no_certificate`, and it is
one test rather than two.

## §12.11: a parent overstating what its children do

`doc/todo/01`'s fifth failure shape is a family head gone stale, and all four recorded instances
have the parent *understating*. §12.11 is the other direction: "Read in full — Table 273's `/S`,
`/V` and `/Penalty`, Table 275's twenty-five types, **Table 276's handlers**", while §12.11.1's row
says `/RH` "is unread, and the requirement it carries is met by construction" and §12.11.5's says
"the `/RH` entry is read by nobody". No source under `crates/` quotes `"RH"`.

The direction is why nothing found it: an overstating parent names a thing the tree *lacks*, which
is the seventh sweep's discriminator, and the seventh sweep only reads `inapplicable` rows.

Beside it, the same correction stopping at the ledger: `MAX_REQUIREMENTS`'s comment said Table 275
defines **twenty-four** types. Both ledger rows said that until the three-hundred-and-seventy-fifth
session counted the table; the rows were fixed and the comment two lines above the `match` was not.
Counted here from `doc/md/`: 8 + 7 + 10 = **25**.

## §12.11.2: the fourth decay of a method that predicts its own decay

Enumerating `Kind::unmet`'s arms against the tree — the technique 637 recommends — found the
signature arm expired. One sentence answered `DigSigValidation`, `DigSig` and `DigSigMDP`: "no
signature validation or signing: §12.8 is read and reported, and verifying a signature needs a
certificate store". A signature's value **is** verified here since ADR 0229, which
`Authenticity::Verified` says and which all ten corpus signatures reach; and Table 275 words two of
the three types as strict increments, so one sentence could not name three. Three arms now — the
trust decision, signing, and §12.8.2.2.2's comparison — gated by
`the_three_signature_requirements_name_three_different_increments`.

The other arms were checked and hold: no command deletes a markup annotation, no `Query` shows a
DPart hierarchy, no control asks for separation simulation, nothing adds or renames an attachment.

## One more, in the source only

`Authenticity::UnknownDigest`'s doc comment said three corpus signatures reach it, each stating
`1.2.840.113549.1.1.5` where a digest algorithm belongs. **None does**, and none could have when
the sentence was written: reading `digestAlgorithm` by shape found the issuer's `SEQUENCE`, and
`cms`'s `the_signers_own_sequence_is_not_mistaken_for_its_digest_algorithm` has pinned the fix
since the three-hundred-and-seventy-seventh session — two hundred sessions before the comment
arrived. An observation of a defect, written down after the defect was gone.

`spec-errata emit` over all fourteen documents before writing: §12.11's errata are Issue #187 —
already recorded in §12.11.1's row, and it vindicates the code — and Issue #656, an editorial column
heading. Nothing touches §12.8.3.3.2, §12.8.3.4.x or §12.8.5.

## Gates

`pdf-model` is the change→gate map's first row and `tools/round.sh` called this a fifth round, so
the whole sequence ran; every line exit 0.

`fmt` clean. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0 — it caught
two things first, a doc-markdown miss inside a quotation and a census function pushed one line over
`too_many_lines`, both fixed rather than allowed. `cargo nextest run --workspace` **2357 passed /
17 skipped**. Doctests clean.

Corpus **974 documents, 68 incomplete** — unchanged, which is the number that says trap 11 was not
sprung. Oracle **1794 pages: 907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2 reference
geometry, 13 not comparable, 18 no render** — identical to 637's, verdict for verdict. Text
extraction 10969/11163 matched words in bounds (98.26%) over 508 documents; PDFBox lane 99.8%
(14257/14281); pdftotext lane 99.2% (22834/23013). `selection_census` 1000/1011 words over 453
documents; `accessibility_census` 104 documents with structure, 90 tagged, 0 pages answering with
structure they do not state; `dates` 1545 strings, 1514 conforming; `xmp` 319 documents, 318 read;
`jpeg2000` green. `render-quorra` 957 pages: 932 agree, 23 differ, 2 refused, 17 not comparable.
`fixed_documents` 31 checked, 0 absent. `cargo test -p conformance` green — **875 rows, 0
unreviewed, 960 verbatim quotations**, and the status breakdown is unchanged at 436 implemented,
222 partial, 18 reported, 78 inapplicable, 8 writer-side, 113 out-of-scope. **No `silent` row.**

Sweeps run because the ledger moved: `quotations` — 1702 ledger quotations, 1 diverging, and that
one is §8.9.5's and was there before; `pointers`, `counts` and `tables` printed their standing
false positives plus this round's own corrections, correctly marked as corrections, and no new hit.
§5's binaries rebuilt and installed.
