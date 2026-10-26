# 1781 — XFA's second round: a hybrid form's save keeps its `datasets` packet

Session 1781, a clause round in batch 118; 2026-10-26. ADR 2396; ADR 2397 unused; no Q.

**Contract.** Slot 3, RFC 0011 section 13 item 2. `ViewState::save` writes `/V` and never the XFA
`datasets` packet, which breaks §K.2's "The XFA field values shall be consistent with the
corresponding V entries of the PDF field objects". Fix it for the static half and prove it on the
corpora's four hybrids.

**Premises.** *Default binding by name in all four* held only in part. In `bug1815476.pdf` the page
subform states `bind match="none"` and two choices state `match="global"`. Page 176 says nothing of
a `none` subform's children, so ADR 2396 §4 reads it as transparent; the file's data is the
evidence. *One writer function* held (`write_xfa_data`, with the splice in `xfa::datasets`). *The
four save, re-read and agree* held, and mutool's dump of each saved packet agrees. Not foreseen:
each hybrid carries a `form` packet with a `checksum`, which XFA 3.3 does not define, so it is left
as written (ADR 2396 §8).

**Built.** Four modules in `pdf-model`: `xfa::Resource` reads both of §K.2's forms; `xfa::xml` is a
spanned element tree under budgets; `xfa::template` holds the containers, SOM resolution (pages
72–73, 94–96) and chapter 4's direct match; `xfa::datasets` holds the reading, the value rules and
the splice. A field is changed where its inherited `/V` differs through the update. Each changed
field's data value is spliced in and every other byte is the producer's. A field that cannot be kept
is named in `Written::xfa_unkept`, and `viewer-core` reports it. One such case is a numeric or date
field, whose data must be canonical: that is step 3's work, and step 3 is next.

**Files.** New: `crates/pdf-model/src/xfa/{xml,template,datasets}.rs`,
`crates/pdf-model/tests/xfa_datasets.rs`, ADR 2396, this record. Edited:
`crates/pdf-model/src/{xfa,view}.rs`, `crates/pdf-model/src/view/writing.rs`,
`crates/viewer-core/src/viewer/editing.rs` (one call and one helper, `-U0`),
`doc/conformance/ledger.toml` (K.2's note, `-U0`, no status moves), RFC 0011 section 13 and
`doc/state-of-play.md`. I worked in `scratchpad/r1781/`.

**Gates.** Tier 1: `rustfmt --check` on my files, exit 0; `cargo fmt --check -p pdf-model`, exit 0;
clippy `-D warnings` `-p pdf-model --all-targets`, exit 0; `-p viewer-core` lib clean, exit 101 on a
sibling's `tests/movies_and_sounds.rs` only; `cargo nextest run -p pdf-model`, exit 0, 2 248 passed;
`-p viewer-core`, exit 0, 424 passed; `--test xfa_datasets`, 5 passed, 4 red with the write planted
out (trap 13). Against HEAD's tree plus my paths: `save_round_trip`, exit 0, ratchets at their floors;
`writer_corpus`, exit 0. `tools/state.sh --round 1781 docs`, exit 0. `cargo test -p conformance
--no-fail-fast`: exit 101, 465 passed, 1 red and not mine (record 1780's Gates paragraph).
