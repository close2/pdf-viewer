# 1033 — a name the clause restricts, and a print half split three ways

Date: 2026-09-13. ADR 1050. Two rows read. Touched `text_string.rs`, `collection.rs`, `page.rs`,
`viewer-confined`'s panel codec, `viewer-host/src/panel.rs`, `chrome.rs`, three tests, the ledger.

**§12.3.5.2 — the conditional `shall` is taken, and the choice is support.** The clause bounds a
collection's names with five bullets and one further sentence, then: "An interactive PDF processor
may choose to support invalid names or not. If not, an appropriate error message shall be
provided." `folder` applied none of the six, leaving the program in the third state that sentence
does not offer; `is_file_name` existed and only its own tests called it. All six are applied now
and the choice is **support**, because the alternative loses content the same clause requires shown
— §12.3.5.1's `shall`, and "all files in the EmbeddedFiles name tree … shall be treated as members
of the folder structure", whose remedy for an unparseable key is the root folder and never refusal.
`Collection::invalid_names` is that choice's other half rather than a comment, and
`viewer_host::panel::restricted_names` the sentence all three windows say: supporting a name is not
being quiet about one.

**The first restriction is about bytes** — §7.9.2.2's three encodings can each be contradicted by
the bytes claiming them and `text_string` answers U+FFFD for all three, so the new
`pdf_syntax::is_text_string` sits beside the decoder. **The sixth is about two names** — subfolders
and files share one namespace per folder, the clause defining *file name* for "[a] folder, as well
as its associated files", making Table 159's sibling sentence a restatement. The row stays
`partial`, held now by one sentence: Annex #21's `toCasefold` against `char::to_lowercase` (ADR
1050 §4). And a producer breaks one of the six — `digitally_signed_3D_Portfolio.pdf`'s root folder
is named `()`, outside "between 1 and 255 inclusive", asserted in `tests/collections.rs`.

**§12.2 — the print half is three debts, not one.** `/PrintArea` and `/PrintClip` ask a *renderer*
what `/ViewArea` and `/ViewClip` ask, and `Pages` carries one pair: one function wide, unbuilt
while nothing renders for paper, said so in `page.rs` beside the field. Five state their obligation
about a *dialogue*, a host's — `Query::Preferences` hands all five over, principle 3's shape
reached without anybody calling it that, so the debt is per host and no host has a print action.
`/PrintScaling` carries the one `shall` that is not a dialogue's — "If the print dialogue is
suppressed … this entry nevertheless shall be honoured" — so it is first to build; §8.11.4.5's
`Print` event waits on the same missing *operation*. Row stays `partial`, no code.

Gates, all exit 0: `fmt --all --check`; `nextest --workspace` (4595 passed); `--workspace --doc`;
both `fuzz/` lines; and the whole of tier 2 — `pdf-model` corpus, `raster_golden` (unmoved),
`dates`, `xmp`, `jpeg2000`, `pdf-transform` gate, `pdf-syntax` on_disk, `viewer-ui` launch_path.
Workspace clippy and `-p conformance` fail **only** on a sibling's in-flight `annotation_state.rs`.
Both new reports have a control (trap 13): the conforming folder and key absent, the sentence
missing for a valid name.
