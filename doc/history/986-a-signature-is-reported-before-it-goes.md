# 986 — A signature is reported before it goes, and the question is the conversion's own

Date: 2026-09-12. Branch `round-945/the-fifth-round`, from `0dde6224`. ADR: 1007. Stream: the
PDF/A converter, `crates/pdf-transform`. Five sibling rounds were live in the same working tree
throughout (984 review, 987 colour and images, 988 rendering and transparency, 989
`pdf-archive`, 990 `pdf-syntax`, the viewers and `tools/`), none of them in `crates/pdf-transform/`.

## What the round was asked to do, and what it did

Build `doc/pdf-a-conversion-limits.md` section 3.6, which ADR 1006 found the converter had never
honoured: a loss kind for a signature's cryptographic assertion, the rewrite that removes each
signature's value and keeps the field and its appearance, the report line per signature over the
source, and the catalogue corrections.

- **`Loss::SignatureAssertion`**, `--authorise signature-assertion`, wired exactly as the four
  losses before it: one variant, one word, one `Authorisations` field, and a compile error
  everywhere it has to be answered.
- **The question is asked by the conversion, not by a requirement row** — `Conversion::signatures`,
  a `SignatureDecision` beside `decided`. A rewrite invalidates every signature whatever
  requirement asked for it, and a part 4 target has no row about a signature's range, so a
  converter that removed signatures only where a row failed would have gone on writing lying
  signatures under PDF/A-4 with nothing to catch it. ADR 1007 section 2.
- **`Rewrite::SignatureValueRemoved`**: each field's `/V` goes (§12.7.5.5); `/Perms`'s `DocMDP`
  and `UR3` go (Table 263, §12.8.2.3); Table 225's `AppendOnly` bit clears. The field, widget,
  `/AP`, `/Lock`, `/SV`, `/DSS`, `/Legal`, `SignaturesExist` and an emptied `/Perms` stay, each
  with its sentence in `signatures.rs`'s module comment and ADR 1007 section 3.
- **`Rewrite::ForeignPermissionHandlers`**, `Mechanical`: a `/Perms` key outside Table 263 names
  a permission handler §12.8.6 does not define, so no conforming processor could consult it. Both
  corpus fail-cases for `file-structure/permissions-dictionary-keys` carry `/XX (value)` and no
  signature; the catalogue had lumped the row in with the signature rows.
- **The report line**, computed over the *source* before anything moves: where reached, `/Name`,
  `/M`, `/Reason`, what `DocMDP` permitted, `/ByteRange` coverage, `Signature::integrity`,
  `Signature::authenticity`. Never the word *valid*. On the corpus's three certification
  signatures it says the source was modified after signing and the value verifies over a digest
  the source no longer produces — which is how veraPDF made its fail-cases, and the report says
  so without anybody having to know it.
- **The proof**: the output is walked the same three ways (field tree, every page's `/Annots`,
  `/Perms`) and refused by name if a signature remains.
- **Three `REFUSED_BY_NAME` rows moved to `REMEDIES`** — `document-signature-states-no-digest`
  and `digest-covers-the-whole-file` under the loss, `permissions-dictionary-keys` mechanical —
  and the two constants `SIGNATURE_STRUCTURE_NOT_BUILT` and `SIGNATURE_RANGE_IS_THE_SIGNERS` are
  gone. `tests/archive_unconsidered.txt` is unedited and still empty.

## The eight documents

| document | target | before | after, default | after, `--authorise signature-assertion` |
|---|---|---|---|---|
| `6-1-12-t01-fail-a` | 2b | refused (`SIGNATURE_STRUCTURE_NOT_BUILT`) | converts, `/XX` removed, conforms | same |
| `6-1-12-t01-pass-a` | 2b | copied | copied | copied |
| `6-1-12-t02-fail-a` | 2b | refused | refused, signature named, `--authorise` offered | converts, conforms |
| `6-1-12-t02-fail-b` | 2b | refused | refused, signature named | converts, conforms |
| `6-1-12-t02-fail-c` | 2b | refused | refused, signature named | converts, conforms |
| `6-1-12-t02-pass-a` | 2b | copied | copied | copied |
| `6-1-11-t01-fail-a` | 4 | refused | converts, `/XX` removed, conforms | same |
| `6-1-11-t01-pass-a` | 4 | copied | copied | copied |

Every converted output was held to its target again by this tree's validator and conforms; a
converted `t02-fail-a` has `/Perms << >>`, `/SigFlags 1`, the field without its `/V` and with
its `/Lock`, `/T` and `/F`, and no `ByteRange` anywhere in the file. veraPDF, run as evidence and
not read, passes all five converted outputs.

## Requirement identifiers

None added, none removed, none re-keyed. Three moved from a refusal to an answer:
`file-structure/permissions-dictionary-keys` (`Mechanical`),
`file-structure/document-signature-states-no-digest` and
`signatures/digest-covers-the-whole-file` (`Loses(SignatureAssertion)`).

## The figures, off the run

The census (`cargo run -q -p pdf-transform --example archive_census`): **`unconsidered` 0 at all
six targets**; `remedy` 47/49/51/46/46/46 and `refused` 71/71/76/65/64/63 for 2b/2u/2a/4/4f/4e —
three rows moved from the second column to the first at the part 2 targets and one at part 4.

`archive_corpus` (`tools/bounded.sh -- cargo test --profile gates -p pdf-transform --test
archive_corpus -- --ignored --nocapture`), exit 0, 17 s, peak 0.87 GiB over the process tree,
where session 985's merge had stopped it on the first signed document:

| target | conforming, still conforming | every loss authorised: converted / refused | default: converted / refused |
|---|---|---|---|
| PDF/A-2b | 377 / 377 | 433 / 176 | 135 / 474 |
| PDF/A-2u | 13 / 13 | 1 / 8 | 1 / 8 |
| PDF/A-2a | 11 / 11 | 4 / 12 | 4 / 12 |
| PDF/A-4 | 128 / 128 | 171 / 188 | 150 / 209 |
| PDF/A-4f | 4 / 4 | 5 / 2 | 4 / 3 |
| PDF/A-4e | 11 / 11 | 5 / 3 | 3 / 5 |

The same walk at `0dde6224`, run on a detached worktree of `HEAD` with its own target directory
and the submodules linked in, exit 0: PDF/A-2b **429 / 180** authorised and **134 / 475**
default; PDF/A-4 **170 / 189** and **149 / 210**; the four small targets unchanged. The
difference is exactly the five fail-cases — four at 2b under every loss authorised (the `/XX`
document mechanically, the three certification signatures under the new word), one at 2b and
one at 4 by default (the two `/XX` documents, which need no authorisation) — and no other
document moved in either direction.

## Gates

Run detached, each step started only when no sibling walk was on the machine and the load was
under 5 (`gates/summary.txt` in the round's scratchpad has each exit status and its seconds):

- `cargo fmt --all --check` — exit 0.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — **stops in
  `tools/conformance/tests/state_sections.rs:31`** (an unfulfilled `#[expect]`), round 990's
  untracked file; `pdf-transform` checks clean there and alone, and `cargo clippy -p
  pdf-transform --all-targets` reports nothing in the crate.
- `cargo nextest run --workspace --no-fail-fast` — **4390 run, 4389 passed, 1 failed, 35 skipped**;
  the one is `conformance::state_sections every_gate_line_the_sequence_states_is_one_the_state_script_runs`,
  the same sibling file. Every `pdf-transform` test passes: 74 in `tests/archive.rs` (three new),
  4 unit tests in `signatures.rs`.
- `cargo test --workspace --doc` — exit 0. Both `fuzz/` lines — exit 0.
- `cargo build --profile gates -p pdf-sandbox --bins` — exit 0.
- `pdf-transform`'s row, each under `tools/bounded.sh`, all exit 0: `gate` 2 s (peak 0.60 GiB),
  `writer_corpus` 9 s (0.67), `split_corpus` 55 s (2.98), `merge_corpus` 59 s (2.93),
  `pages_corpus` 89 s (3.29), `optimize_corpus` 37 s (3.12), `foreign_corpus` 76 s (6.56),
  `archive_corpus` 17 s (0.87).
- `cargo test -p conformance -- --nocapture` — 234 + 6 + 1 + 1 + 1 passed; **exit 101 on
  `tests/state_sections.rs`** alone, round 990's.

Whoever merges owns `doc/todo/02` section 2's sequence on `main`; the two red lines are one
sibling file.

## Files touched

- `crates/pdf-transform/src/archive/signatures.rs` — new: the walk, the report line, the proof,
  the foreign-key reading, four unit tests
- `crates/pdf-transform/src/archive/decision.rs` — the loss, three rows moved, two constants gone
- `crates/pdf-transform/src/archive/rewrite.rs` — two rewrites, the permissions and form edits
- `crates/pdf-transform/src/archive/prepare.rs` — `Prepared::signatures`,
  `Owed::foreign_handlers`, two obstacle arms
- `crates/pdf-transform/src/archive/report.rs` — `SignatureDecision`, the report section, JSON
- `crates/pdf-transform/src/archive/mod.rs` — the conversion's own question, the proof, the
  module comment
- `crates/pdf-transform/src/bin/quorra-transform.rs` — `--authorise` help text
- `crates/pdf-transform/tests/archive.rs` — three tests and a signed fixture
- `doc/pdf-a-conversion-limits.md` section 3.6, `doc/pdf-a-mitigations.md` section 2's
  permissions entry
- `doc/adr/1007-a-signature-is-reported-before-it-goes.md` — new
- this file

`crates/pdf-model/src/signature.rs` was read and not touched; nothing in it needs to change.

## What the next round should know

- **The identity path lists nothing.** A conforming signed source is copied and its signatures
  are kept, and the report says only that it conformed. Listing what it keeps would be a courtesy
  and is not built.
- **`doc/pdf-a-mitigations.md`'s `xmpMM:History` entry, appended page and attached source for a
  signature are `doc/rfc/0007`'s configuration** and wait on it; the catalogue entry now says so.
- **The trap ADR 1007 section 8 records**: a loss caused by serialising rather than by an object
  has no requirement to be keyed by, and the census cannot count it. A file's `/ID` derived from
  its content and a linearisation hint table are the same shape.
