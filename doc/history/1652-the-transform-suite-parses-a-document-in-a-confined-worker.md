# 1652 — The transform suite parses a document in a confined worker

Session 1652, slot 6 of batch one hundred and one, a "host" round; 2026-10-20. `doc/questions/A02`
("Yes, implement it "now"", 2026-09-06), built six weeks late; ADRs 2140 and 2141. No Q file. No
ledger row moved: no clause is implemented here.

**Built.** `quorra-transform` is a broker by default: it starts `quorra-transform-worker`, which
confines itself under `Profile::Interpreter` before it reads an argument and runs the binary's own
file, taken by `#[path]`, over `pdf_transform::confined::World`: `Forwarded` is a frame to the
broker for each named file, document (a descriptor, four at most, then bytes), password descriptor,
terminal question, line of output, pass of outputs, tool run and file beside the document; the broker
answers from `Here` through `Fences` drawn from its own reading of the arguments (ADR 2141, with
`Pattern::admits` and `admits_stem`). `--in-process` is the fallback, said aloud. `confined-transport`
gained `Host::send`/`receive` and `send_to_the_host`/`exchange_with_the_host` under the port's lock.
Also `tests/confined.rs`, CI's and the installer's bin lists, `tools/state.sh confined`,
`doc/todo/57` §4 and RFC 0002 §13 question 3, both closed.

**The premise held; the hypothesis did not, in four places.** The plan does not cross as data: a
second encoding of `Plan` and `Report` would restate every verb, so the plan's wire form is the
command line and the report crosses as printed bytes (ADR 2140). Sinks cannot be descriptors the
worker writes, since `RLIMIT_FSIZE` is 0; the broker writes each output. `--attach` needs no new
descriptor kind. And `tests/foreign_corpus.rs` calls `apply` in-process and never runs the binary,
so it could not see this change; a digest walk of the binary stands in for it below.

**Found.** `images --native` appends the codec's extension and a mask replaces it, so the first
fence refused real names (`tests/confined.rs`); a fetching `preserve` remedy runs a tool too
(`tests/archive.rs`); the worker's pool ignored `RAYON_NUM_THREADS` (ADR 1766), and honours it now.
**Unfinished**: no worker off Unix (`doc/todo/35`); past four gibibytes, `--in-process` only.

**Gates.** Tier 1 for both crates: fmt 0, clippy `-D warnings` 0, nextest 617 of 617.
`cargo test -p conformance`: 101, only `records`, over a sibling's 43-line record (1654).
`tools/state.sh confined`: 0, `view-awkward` killed 0, the worker's three tests passed; `docs`: 0.
One `--clock` hold: the transform gate 0, 53.6 pages/s against a floor of 40, before the thread fix;
`t3-foreign_corpus` 0, 203 documents. The worker's price, a second hold, 200 pages at 150 dpi on four
threads, interleaved: confined 3.749, 3.856, 4.440 s, in-process 4.199, 4.153, 4.987 s, one pages
digest for all six. HEAD's binary against HEAD plus this round's hunks alone (an export), SHA-256 of
status, stdout and every file, twelve verbs over every twelfth pdf.js document: 82 documents, 984
of 984 runs the same, 1 059 files. Against the whole worktree 4 renders of 492 differ from HEAD, each
the worker's equal under `--in-process`: a sibling's pixels. Duration: 6 400 s.
