# 2535 — The rewrite reaches every place the validator counts, and the census counts the signature sentence

Status: implemented. Session 1850. Q716 is not asked; section 4 says what the shipped profiles should do at part 4 and
why it is not built here.
Context: ISO 19005-2 §6.2.6, §6.3.2 and §6.2.11.5; ISO 32000-2 §9.6.2.1 (Table 109) and §12.5.3; trap 5; ADRs 0947, 2534;
`doc/pdfa-remedies.md` (the `signature-assertion` loss).
Code: `crates/pdf-transform/src/archive/sites.rs` (`rendering_intents`, `reports_a_prefix`),
`archive/rewrite.rs` (`annotation_objects`, `Sites::direct_annotations`), `archive/rewrite/annotations.rs`
(`flag_direct_annotations`), `archive/fonts.rs`, `archive/fonts/described.rs` (`state_laid_out_widths`),
`crates/pdf-transform/tests/archive_corpus.rs` (`convert_under`), `tests/archive.rs` (the tests named below).
Documents: `doc/pdfa-blocking-issues.md` section 8.

## 1. A rewrite gated on a prefix (rendering intents)

`pdf-archive` keeps at most 32 places of a failing requirement and counts the rest (`Findings`, `MAX_KEPT`). The
rendering-intent rewrite took its sites from that list, so a document stating the unknown intent in 114 places had 32
restated and failed the second verdict (`AIAA-2002-4016-606`). Where `Outcome::Failed`'s `total` exceeds the places
kept, the sites are found structurally by `pdf_archive`'s own rule for what is a graphics state (§8.4.5's `ExtGState`
table, or a dictionary typed `ExtGState`) and for an image; an object that is neither is not a site. `/RI` is restated
only in a dictionary the rewrite knows to be a graphics state (`Held`): ISO 32000-2 Table 192 gives a push button's
`/MK` an `/RI` that is a form XObject, its rollover icon, and rewriting it to a name would destroy the producer's content.
Test `a_rendering_intent_past_the_validators_kept_places_is_restated_with_the_first` (40 places; with the fix removed
it restates 32 of 40) and `a_push_buttons_rollover_icon_beside_more_intents_than_the_validator_keeps_is_left_as_written`
(a button beside 40 states; with every object a site it reports 41 places changed).

**Not fixed**: the other users of `sites::objects_named` (blend modes, halftones, the shape of an appearance …) take
the same prefix. None is shown by the corpora to fail past 32 places; each is a latent defect of the same cause.

## 2. An annotation written inside `/Annots`

An annotation dictionary written directly in a page's `/Annots` array has no object number, so the flags rewrite, which
walked object numbers, never reached it and the output kept an annotation with no `/F` (§6.3.2: the flag is required).
`annotation_objects` now also returns the pages that hold such a dictionary, and `flag_direct_annotations` writes the
page's array back with the flags given to each dictionary that states none (`needs_flags`: not a popup, no integer `/F`).
Test `an_annotation_written_directly_in_the_annots_array_is_given_the_flags_the_clause_requires` fails without it.
`CLLASS Report for ER&L 2020` and `Wiki SI 563 2021` convert at both parts.

## 3. A font with a descriptor and no widths

Table 109 makes `/FirstChar`, `/LastChar`, `/Widths` and `/FontDescriptor` all present or all absent for the standard
fourteen. A font that states a descriptor and no widths was embedded as a face with a dictionary that stated none, so a
checker reading the dictionary found every shown code 0 wide (§6.2.11.5). `state_laid_out_widths`, the function the
existing path already used, is called for it: the numbers written are those the viewer draws the font with, so no glyph
moves. Test `a_font_with_a_descriptor_and_no_widths_is_given_the_widths_the_viewer_draws_it_with` fails without it.
`NeilJairath_3`, `issue2099-1` and `issue5470` convert; `NeilJairath_3` at part 4 is now refused by the press, with its sentence.

## 4. The signature sentence the census did not count

The five documents `no-decision` at part 4 (`bug854315`, `issue16553`, `issue17069`, `Montenegro2021`,
`signed_verified`) were not refused without a sentence: `signature_decision` refuses a signed source with
`--authorise signature-assertion allows it`, a decision that is a field of the conversion (`Conversion::signatures`)
and not a row of `decided`. `convert_under` read only `decided`. It now reads `signatures` when `decided` has no
refusal, and files the document under `signatures/signature-assertion`, which `rank.py` already maps to R16.

**What the profile should do, and why it is not built here.** At 2b both shipped profiles `discard`
`file-structure/document-signature-states-no-digest`, which authorises `signature-assertion`. At part 4 that row does
not exist, so `Configuration::authorisations` authorises nothing there and every signed source stops. The table of
losses (`doc/pdfa-remedies.md` section 5) keys the loss by loss, and lists both shipped profiles as
`discard/stop`: the loss is that the output carries no signature, which is the same loss at both parts. The proposal is
that a `discard` at the 2b row authorises the loss at every target. It is not built because it widens what a shipped
profile deletes at a target the profile's author did not name, and `authorisations` is being edited by another round in this batch
(another round's uncommitted edit of `config.rs` is in the same tree); it is a one-line change after the batch merges, and it is the orchestrator's call
whether it goes to the owner as a question.

## 5. Evidence

veraPDF at ISO 19005-2 (2b) and ISO 19005-4: `failedRules` 0 for `AIAA-2002-4016-606`, `CLLASS Report for ER&L 2020`,
`Wiki SI 563 2021`, `NeilJairath_3` (2b), `issue2099-1` and `issue5470`; page counts equal to the source's.
