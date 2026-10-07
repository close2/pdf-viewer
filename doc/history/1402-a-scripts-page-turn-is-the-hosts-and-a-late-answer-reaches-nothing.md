# 1402 — A script's page turn is the host's, and a late answer reaches nothing

Script slot of batch sixty-three. ADRs 1640, 1641; no ledger row moves (§12.6.4.17 stays
`out-of-scope`, `Q286`), no question.
**Premises.** All three held: `question_withdrawn` at `client.rs:527`, no withdrawal event in
`viewer-core`, `pageNum` read-only at `bridge.rs:173`. One did not hold beneath them: ADR 1628's
"that late answer is dropped" was true only for an empty queue — `answer` neither ran the expiry
first nor asked whether the host had taken the question, so a press on a withdrawn card could
answer the question a queued script asked since (ADR 1641 section 1, now a test).
**Page turn (ADR 1640).** `this.pageNum = n` records `ScriptEdit::GoTo` (wire tag 6, `VERSION` 5),
read back for the run, the latest of a run kept; `ViewState::take_page_request` holds it for the
host, as `setFocus` is held. Not finite or outside `0..numPages`: a note, no turn, no throw. A field
event now reads its widget's `/P` page, so `this.pageNum++` counts from where the person is. The
reference's two examples are fixtures.
**Withdrawal (ADR 1641).** `answer` expires first and resumes only a taken question;
`question_withdrawn` is true only for a taken one; `question_deadline` gives a window the moment.
Slot 2's `Event::ScriptQuestionWithdrawn { document }` and `ScriptAsks::withdrawn` match.
**The column's 9 541 throws, by cause** (runs / documents; todo 56 has the classes, the column's
new `one_document` test prints a document's throws with scripts and report sentences): a function
no file defines 8 908 / 7 (`TFMC` alone 8 892 / 1: one `/C` on hundreds of fields, every commit);
the author's folder-level library 40 / 5; a file's own slip 410 / 5 (`defaultValue` 223, `getField`
of a deleted name 185, `AFSpecial_FormatEx` 2); a name with trailing spaces or a trailing period
about 166 / 9, one an Acrobat-generated calculation — evidence only; a root field `/Fields` omits
(inside one PDFBOX form); `fn.arguments` on a sloppy function, which ECMA-262 makes throw and Boa
follows, 8 / 4; Tier 1 gaps of ours 8 / 3 (`AFExactMatch`, `style`, `cursor`). No Tier 2 member.
A reader sees each run as not run, one report sentence per field; the 5712688 form fills the
report's 256 sentences before its first throw is said.
**Per site** (threw, before → after, round 1395's record before): every site unchanged — Calculate
9 488, Keystroke 18, Validate 26, Format 2, Page(Open) 4, Library 1, WillPrint 1, DidPrint 1;
finished 11 308 → 11 311, finished refused 3 → 0 (the three `this.pageNum=` catches), ceiling to 0.
**Gates.** `rustfmt --check --edition 2024` on my 11 `.rs` files: 0. Clippy `-D warnings` 0:
`pdf-script` + `pdf-script-worker` with and without `engine`, `pdf-model`. Nextest: 119 / 119 with
the engine (19 runs in a row), 26 / 26 without, `pdf-model` 1 955 / 1 955; two earlier engine runs failed
`the_engines_library_runs_inside_the_filter` at load 13–14 and passed 6 alone and 19 combined at
load 3–6 (a timing failure under neighbours' builds). `cargo test -p conformance`: exit 101, two
tests on slot 5's in-flight §12.7.8.3 rows, none mine. Behind the lock: Tier 1 column 0 (134 s),
worker column 0 (140 s; 141 workers, 0 lost, 0 `SIGSYS`, count for count, about 1 080 s queued);
then in one hold (about 1 560 s queued) Tier 0's `script_corpus` 0 (40 s, 347 held, 0 moved) and the
final Tier 1 column 0 (103 s, ceilings 9 541 / 0 / 8 met, 20 860 runs).
