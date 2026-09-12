# 1015 — The walks that nobody ran, and the population that is now checked

Session 995. Status: **accepted**. The direction review's first next step
(`doc/reviews/984-direction-and-boundaries.md`, Finding 3; ADR 1005 §3 is the proposal), taken
whole: every `#[ignore]`d test file in the tree is either a line of `doc/todo/02` §2's sequence
or says, in a line a check can read, why it is not — and the claim is a gate rather than a grep.
Amends `doc/todo/02` §2 (six gate lines, two bullets, two map rows), `tools/state.sh` (four
sections), `tools/conformance/tests/state_sections.rs` (a second test), `doc/state-of-play.md`
and `doc/verify.md`; puts one `// no sandbox worker:` line in each of five test files.

## 1. What was found, measured rather than restated

The review matched every test file carrying `#[ignore]` against §2 and the script and listed
seven in no gate line. Measured here by the check this round built — an attribute at the start of
a line, over every tracked `.rs` under `crates/` and `tools/` — the number is **six**, and the
seventh is worth a sentence: `crates/viewer-confined/tests/confined.rs` carries no `#[ignore]`
attribute at all. What it carries is a doc comment reading "[i]t was `#[ignore]`d when it was
written, because it failed", above a test ADR 0888 un-ignored. A grep for a name finds sentences
about the name (trap 11's sixth instance, `tools/state.sh hosts`'s lesson), and the reason the
matcher here asks for the attribute where an attribute is — first on its line — is that the
review's own instrument was the one that miscounted.

The six, each run alone under `tools/bounded.sh` on a machine with siblings building (load 7–9),
every one exit 0, every one a count and none a clock:

| file | what it holds | cost, as `bounded.sh` printed it | decision |
|---|---|---|---|
| `pdf-archive/tests/corpus.rs` | the validator against every veraPDF witness, per target; **reports and does not assert** — its header says so and says why | 2 s, 0.12 GiB | gate line |
| `pdf-transform/tests/archive_corpus.rs` | conforming in → conforming out, under no authorisation and under all; nothing errors; a fix count above zero | 6 s, 0.17 GiB | gate line |
| `pdf-model/tests/save_round_trip.rs` | §7.5.6's update over the corpus, read back by three readers; ratchets since ADR 1011 | ADR 1011's figures | the line ADR 1011 §2 owed |
| `pdf-model/tests/actions.rs` | §12.6.3's page-scoped triggers counted over the corpus and held **in both directions**; §12.6.4's embedded go-to opened and its pages counted | 1 s, 0.03 GiB | gate line |
| `pdf-syntax/tests/on_disk.rs` | every corpus document read from disk and from memory, every object the cross-reference names compared; fails on one object or under nine hundred documents | 2 s, 0.32 GiB | gate line |
| `viewer-confined/tests/awkward_classes.rs` | `pdf-view-worker` over a document of each awkward class from every corpus on the disk; fails on a death | 12 s, 4.65 GiB | gate line, moved from `doc/verify.md` |

**None is excused.** The check supports an excuse — a line beginning `// not a gate:` with a
reason — and after this round no test file in the tree carries one, which is the right state
for the six and the wrong shape to leave unexercised; §3 says how the excuse was calibrated.

Two of the six had a reason on record for being out. `on_disk.rs` said "not a gate: run it after
a change to how the file is read" — and `pdf-syntax` is one of the seven crates a change to
which runs the whole sequence (§2 rule 2), so the condition under which it should run is the
condition under which everything runs, and a two-second walk that fails on one object is exactly
what rule 2 is for. `awkward_classes.rs` was `doc/verify.md`'s on the argument that `pdf-vfs`'s
read walk gates the same class of defect over more questions. That is true of the *filter*, which
the two confined programs share, and not of the *program*: a system call reached from code only
`pdf-view-worker` links is caught by this walk alone, and the process it protects is the one a
person reads pages in. Twelve seconds and a `--bins` line (trap 10, a fourth time) is what the
sequence pays for that; `doc/verify.md` keeps the walk's calibration — 28 deaths in six classes
with `no_machine_fonts()` taken out — which is the fact that shows the line can fail.

The other four had no reason on record at all: `#[ignore]` for a submodule's size, which every
corpus gate in the sequence shares, and then nothing. The validator's walk had the most
consequential silence: `doc/state-of-play.md` said `tools/state.sh` printed the comparison, and
the script had no section that opened a veraPDF file. **A change to `pdf-archive` ran "the core"
and nothing else** — no row in the map, no line in the sequence — for the whole life of a
31,000-line crate.

## 2. Where the lines went, and what each is held to

`doc/todo/02` §2's block, in the style of its other lines:

- after `foreign_corpus`, the two veraPDF walks — the validator first, because the converter's
  contract is that validator run twice, before and after;
- after `read_corpus`, `cargo build --profile gates -p viewer-confined --bins` and then
  `awkward_classes`, for `pdf-vfs`'s reason: a `--profile gates --test` line builds one target,
  and the worker beside it would otherwise be whatever an earlier round left;
- after `xmp`, the save round-trip (where ADR 1011 §2 placed it and `tools/state.sh save`
  already ran it), then `actions`, then `on_disk`.

`tools/state.sh` gained `archive`, `confined`, `actions` and `on-disk`, in its cost order, each
filter keeping the gate's own summary lines and each walk under `tools/bounded.sh`; the
`archive` section prints the validator's heading, column names and `all` row per target — `over`
is the column that matters, and its own legend says every one of it is a question for
`doc/pdfa/` before it is a bug — and the converter's summary line per target with what a user
who answers nothing gets. `tools/conformance/tests/state_sections.rs`'s first test, session
990's, is what holds the block and the script to the same list, and it is why the section and
the line arrived together.

**Every new line answers `sandbox_gates.rs`**, and the answer is the same for all five: a
`// no sandbox worker:` line, because none of them interprets a page. The validator and the
converter read the object graph and §7.4's standard filters (`decoded_stream_data` stops at the
image codecs; `pdf_archive` reaches `pdf_model::jpeg`, `inline_image::scan` and `content::reader`,
none of which decodes an image); `actions` reads `/AA` dictionaries; `on_disk` is in a crate with
no decoder to reach. `awkward_classes` is the interesting one: `pdf-view-worker` sets
`Isolation::InProcess` and decodes JBIG2, JPEG 2000 and CCITT inside the confined process, since
a confined process may spawn nothing — so there is no `pdf-sandbox-worker` whose absence could
move what the walk holds, and the program that has to be built beside it is its own.

**The validator's line is a report in a gate's clothing, and the map row says so.** Its test
asserts nothing about the sweep — the header states that turning an adjudicated expectation into
a gate is a later and deliberate step — so what fails the line today is a panic or an unreadable
witness, and `over` is read by the round. On every target this round `over` is 0 and `missed` is
1 (PDF/A-2b), and a ratchet holding those two is one assertion in a file `crates/pdf-archive/`'s
own round owns; the map row for `pdf-archive` names the column to read until it does.

## 3. The population as a checked claim, and its calibration

The second test in `state_sections.rs` reads the index rather than a list — `git ls-files` over
`crates/*.rs` and `tools/*.rs`, for `workspaces.rs`'s reason that a worktree round's submodules
are symlinks — keeps every file with an `#[ignore` attribute at the start of a trimmed line, and
sorts each into one of five cases:

| the file | is named by a §2 line | carries `// not a gate:` | result |
|---|---|---|---|
| `<members>/<package>/tests/<target>.rs` | yes | no | counted |
| the same | yes | yes | **fails**: the excuse has outlived its fact |
| the same | no | yes | printed, with its reason |
| the same | no | no | **fails, by name**, with the `-p … --test …` the line would use and the attribute's own reason |
| a `src/` file, or a module under `tests/` | — | — | printed: no `--test` line can name it, and the excuse is owed there too |

and a sixth, outside the index: an **untracked** file carrying the attribute is printed with the
sentence that it will fail the check when it is added. That is a deliberate reading of what
"tracked" means in a tree with parallel rounds in it — an untracked test file is somebody's
mid-edit, and a red gate over a neighbour's unfinished work is the shape ADR 0992 says to report
rather than repair — while a file that has been added is the tree's and fails on the day.

Two things are printed rather than failed, and each has one instance today. The `src/` case is
`crates/pdf-model/src/signature.rs`'s priced measurement of the digest window, whose doc comment
says why it is ignored and which is in a crate this round did not own; the line it owes is
`// not a gate: a measurement of the digest window, printed, not a check (ADR 0812)`, and once it
carries one the print becomes a failure by changing one arm. The untracked case is a sibling's
`crates/pdf-model/tests/raster_golden.rs`, Finding 4's self-golden, which will need a §2 line
and a `state.sh` section at the merge — and the check said so, unprompted, on its first run.

**Calibrated by planting, four ways** (trap 13). A file `crates/pdf-syntax/tests/planted_walk.rs`
with one ignored test: untracked, the check printed it and passed; `git add`ed, it **failed
naming `planted_walk`** with the line it would need; with `// not a gate: it is a plant` added, it
passed and printed the excuse. Then the excuse line planted in `on_disk.rs`, which the sequence
now names: **failed as spent**, naming the file and the reason. Plant and lines removed, the
index restored, and the check passed. And before any line was added to §2, the same test run
against the tree named exactly the review's six — which is the calibration against the defect
itself rather than against a plant.

Two constants are spelled in two pieces (`concat!("#[", "ignore")`), and the file skips itself
by name as well: a scanner reads its own source, and this file is in the population it scans
(`doc/habits/tests-gates-and-reports.md`, ADR 1010's three self-matches).

## 4. Two lines that were owed, and why they were

ADR 1011 §2 stated the save round-trip's §2 line and left it for the owner to add;
`tools/conformance/tests/questions.rs` was one `rustfmt` hunk from clean at the `date_is_a_date`
closure. Both landed in `main` that way because the main checkout's copies were not writable by
this account. The line is in §2 now, and the hunk is formatted — and formatting it pushed
`every_answer_says_what_it_left_open` to 101 lines under clippy's `too_many_lines`, the second
`rustfmt` hunk (line 405's supersession comparison) having grown from two lines to six. The
comparison is a function of its own, `comes_after`, which is why the formatted file lints: the
two instruments disagreed about one expression and the fix is to have neither lay it out.

## 5. Gates

`doc/history/995` has the figures as printed. Nothing this round touched draws a pixel, so the
core four, both `fuzz/` lines and `cargo test -p conformance` are what §2 owes it; each of the
six lines added was run alone as well, and its figures are in the history file rather than here.
