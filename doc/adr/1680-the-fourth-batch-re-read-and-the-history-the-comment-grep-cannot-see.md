# 1680 — The fourth-batch re-read, and the history the comment grep cannot see

Session 1423. Status: **accepted**. ADR 1638 rule 7's scheduled re-read of the four navigational
documents, under `CLAUDE.md`'s *Where knowledge lives* and ADR 1023's rule; it decides no new rule
and records what the re-read found, so that the next one starts from the residue rather than from
the documents again.
Context: ADRs 1023, 1403, 1576, 1637, 1638.
Prose: `doc/PLAN.md`, `doc/crate-map.md`, `doc/state-of-play.md`, `doc/HANDOVER.md`;
`crates/viewer-ui/src/bin/quorra/cadence.rs`, `surface.rs` (two comments).

## 1. How the four were checked

Three instruments and a reading. `cargo metadata --no-deps` against the crate map's rows (43
members and `kio/`, no row missing and none extra); `tools/state.sh navigation`, whose `pointers`
run named one absent pointer in the four (`doc/questions/A170`, which is the owner's answer and is
uncommitted in the main checkout, so it resolves there); and a throwaway extractor over every
backticked path and identifier in the four, resolved against `git ls-files` and every identifier the
tree's sources hold — which the `pointers` sweep does not do for a bare file name or a type. Then
every sentence was read as *what is*, and a counted claim checked by its command.

## 2. What was false, and what replaced it

| the document said | the tree says | checked by |
|---|---|---|
| `render-raster` draws with `QuorraPresenter` | `QuorraWindowRenderer` | `grep` of `crates/render-raster/src/lib.rs` |
| `viewer-ui`'s sidebar has three tabs; "a native host would" draw them | `viewer_host::Tab` has six, and both native hosts draw them | `crates/viewer-host/src/panel.rs` |
| `pdf-vfs-ffi` has 35 entry points (crate map and state of play) | 40, and the count is now the `grep -c` that prints it | `grep -c 'unsafe(no_mangle)'` |
| `pdf-colour` has six modules | seven: `black_generation.rs` was unnamed | `ls crates/pdf-colour/src` |
| `viewer-gtk` is four modules and `Host`, `HostError` | six modules, and `Opening` is public too | `ls`, `crates/viewer-gtk/src/lib.rs` |
| `pdf-signature`'s modules (no `policy.rs`) | `policy.rs` is ADR 1219's signature-policy reader | `ls` |
| `confined-transport` exists for *two* workers | three: `pdf-script-worker` speaks over it too | `grep -l confined-transport crates/*/Cargo.toml` |
| `pdf-script-worker` is "the third confined program" | it is the third of `pdf_sandbox`'s *profiles*; the programs are four | `crates/pdf-sandbox/src/lockdown.rs` |
| the transform suite has three corpus walks beside its gate | one per writer, which `doc/todo/02` §2 names | `doc/todo/02` §2's `pdf-transform` row |
| a hand-written list of the fuzz targets | the list had fallen behind `fuzz/fuzz_targets/`; the directory is the population now (trap 25) | `ls fuzz/fuzz_targets` |
| hayro "is ahead on feature completeness, and not narrowly", listing features this tree lacked | every feature listed is in this tree; which is further on is the ledger's and the oracle's to say | the ledger; `crates/pdf-font/src/predefined.rs` |
| "embedded `CMap`s are 14 documents in the text gap" | `cmap.rs` in `pdf-font`, a fuzz target of its own | `fuzz/fuzz_targets/cmap.rs` |
| "nothing in the tree tracks which requirements are implemented" | the ledger does; the sentence is now the argument for it | ADR 0016 |
| `render-gpu` "is permitted `unsafe`… and contains none" | its header permits it and the crate *forbids* it | `crates/render-gpu/src/lib.rs` |
| `json.rs` "a hundred lines", the ZIP reader "200 lines" | 158 and 511; the sentences now name the modules, not their size | `wc -l` |

Chronology removed: every "now", "yet", "the same round", "found by", "for the whole of its life",
"which the owner ratified on" a date, and the history the transform, archive and file-system
paragraphs carried of which round found what. Each was a sentence about how the tree got here; the
ADR it cited keeps it.

## 3. What `doc/state-of-play.md` lost, and why it was safe

1 379 lines and 21 870 words became 1 032 and 16 307. **What went is what another file already
carries**: `raster-gpu`'s construction-by-construction list (its crate-map row names the module of
each, and the ADRs stay listed in the capability sentence that replaced it); the converter's
per-mitigation paragraphs (`doc/pdf-a-mitigations.md` is that catalogue, with its clause); the
redaction's and the transform suite's operator-by-operator detail (`doc/todo/57`, RFC 0002, the
`redact` modules); the file-system faces' (RFC 0003 and two crate-map rows); a census percentage,
three cost figures and two launch timings (each ADR the sentence cites holds it). Every ADR number
the removed prose cited is still cited by the sentence that replaced it. `doc/PLAN.md` 757 → 756
lines (9 125 → 9 115 words), `doc/crate-map.md` 64 → 64 (10 539 → 10 524), `doc/HANDOVER.md` 88 →
88 (1 600 → 1 608): those three were corrected rather than cut.

## 4. The comment grep's 230 hits are this program's own sessions

`grep -rE "hundred-and-|session" --include=*.rs crates tools` prints 230 lines; `tools/comment-
history.py` sorts them into 111 in code, 95 legitimate, 6 unread and 18 "history" — and all 18 are a
`setsid` session, the batch branch's first session number, and `ledger_notes.rs` describing its own
gate. **Not one is a round's history.** Two legitimate-classed comments told a defect's story in the
past tense (`cadence.rs`, `surface.rs`) and were rewritten to the present reason, ADR 0384 cited.

**The history the rule is about is spelled `round <number>`, and nothing counts it.** 56 comment
lines in 20 files under `crates/` (`viewer-ui/tests/launch_path.rs` 16, `pdf-vfs` and `pdf-fuse` 30
between them) cite a round by number — `(round 911)`, "round 938 measured" — which is exactly "which
session changed it". `CLAUDE.md`'s grep does not match it, `spelled_ordinals.rs` holds only the
spelled and the capitalised forms, and `comment-history.py`'s `HISTORY` has no `round` arm. Owed,
and to whom:

- **The instrument first** (trap 13): `comment-history.py` and `spelled_ordinals.rs` gain the digit
  form `round <number>` in a comment, counted and printed; the site in `tools/conformance/tests/
  bounded.rs` that parses a lock log's `round 103` is data, and a fixture string in
  `crates/pdf-vfs/tests/write_corpus.rs` ("round 909 wrote this") is code the rule reaches as
  `spelled_ordinals.rs` says a string literal does. The instruments slot.
- **Then the 56 sites**, each to the ADR its round wrote — 911 → ADR 0864, 923 → 0886, 935 → 0910
  and 0911, 938 → 0916 and 0917, 909 → 0860 and 0861, 914 → 0870, 902 → 0847, 896 → 0836, 908 →
  0858 — one crate per round as ADR 1023 §5 says, the crate's owner of that batch doing it.

## 5. Found and left to its owner

`crates/pdf-sandbox/src/lockdown.rs` calls `Profile::Decoder` "the narrower of the two" beside a
`Script` profile documented as "the narrowest of the three". The owner's open question Q287, in
the main checkout only, asks whether the winit window stays the flagship; `doc/PLAN.md` §1 states ADR 0001's
reason as it stands and will need the answer.
