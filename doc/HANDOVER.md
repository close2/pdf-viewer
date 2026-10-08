# Handover

Read `/CLAUDE.md` first — the five principles, what *done* means, and the closed exclusion list.
**Principle 5 is the one that changes how you work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never the
definition of right.

**This file is an index and nothing else.** It says which file a round with a given job opens;
everything it points at is stated in full one hop away. It carries no numbers — `tools/state.sh`
prints those (ADR 0281) — and no round reads it top to bottom: the every-round reading is the
five short things `tools/round.sh` prints, and this table is opened for a file that list did not
give you (ADR 1639).

**A lesson lives here exactly once**: in a trap if it changes how you write code, in a habit if it
changes how you work. A session's narrative belongs in its ADR and in [`doc/history/`](history/README.md),
nowhere else; every sentence here carries the current reason and the ADR by number, and a retired
sentence is deleted rather than annotated (ADRs 0232, 0281, 0428, 0974, 0983, 1023).

**The round's own record is one *new file*, never an edit to an existing one** —
`doc/history/<session>-<slug>.md`, and nothing about the round anywhere else
([`doc/history/README.md`](history/README.md), `doc/todo/02` section 0).

## Every round

`tools/round.sh` prints it, and `tools/round.sh --lines` counts it: `CLAUDE.md`;
[`doc/todo/02`](todo/02-every-round.md) **section 0** (the round on one page — contract, tier 1,
the record); the rule block that opens [`doc/environment.md`](environment.md) (one line per
shared-machine rule); [`doc/traps/every-round.md`](traps/every-round.md) (the ten traps the records cite
most, as `tools/state.sh traps` counts them); [`doc/habits/every-round.md`](habits/every-round.md) (the ten
habits the records show rounds paying for). Then `tools/round.sh <kind>` for the kind of round you
are, and what your contract names. The brief is written from
[`doc/todo/_brief-template.md`](todo/_brief-template.md) and repeats none of this.

## By what the round is

| a round that | opens |
|---|---|
| asks what the program already does | [`doc/state-of-play.md`](state-of-play.md) — the capability list, and which clause each came from |
| asks what is left, and why it is not done | [`doc/todo/65`](todo/65-the-remaining-frontier.md) — the `partial`/`reported` rows grouped by their blocker; a status change edits its map in the same pass (it is a gate) |
| wants a number | `tools/state.sh` — `quick` in seconds, a section in minutes; never a document. What a merge's gates cost is `gates-cost`, a batch's clock `batches` (ADR 1476) |
| needs the shape of the batch loop, or runs a merge | [`doc/todo/02`](todo/02-every-round.md) section 8 and [`tools/batch.sh`](../tools/batch.sh), whose header lists its verbs in the order a merge runs them (ADRs 1313, 1511, 1526); `check` before reporting |
| reads a clause, or writes a ledger row | [`doc/habits/reading-the-specification.md`](habits/reading-the-specification.md), [`the-ledger-and-claims-about-this-tree.md`](habits/the-ledger-and-claims-about-this-tree.md), [`doc/todo/01`](todo/01-ledger-partial-rows.md); [`doc/ledger-and-claims.md`](ledger-and-claims.md) and [`doc/errata-read.md`](errata-read.md) on demand. An `inapplicable` row rests on a condition the clause itself states ([`doc/PLAN.md`](PLAN.md) section 5a, ADR 1461); `departed` is one decided `shall` inside an otherwise-executed clause, priced by its ADR — a declined `should` or a permission not taken is `implemented` with the choice named (ADRs 1119, 1622) |
| opens an encrypted corpus document, or writes a gate that walks the corpus | `crates/pdf-model/tests/support/corpus_passwords.rs`, the one table of published passwords every corpus gate reads through a `#[path]` (ADR 1377) |
| judges a page against other renderers — a robustness round | `tools/state.sh oracle-held` first (ADR 1512), then the oracle's own ranking in `crates/pdf-model/tests/oracle.rs`'s `WHOSE_DEPARTURE` (ADRs 1483, 1572), then [`doc/habits/judging-against-other-implementations.md`](habits/judging-against-other-implementations.md), [`doc/oracle-and-corpus.md`](oracle-and-corpus.md), [`doc/todo/00`](todo/00-ambiguous-bucket.md) |
| needs the owner's word on something | [`doc/questions/`](questions/) — one `Q` file per open question with a recommendation, answered by an `A` file of the same name; written in the same commit as the work that raised it (`doc/todo/02` section 6a) |
| **measures** anything | [`doc/habits/measuring.md`](habits/measuring.md), [`doc/performance.md`](performance.md), [`doc/verify.md`](verify.md); the number is printed, never quoted — and a figure a gate already holds (a band in `doc/checks/`, a ratchet, a held list) is read from the gate, not re-taken (ADR 1638) |
| changes what `render-raster` or `raster` draws | a change under `raster/crates/raster-gpu/src/` runs `tools/batch.sh raster-examples` behind the lock (ADR 1575); `crates/render-raster/tests/corpus.rs` is the gate against the processor's raster at 1× and 4×, its `differs` lists empty, a page that comes to differ read first against a per-pixel reference from its own geometry (ADRs 1435, 1471) |
| measures **latency** — a page turn, a zoom step, launch | [`doc/performance.md`](performance.md) section 3e is the baseline, re-taken in one sitting; **the gate a performance round leaves green is `turn_path`** (`cargo test --release -p render-raster --test turn_path -- --ignored --nocapture`, ADRs 1513, 1537, 1577, 1607), its bands in `doc/checks/turn-path.toml`, a band moved only with its reason beside it (trap 97). A stroke has two expansion paths in `raster-gpu` (trap 78). The other instruments: `tools/state.sh frame`, `--test launch_path` with `PDFVIEWER_LAUNCH_CLOCKS=1` ([`doc/verify.md`](verify.md)), callgrind — each arm built once per commit in a target directory of its own (trap 50) |
| writes a host, or adds a message | [`doc/ui-boundary.md`](ui-boundary.md), [`doc/todo/30`](todo/30-a-native-host.md)–[`33`](todo/33-annotation-editing.md), [`doc/todo/38`](todo/38-a-documents-restrictions-have-levels.md) — and the last thing it runs is [`tools/drive-windows.sh`](../tools/drive-windows.sh), all three windows under Xvfb, each step waiting for the window's own word (ADRs 1453, 1478, 1605; trap 86); `tools/state.sh drive` counts its verdicts |
| writes a whole file — `split`, `merge`, `pages`, `optimize`, `optimize --linearize`, `redact` | [`doc/todo/57`](todo/57-the-transform-suite.md), ADRs 1124 and 1371 for redaction, [`doc/rfc/0002`](rfc/0002-the-transform-suite.md) |
| validates a document against ISO 19005, or converts one | [`doc/rfc/0006`](rfc/0006-pdf-a-validation-and-conversion.md) and [`0007`](rfc/0007-a-refusal-is-a-question-somebody-can-answer-in-advance.md), [`doc/pdf-a-mitigations.md`](pdf-a-mitigations.md), [`doc/todo/66`](todo/66-the-mitigation-catalogue-build-out.md), [`doc/third-party-data.md`](third-party-data.md) for the texts |
| asks about JavaScript, or a script a document carries | [`doc/rfc/0008`](rfc/0008-a-script-is-a-document-acting-on-its-reader.md) — accepted by the owner (`doc/questions/A193`), built in its section 11's order; what is built is what `doc/state-of-play.md` says. The gate a change to `pdf-script` leaves green is the Tier 1 column, `--features engine --test script_corpus` behind the lock (ADR 1625) |
| asks whether an exclusion should lift, or what printing still owes | [`doc/rfc/0009`](rfc/0009-what-each-exclusion-protects-and-what-lifting-it-would-mean.md) — proposed, `doc/questions/Q254` open, nothing of it built |
| adds or questions a dependency | [`doc/stack.md`](stack.md), [`doc/third-party-data.md`](third-party-data.md), [`doc/PLAN.md`](PLAN.md) section 1 |
| fixes a defect in a dependency this tree pins from a fork, or patches one from crates.io | a patch under `doc/patches/` against the pinned `rev`, opening with `Repository:` and `Base:` (ADRs 1447, 1463), or with the `Fork:` the owner is to create (ADRs 1520, 1589) — `zune-jpeg`'s is `close2/zune-image`, created and pinned by `rev` (`doc/questions/A227`, ADR 1730); `tools/state.sh main-checkout` lists the owner's step until the manifest pins it |
| touches text this program writes itself — variable text in a right-to-left or cursive script, an interface label | `crates/pdf-font/src/shaping/` (ADRs 1413, 1414), its tables under `data/unicode/` with `PROVENANCE.md`; [`doc/stack.md`](stack.md) says why content a document positioned is never shaped |
| touches `pdf-signature`'s own curves, `brainpool_p512.rs` or `ed448.rs` | ADRs 1385, 1386, 1538 and [`doc/stack.md`](stack.md): the owner's exception (`A170`), and the swap condition with its re-check command in [`doc/todo/65`](todo/65-the-remaining-frontier.md) |
| fetches a free specification text | into `scratchpad/r<round>/` first and never under `doc/` (trap 43); `tools/spec-md.py` into the ignored `doc/md/`; a section in [`doc/third-party-data.md`](third-party-data.md); the PDF kept at `/home/AI/specs/` (ADR 1349) |
| quotes, or wants to quote, a standard that is not ISO 32000-2 | [`doc/third-party-data.md`](third-party-data.md), ADRs 0187 and 1085 — cite the section and paraphrase, never quote, nothing of it committed; the ETSI texts permit no reproduction at all |
| runs the program | `target/quorra` in the main checkout, filled by the merge's `tools/batch.sh install` (ADR 1511); a measurement builds its own `--release` first; [`doc/running-the-viewer.md`](running-the-viewer.md) |
| runs an instrument that is not a section 2 gate, or fuzzes | [`doc/verify.md`](verify.md) — `deny`, callgrind, the cross-target checks, the censuses, AT-SPI, and the fuzz block: `tools/fuzz.sh`, `fuzz/seeds.sh check <target>` before a campaign (ADR 1559), `-s none` behind the lock, artefacts into `scratchpad/r<round>/` (ADR 1423); a new target owes its `doc/verify.md` line and its `fuzz/seeds.sh` arm (ADR 1439) |
| leaves something on the owner's disk, or wants to know what CI says | [`doc/environment.md`](environment.md)'s *After a merge* and `tools/state.sh main-checkout` — the owner's list, numbered, each with its command (ADRs 1440, 1563, 1588, 1601) |
| looks for where something lives | [`doc/crate-map.md`](crate-map.md), [`doc/PLAN.md`](PLAN.md) |
| asks *when* something landed | [`doc/history/`](history/README.md), one file per round from 446 on; [`doc/history.md`](history.md) for the rows before it |

## Traps and habits

[`doc/traps/README.md`](traps/README.md) is the full trap index — one row per trap, the position
that springs it and the rule, resolving any citation by number in one hop, and
`cargo test -p conformance --test traps` holds every row to its entry in the group file it names
and every entry to its row (ADR 1379). A round reads [`every-round.md`](traps/every-round.md) up
front and opens a group file where a line bites (ADR 1036). A trap's story is never rewritten,
but a pointer in it is kept current (ADR 1525). `tools/state.sh traps` counts what the rounds cite.

[`doc/habits.md`](habits.md) is the index of the six habit files, one per kind of work (ADR 0983);
[`doc/habits/every-round.md`](habits/every-round.md) is the ten every round reads, and
`tools/round.sh <kind>` names the file for the rest. Three habits bind every round and live in
`doc/todo/02` section 7.

## The texts under `doc/md/`, and the one rule that is not about code

ISO 32000-2's sentences are quoted verbatim and the conformance gate checks every one of them
against `doc/md/`. **Every other specification text on this disk is held as licensed to a single
reader: cite the clause or the section and paraphrase, never quote, and nothing of it is
committed** (`/doc/*.pdf` and `/doc/md` are ignored; what is tracked is the encrypted
`doc/specifications.zip`). ADR 0187 is the position; ISO 19444-1 and the XFDF 3.0 text are held
under it (ADR 1297), and the three ETSI texts permit no reproduction in any form (ADR 1085).
[`doc/third-party-data.md`](third-party-data.md) states it per text.
