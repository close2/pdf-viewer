# Handover

Read `/CLAUDE.md` first — the five principles, what *done* means, and the closed exclusion list.
**Principle 5 is the one that changes how you work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never the
definition of right.

**This file is an index and nothing else.** It says which file this round opens and which traps
this round is in a position to spring; everything it points at is stated in full one hop away, in
the file a round with that job opens. It carries no numbers — `tools/state.sh` prints those, and
`doc/traps/instruments-and-reports.md` says how to read them (ADR 0281).

**A lesson lives here exactly once**: in a trap if it changes how you write code, in
[`doc/habits.md`](habits.md) if it changes how you work. A session's narrative belongs in its ADR
and in [`doc/history/`](history/README.md), nowhere else. If you find yourself retelling a session
here, you are undoing what this file is — and `CLAUDE.md`'s comment rule says the same of every
sentence in it: the current reason, the ADR by number, and a retired sentence deleted rather than
annotated (ADRs 0232, 0281, 0428, 0974, 0983, 1023).

**And the round's own record is one *new file*, never an edit to an existing one.** Write
`doc/history/<session>-<slug>.md`, named so that `ls` sorts it last, and write nothing about the
round anywhere else: not into `doc/history.md`, whose table is closed and whose one exception —
a block summary — belongs to a closing round alone; not into a table here, in
[`doc/todo/README.md`](todo/README.md) or in [`doc/todo/02-every-round.md`](todo/02-every-round.md);
and not into a neighbouring session's file. [`doc/history/README.md`](history/README.md) is the
argument and says what goes in it.

## Which file this round needs

Each of these is *all* of what it holds, not a précis. Open the one your round is about — that is
what these files are split by.

**Every round, whatever it is about — and this list is short on purpose:**

| | |
|---|---|
| [`doc/todo/README.md`](todo/README.md) | the index of owed work, one file per item, `ls` sorting by priority |
| [`doc/todo/02-every-round.md`](todo/02-every-round.md) | what a round does around whatever it takes: which gates its change needs, the sweeps, the binaries, the commit |
| [`tools/round.sh`](../tools/round.sh) | run it first: the next session number, the reading list for this kind of round, whether the full gate sequence is owed, and each thing a round has got wrong here before |
| [`doc/todo/02-every-round.md`](todo/02-every-round.md) §8 | **the round's own contract**, which is what a brief is written from: the ledger rows named by number, the record budget a record is counted against, scratch under `scratchpad/r<round>/` because a path a sibling also writes is a log one of you loses, and waiting on a pid you hold rather than on a `pgrep -f` that matches its own command line |
| **one heavy walk on the machine at a time** | six rounds share this machine, and six concurrent corpus walks is what the kernel's out-of-memory killer takes a round for. Anything that walks a corpus — a `tools/state.sh` section that runs for more than a few seconds, any `--profile gates --test … --ignored` line — goes behind `RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh -- <command>`: the lock queues you, and you wait *in* it rather than polling around it; the wrapper holds the command to the agent's task budget as well as to its memory bounds, because a tool that forks without waiting is what took every round down once (trap 116, ADR 1612). It costs wall-clock and it costs no correctness; a killed process costs the batch. `cargo test -p conformance` is not a walk and needs neither |
| [`tools/batch.sh`](../tools/batch.sh) | the orchestrating loop's command — `open`, `gates`, `check`, `commit`, `close` — with §8 as its reason. `check` prints, one line each, the things a merge would otherwise have to remember and exits non-zero if any bites; run it before reporting. It reads the same NUL-separated population `commit` stages, so a path git would print quoted is placed by its real directory. A sibling's in-flight file shows up in it as a finding, which is the instrument working. `commit` stages the batch's population by name and refuses on any difference from the index, and `close` refuses a worktree holding anything uncommitted (ADR 1313). The merge runs tier 1, the gates, `commit`, the fast-forward, then `cargo test -p conformance` **in the main checkout** before anything else — a check that fails only for tracked files passes in every round and fails there first (trap 100) — then `install`, then `close`, which prints the batch directory's `debug` and its prune when it is over the hundred-gigabyte rule and runs none (ADR 1526); the prune is run between `close` and `open`, which prints the same line before it makes the next worktree |
| [`tools/worktree.sh`](../tools/worktree.sh) | the *per-round* worktree beside it — a branch with a build directory of its own, the gitignored data linked in, every gitlink pinned — and `close` takes the checkout and the build directory away together. [`doc/environment.md`](environment.md) is the prose |
| [`doc/environment.md`](environment.md) | the machine, the agent's account, the display, the build directory, the working agreements, and the one command a fresh clone needs |
| [`doc/traps/README.md`](traps/README.md) | the trap index: one line per trap, the position that springs it and the rule; open a group file where a line bites |

**And then by what the round is:**

| a round that | opens |
|---|---|
| asks what the program already does | [`doc/state-of-play.md`](state-of-play.md) — the capability list, and which clause each came from |
| asks what is left, and why it is not done | [`doc/todo/65`](todo/65-the-remaining-frontier.md) — the `partial`/`reported` rows grouped by their blocker (host surface, dependency, architecture, feature depth, nothing owed), for steering the campaign |
| wants a number | `tools/state.sh` — `quick` in seconds, the whole thing in minutes; never a document |
| reads a clause, or writes a ledger row | [`doc/habits/reading-the-specification.md`](habits/reading-the-specification.md) and [`the-ledger-and-claims-about-this-tree.md`](habits/the-ledger-and-claims-about-this-tree.md), [`doc/ledger-and-claims.md`](ledger-and-claims.md), [`doc/errata-read.md`](errata-read.md), [`doc/todo/01`](todo/01-ledger-partial-rows.md) — and an `inapplicable` row rests on a condition the clause itself states, asked of the writers, the print path and the archive path as well as the viewer, never on "this is a viewer" ([`doc/PLAN.md`](PLAN.md) §5a, ADR 1461) |
| moves a ledger row that records **one decided departure** inside an otherwise-executed clause | it takes `departed`, not `partial`: every requirement executed except the one the note's first sentence names and its ADR prices; `tools/state.sh` counts it apart from `implemented` and `partial` (the owner's word, `doc/questions/Q63`, ADR 1119) |
| opens an encrypted corpus document, or writes a gate that walks the corpus | `crates/pdf-model/tests/support/corpus_passwords.rs` — the one table of the corpus's published passwords, which every corpus gate reads through a `#[path]` rather than a copy of its own, so a password added there reaches every gate at once (ADR 1377) |
| judges a page against other renderers — a robustness round | `tools/state.sh oracle-held` first: every verdict's held pages by group and size, read from the oracle's own constants without the walk, with the departure-of-ours candidates named (ADR 1512). Then the oracle's ranking: every contradicted group says whose departure it holds — ours, the references', or a documented choice — in `crates/pdf-model/tests/oracle.rs`'s `WHOSE_DEPARTURE`, beside the clause its note decides it by, and the run names the highest-ranked page held as a departure of ours, which is the next page to take, or says that no page is (ADRs 1483, 1572). Then [`doc/habits/judging-against-other-implementations.md`](habits/judging-against-other-implementations.md), [`doc/oracle-and-corpus.md`](oracle-and-corpus.md), [`doc/todo/00`](todo/00-ambiguous-bucket.md) |
| needs the owner's word on something | [`doc/questions/`](questions/) — one `Q` file per open question, answered by an `A` file of the same name whose `Status` header says what the answer left open; write yours in the same commit as the work that raised it |
| **measures** anything | [`doc/habits/measuring.md`](habits/measuring.md), [`doc/performance.md`](performance.md), [`doc/verify.md`](verify.md) — and `tools/state.sh`, because the number has to be printed rather than quoted; what a merge's gates cost is `tools/state.sh gates-cost`, and a batch's clock across batches `tools/state.sh batches` (ADR 1476) |
| changes what `render-raster` or `raster` draws | a change under `raster/crates/raster-gpu/src/` runs `tools/batch.sh raster-examples` behind the lock — CI's fourteen examples with `--check` under Xvfb, which `cargo test` never builds (ADR 1575); and `crates/render-raster/tests/corpus.rs`, the gate against the processor's raster at the page's scale and at four times it: its two `differs` lists hold no page, so a page that comes to differ is first read against a per-pixel reference computed from its own geometry — §10.7.4's pixel as a unit square, each mark's area intersected with its clip, composited in paint order — before either backend is called right and the page is held to a bound (ADR 1435); `REFUSED_BEFORE_THE_SCENE` is empty too, since a group resolved after it composites is drawn as a frame of its own (ADR 1471) |
| measures **latency** — a page turn, a zoom step, launch | [`doc/performance.md`](performance.md) §3e: its frame table is the baseline, re-taken in one sitting, so a figure is quoted only against one taken that day or re-taken beside it; ADR 1395 is the per-commit method. **The gate a performance round leaves green is `turn_path`** (`cargo test --release -p render-raster --test turn_path -- --ignored --nocapture`, ADR 1513): it holds every `turn` and `step` row of that table to a band in `doc/checks/turn-path.toml`, and each page's `seventh` row — the same turn as the seventh frame of a device that has drawn six (ADR 1607) — out of the bottom as well as the top, beside a command count no clock moves, and only on a quiet device — a "not judged" line is not a pass (ADR 1537); every round of the method starts by keeping the cores it may run on busy for a moment, so a figure is of the tree rather than of how long the machine idled (ADR 1577). A round that moves a row on purpose — a cost paid for exactness, or a win — re-takes it, moves the band by the file's own rule and writes the reason beside it; a slowdown nobody chose is not a reason, and a re-taken table that no band follows is trap 97. A stroke has **two** expansion paths in `raster-gpu`, `encode/stroke.rs` beside `encode/parallel.rs`, and a probe of one concludes nothing about the page (trap 78). The instruments are `tools/state.sh frame` (`render-raster`'s `examples/frame_budget`), `--test launch_path` with `PDFVIEWER_LAUNCH_CLOCKS=1` ([`doc/verify.md`](verify.md)), `render-raster`'s corpus gate at `PDFVIEWER_RASTER_SCALE=1` and `4`, and callgrind for a count the load cannot move — each arm built once per commit in a target directory of its own and told apart by `md5sum` (trap 50), pinned as [`measuring.md`](habits/measuring.md) says |
| writes a host, or adds a message | [`doc/ui-boundary.md`](ui-boundary.md), [`doc/todo/30`](todo/30-a-native-host.md)–[`33`](todo/33-annotation-editing.md), and [`doc/todo/38`](todo/38-a-documents-restrictions-have-levels.md) for the restriction levels' operations and what the two gestures still owe — and the last thing it runs is [`tools/drive-windows.sh`](../tools/drive-windows.sh), which takes all three windows under Xvfb through what a reader does and photographs each step, each step waiting for the window's own word under a ceiling rather than for a fixed time (ADR 1605), and whose verdicts `tools/state.sh drive` counts, with where its time goes — per window, per step group and per step, from the time column each verdict carries; a test suite green over a host is not a host that works; each step's verdict is read off the window — a trace, a note, AT-SPI's text, a colour's pixels — and the one step with no witness but its picture, `quorra`'s reopened form, is a golden a person looked at once (ADRs 1453, 1478, trap 86) |
| writes a whole file — `split`, `merge`, `pages`, `optimize`, `optimize --linearize`, `redact` | [`doc/todo/57`](todo/57-the-transform-suite.md) (the suite, Annex F's writer among it), ADRs 1124 and 1371 for redaction, [`doc/rfc/0002`](rfc/0002-the-transform-suite.md) for the design |
| validates a document against ISO 19005, or converts one | [`doc/rfc/0006`](rfc/0006-pdf-a-validation-and-conversion.md) and [`0007`](rfc/0007-a-refusal-is-a-question-somebody-can-answer-in-advance.md) (the designs), [`doc/pdf-a-mitigations.md`](pdf-a-mitigations.md) (every refusal's remedy), [`doc/todo/66`](todo/66-the-mitigation-catalogue-build-out.md) (which remedies this version carries out, and the command that prints it), [`doc/third-party-data.md`](third-party-data.md) for the texts |
| adds or questions a dependency | [`doc/stack.md`](stack.md), [`doc/third-party-data.md`](third-party-data.md), [`doc/PLAN.md`](PLAN.md) §1 |
| fixes a defect in a dependency this tree pins from a fork (`close2/hayro`'s codecs) | the fix is a patch under `doc/patches/`, written against the pinned `rev` and opening with `Repository:` and `Base:` lines, because a round cannot push to the fork; the owner applies it and bumps the `rev` ([`doc/environment.md`](environment.md)'s *After a merge*, its `doc/patches` line), `tools/state.sh main-checkout` lists it until the manifest no longer pins its base, and a test of the defect names the patch and admits both outcomes — the tree's own bound before it is applied and the codec's after — so the bump moves no gate (ADRs 1447, 1463); a patch to a dependency taken from crates.io with no fork pinned (`zune-jpeg`'s two) is written the same way, its preamble naming the question that decides it and the `Fork:` that is to carry it: `zune-jpeg`'s is the owner's to create under `doc/questions/A227`, the stanza that wires it is written out in a comment above `zune-jpeg` in the root `Cargo.toml`, each patch's upstream report is the `.md` beside it, and until the manifest pins the fork `main-checkout` prints the owner's step in one sentence (ADRs 1520, 1589) |
| touches text this program writes itself — §12.7.4.3's variable text in a right-to-left or cursive script, or an interface label | `crates/pdf-font/src/shaping/` (`pdf_font::shaping`: UAX #9's order through `unicode-bidi`, the cursive joining, the machine face), ADRs 1413 and 1414; its tables are `data/unicode/`'s `BidiMirroring.txt`, `DerivedJoiningType.txt`, `ArabicShaping.txt` and `UnicodeData.txt`, compiled in by `pdf-font`'s `build.rs`, with `data/unicode/PROVENANCE.md` and `NOTICE` section 5 for their terms; the gate is `crates/pdf-font/tests/bidi_character_test.rs`, every line of the UCD's `BidiCharacterTest.txt`; [`doc/stack.md`](stack.md) says why content a document positioned is never shaped |
| touches `pdf-signature`'s own curves, `brainpool_p512.rs` or `ed448.rs` | ADRs 1385 and 1386 and [`doc/stack.md`](stack.md): the two are the owner's exception to taking reviewed arithmetic (the owner's answer A170), and the swap condition is the owner's — the day a stable, reviewed package covers a curve, the swap is decided on `doc/stack.md`'s terms and the file is deleted; what meets that condition is a stable release on RustCrypto's own line or a package with an audit covering the arithmetic and the verification, a search's tail below it is not re-read, and the re-check that prints exactly those is the command block in [`doc/todo/65`](todo/65-the-remaining-frontier.md) beside its last date (ADR 1538) |
| fetches a free specification text a clause hands its subject to | into `scratchpad/r<round>/` first and never under `doc/` (a PDF there joins the oracle's population, trap 43); `python3 tools/spec-md.py <pdf> --out doc/md/<name>.md` into the ignored `doc/md/`; a section in [`doc/third-party-data.md`](third-party-data.md) with the URL, the SHA-256 and what the notice permits; the PDF kept at `/home/AI/specs/`. ITU-T T.4 and T.6 are the precedent (ADR 1349) |
| quotes, or wants to quote, a standard that is not ISO 32000-2 | [`doc/third-party-data.md`](third-party-data.md), which states the position per text, and ADRs 0187 and 1085 — see the rule below |
| runs the program | `target/quorra` in the main checkout, which the merge's `tools/batch.sh install` fills with the commit beside it and `tools/state.sh binaries` dates (ADR 1511); a measurement builds its own `--release` first. Then [`doc/running-the-viewer.md`](running-the-viewer.md), [`doc/environment.md`](environment.md) |
| runs an instrument that is not a §2 gate | [`doc/verify.md`](verify.md) — `deny`, the fuzzers, callgrind, the cross-target checks, the census examples, AT-SPI |
| fuzzes — a campaign, a new target, or an artefact | [`doc/verify.md`](verify.md)'s fuzz block: `tools/fuzz.sh <target>` for one run and the campaign recipe beneath the targets' lines — built `-s none`, each target behind the heavy-walk lock and `tools/bounded.sh`, `-rss_limit_mb` and `-max_total_time` as the line states, a scratch corpus first and `-artifact_prefix` into `scratchpad/r<round>/`, because `fuzz/corpus` and `fuzz/artifacts` are the main checkout's (ADR 1423). A target is seeded before it is fuzzed — `flock /home/AI/heavy-walk.lock fuzz/seeds.sh <root> <target>`, its recipe the script's `case` arm — and a run is read as libFuzzer's `INITED` coverage beside its `DONE`, because the difference is what the run bought on top of the seeds (ADRs 0742, 0747); before a campaign, `fuzz/seeds.sh check <target>` behind the lock says whether the corpus on disk is stale against fresh seeds and writes nothing there (ADR 1559). A new target owes its `doc/verify.md` line and its arm in `fuzz/seeds.sh`, which `tools/conformance/tests/fuzz_workspace.rs` checks with the fuzz lock (ADR 1439); a finding owes the fix, a regression test under the crate it lands in, and the artefact named by its hash where the fix is argued, which is how `tools/state.sh main-checkout` knows it was read; `tools/state.sh fuzz` prints what the disk holds |
| asks about JavaScript, or a script a document carries | [`doc/rfc/0008`](rfc/0008-a-script-is-a-document-acting-on-its-reader.md) — **accepted by the owner** (`doc/questions/A193`): it is built in its section 11's order, Tier 0 first, and what of it is built is what `doc/state-of-play.md` says, never what the RFC proposes. The gate a change to `pdf-script` leaves green is the Tier 1 column, `--features engine --test script_corpus` behind the lock, whose ceilings are its own constants and whose `held:` line `tools/state.sh scripts` prints (ADR 1625) |
| asks whether an exclusion should lift — multimedia and 3D, XFA, authoring, scripts — or what printing still owes | [`doc/rfc/0009`](rfc/0009-what-each-exclusion-protects-and-what-lifting-it-would-mean.md) — **proposed, and `doc/questions/Q254` is open**: it prices each of `CLAUDE.md`'s exclusions and changes none; nothing of it is built until the owner answers |
| leaves something on the owner's disk — an ignored file, a stale corpus, an artefact read | [`doc/environment.md`](environment.md)'s *After a merge*, the commands the owner runs in the main checkout, and `tools/state.sh main-checkout`, which says which of them has anything to do (ADR 1440) — first the owner's answers on that disk that no commit holds yet, each dated, and the questions still open once they are counted, which `tools/batch.sh check` repeats (ADR 1588), and last **the owner's list**, everything it found to do numbered once in the order a person does it, each with its command or its files (ADR 1601); the re-seed command for every unseeded or stale corpus among them (ADR 1575) — and what CI's last run on `main` says, job by job, read with no token (ADR 1563) |
| looks for where something lives | [`doc/crate-map.md`](crate-map.md), [`doc/PLAN.md`](PLAN.md) |
| asks *when* something landed | [`doc/history/`](history/README.md), one file per round from 446 on, and [`doc/history.md`](history.md) for the rows before it — that is the only place session bookkeeping goes |

---

## The texts under `doc/md/`, and the one rule that is not about code

ISO 32000-2's sentences are quoted verbatim and the conformance gate checks every one of them
against `doc/md/`. **Every other specification text on this disk is held as licensed to a single
reader: cite the clause or the section and paraphrase, never quote — in a code comment as much as
in a document — and nothing of it is committed** (`/doc/*.pdf` and `/doc/md` are ignored; what is
tracked is the encrypted `doc/specifications.zip`). ADR 0187 is the position. ISO 19444-1 and the
Adobe XFDF 3.0 text it was made from, the two texts this tree has of the format §12.7.8 names and
defines nowhere, are held under it (ADR 1297), and
the three ETSI texts are stricter still — their notice permits no reproduction in any form, so
nothing from them appears between quotation marks or after a `>` (ADR 1085).
[`doc/third-party-data.md`](third-party-data.md) states it per text, with where each came from.

---

## Traps — read the index, open the group a line bites in

Each trap is a mistake somebody actually made in this tree, and the whole of it — the incident, the
evidence and the argument — is in one of the group files, grouped by **what a round is doing**.

**[`doc/traps/README.md`](traps/README.md) is the index and it is what a round reads**: one line per
trap, giving the position that springs it and the rule, plus the table of which group file is for
which kind of round. A round reads the condition column against what it is about to do and opens a
group file only where a line bites — which is the whole change ADR 1036 made, because the group a
round "is in" runs to hundreds of lines (`wc -l doc/traps/*.md`) and the line it needed is six of them. The index also states why
every trap keeps its number and resolves any citation by number in one hop, and
`cargo test -p conformance --test traps` holds every row to an entry of its number in the group file
it names, and every entry to its row (ADR 1379). A trap or a habit is an incident with a rule
attached, so its story is never rewritten — but a *pointer* in it, a command, a section or a file it
tells a round to open, is kept current like any other, because a stale one sends a round to the
wrong place (ADR 1525).

A round that skips the trap its work is in repeats a mistake somebody paid for, and the index is how
it finds out which one that is. `tools/state.sh traps` counts the index's rows, the group files' lengths and which
traps the rounds' records cite most, so none of those is written down here.

---

## Habits these sessions earned

**One file per kind of work**, the way `doc/traps/` is grouped, and [`doc/habits.md`](habits.md)
is their index — it states what a habit is and what each keeps, and it keeps the six section
headings so that a citation naming one still resolves (ADR 0983). Open the one the round is about;
a habit is worth reading when you are about to do the thing it is about, which is why they are not
here.

| open this | about |
|---|---|
| [`doc/habits/reading-the-specification.md`](habits/reading-the-specification.md) | what a modal verb means, what a silence is and is not, when a claim about the standard decays — and what `doc/md/` is, which is the instrument all of it is read through |
| [`doc/habits/judging-against-other-implementations.md`](habits/judging-against-other-implementations.md) | what an agreement is evidence of, what a reference is being asked, when a measurement is of the instrument |
| [`doc/habits/tests-gates-and-reports.md`](habits/tests-gates-and-reports.md) | what discriminates, what a ratchet's direction means, what a suite of small scenes proves |
| [`doc/habits/the-ledger-and-claims-about-this-tree.md`](habits/the-ledger-and-claims-about-this-tree.md) | how a row, a comment or a todo file goes stale, and which greps find it |
| [`doc/habits/measuring.md`](habits/measuring.md) | A/B in one sitting, attribute by removing the suspect, and which number to quote for which change |
| [`doc/habits/code-bounds-and-dependencies.md`](habits/code-bounds-and-dependencies.md) | what a cache's key claims, what a clamp decides, what a dependency is in a position to break |

**Three of them bind every round rather than a particular kind of work**, and
`doc/todo/02-every-round.md` §7 is where those live, beside the round they bind.
