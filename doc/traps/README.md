# The trap index

Status: **standing** — the index of `doc/traps/`, and the only part of it a round reads up front.

**One line per trap: the position that springs it, and the rule.** Read the *condition* column
against what this round is about to do. Where a line bites, open the group file named in the last
column and read that trap in full — the incident, the evidence and the argument are there, and they
are why anybody believes the rule (ADR 1036).

**This is an index and not a summary.** A line here is a lookup key; it is not the trap, it does not
carry the reason, and nothing may be decided from it alone. The rule against deleting a reason binds
this file hardest: a trap is an incident with a rule attached, the history *is* the lesson, and the
group files keep all of it (`CLAUDE.md`, "Where knowledge lives").

**Every trap keeps its number.** `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of
ADRs cite them by number, and an ADR is not edited to follow a file that moved underneath it
(ADR 0232 §2). This table resolves any such citation in one hop.

The five group files, and what each is the group *for*:

| file | the round it is for |
|---|---|
| [`pixels-and-rasterisers.md`](pixels-and-rasterisers.md) | can change a pixel — the interpreter's marks, any rasteriser, colour, a cross-backend scene |
| [`oracle-and-references.md`](oracle-and-references.md) | reads a verdict, diagnoses a page, invokes another renderer, or moves a tolerance |
| [`parsers-and-streams.md`](parsers-and-streams.md) | touches `pdf-syntax`, a filter, a font program, an image codec, or decides what to do with input it cannot fully handle |
| [`the-interactive-loop.md`](the-interactive-loop.md) | turns a press into a command, converts between the page's space, the display list's and the raster's, answers the core about a render, or waits on a toolkit's loop |
| [`instruments-and-reports.md`](instruments-and-reports.md) | runs a gate, believes a number, adds a report, sweeps for a defect, or puts a process under a limit |

Each group file also carries the standing facts about its own area, beside the traps about the same
machinery, and those are not indexed here.

## The index

| # | you are in a position to spring it when | and the rule is | group |
|---|---|---|---|
| 1 | a change of yours can put a mark on a page — **any** such change, however small the diff looked | the metrics lie; render the page and look at it, because no count can see a font that loaded and drew garbage, a page upside down, or a gradient that came out opaque | pixels |
| 2 | you compose a transform into a paint, a gradient, an image or a stroke width | a paint is positioned in the *path's* space and both backends apply the drawing transform to it already, so composing it yourself applies it twice | pixels |
| 3 | you invoke another renderer, or read an answer one gave | check what question the reference was actually asked — page box, print flag, how the answer was collected — before reading it as a verdict | oracle |
| 4 | you write a test out of a hand-built PDF fragment | test against real documents too; the fragment exercises the code you were thinking about and the corpus exercises the ones you were not | parsers |
| 5 | you implement part of a feature, or handle an input you cannot fully support | unsupported input must stay loud; a silent fallback that renders something plausible is the failure mode that reports nothing, and it hides best *inside* a partly-implemented feature | parsers |
| 6 | you convert a colour, or add a route from one space to another | one conversion, in one place: `ColourSpace::to_rgb` and `colour::xyz_d50_to_srgb`, and the standard usually ranks two answers rather than stating one | pixels |
| 7 | you silence a lint | `#[expect(..., reason = "...")]`, never `#[allow]` — an `expect` errors when it stops being necessary and an `allow` hides that forever | instruments |
| 8 | you conclude something from what the corpus does or does not contain | a corpus finds what documents contain, not what the standard says; measure unreachability by *breaking the rule* and watching a gate move, never with the instrument under test | parsers |
| 9 | two references agree and you are about to call that evidence | they can agree because they share code, or because they share a *gap*; read the list of ways it fails rather than the count of them | oracle |
| 10 | you run a `--profile gates --test` line, or any test that decodes JBIG2, CCITT or JPX | the sandbox worker is a separate binary and Cargo will not rebuild it for you; a missing worker and a stale one look nothing alike | instruments |
| 10a | you read an oracle verdict, or change what a reference invocation asks for | a cached reference render is a fourth stale thing; the hit rate is the tell, and a flag not in the key was not passed to the renderer either | instruments |
| 10b | you add a **new module file** and then measure from a release binary | Cargo's release fingerprint will not know the file exists; `touch` each changed crate's `src/lib.rs` before either arm of a two-revision comparison | instruments |
| 11 | you add a report, or a census, or decide when one fires | a report is only as good as the condition it fires on; derive the condition from the clause, print what it matched, and cost it in gated pages | instruments |
| 12 | you read a verdict against a bound derived from the references | where two references agree closely the bound is tighter than eight-bit arithmetic; write the clause's closed form down rather than tuning towards a reference | oracle |
| 12a | you convert a point between the page's space, the display list's and the raster's | the display list's space is not the raster's — the flip is in `TargetSpec::for_page`, about the *page's* height, and a doc comment said otherwise for seventy-five sessions | interactive loop |
| 12b | you judge a backend by a suite of fixtures | a suite of small scenes tests small scenes; ask what *size* every scene is and what parameter every one of them leaves at its default | pixels |
| 12c | a dependency reports a failure through a handler rather than a return value | the handler has an ordering you have to obey; make what it was told into a value the caller can take, and let a type hold the ordering | pixels |
| 13 | you sweep, grep or census for a class of defect and it comes back clean | plant the defect back and confirm the sweep names it; an uncalibrated sweep's clean answer is a sentence about the grep, not about the tree | instruments |
| 14 | you implement a clause that names a **region**, and your gate rasterises that same region | a target that *is* the region cannot tell you whether you applied it; ask whether the gate could distinguish the two answers, and rasterise something bigger once by hand | pixels |
| 15 | you run a sweep binary by its path rather than through cargo, in a worktree | the binary carries the tree it was **built from**; take the path from `cargo metadata`, and the tell is that nothing moves when you re-run after an edit | instruments |
| 16 | you compare two numbers out of one tree and read them as two interpretations | a gate can measure a program the build did not finish producing; ask what the two *programs* were before asking what the two readings are | instruments |
| 17 | you are about to write that a toolkit cannot do what a clause asks | a widget list is a catalogue, not a statement of capability; say what the clause asks for in its own nouns and ask whether the toolkit will *compose* them | interactive loop |
| 18 | you put a process under a limit — `RLIMIT_*`, seccomp, a confinement | ask what the limit does to the channel that reports the limit being hit; a diagnosis that arrives as silence is read as a different failure | instruments |
| 19 | a host places a widget whose geometry the **document** decided | an answer that feeds back into the viewport closes a loop the core cannot see; the host's container must not measure what the file sized | interactive loop |
| 20 | a host abandons a draw it started | `Rendered::Failed` also means *never ask me again*, which is true of a rasteriser's refusal and false of a host's decision; answer nothing at all for what the host decided | interactive loop |
| 21 | you read a latency figure taken through a toolkit's main loop | a poll is a request for a turn of that loop, so the interval is a floor and the ceiling belongs to somebody else; the tell is a wait far larger than the work | interactive loop |
| 22 | a press reaches a shared table through more than one path in one host | a key table is only as level as its narrowest entrance; count the call sites before believing a host is level, and press the key at a real window | interactive loop |
| 23 | you rely on `--all` or `--workspace` to have covered something | those mean *every package in **this** workspace*; `fuzz/` declares its own, so every workspace-scoped command owes a second invocation naming that manifest — and lint levels stop at the same boundary | instruments |
| 24 | you read a fuzz run's exit status | it answers *did it crash*, never *did it run*; read libFuzzer's `INITED` and its final `cov:`/`ft:`, and remember the corpus directory is gitignored | instruments |
| 25 | an instrument's population is a hand-written list of names | it can name a thing that never existed, and finding nothing there prints a tick; derive the population from a manifest, cargo, or the tree | instruments |
| 26 | you rank two **pages** by a worst-tile figure | the tiles are laid from the raster's origin, so one mark measured whole and the same mark straddling a boundary differ by up to 1.77; print `worst_tile_at` first | oracle |
| 27 | you assert that an error, refusal or report *contains* some text | the assertion is only as good as what it excludes; ask what else this input can produce here and whether the assertion would accept it | instruments |
| 28 | you write or read a recovery, a fallback, or the comment above one | the guard states when the recovery is *needed* and the comment states when it is *right*; the round that writes one owes the file where those two disagree | parsers |
| 29 | you lift a bound in a scratch build to find out who reaches it | the lifting moves every site that *derives* from the constant, and the experiment needs a control that must stop — a document known to be finite and deep | instruments |
| 30 | you index a collection whose producer is parallel — a sink, a report, a listing | it hands its outputs back in the order they were *opened*; look an output up by its name, and panic naming every output there was | instruments |
| 31 | code that **opens** a file is linked into a confined worker | `SECCOMP_RET_KILL_PROCESS` does not return an `Err`, so the careful `else` branch never runs; tell the process before the confinement that the disk is not there | instruments |
| 32 | a confined worker **owns** a descriptor and lets it go | `OwnedFd::drop` calls `fcntl` under a check compiled per build, so release survives and every debug worker dies; and a probe for it must issue the call by name | instruments |
| 33 | you assert on a counter to prove how often something runs | a counter of what was *produced* cannot see a cost paid in validation; wrap the expensive call and count *it* | instruments |
| 34 | a gate guards a measurement with a calibration figure | the guard has to be made of the same stuff as the figure — a first pass in a fresh process is not the quickest of fifty warm ones | instruments |
| 35 | you band, budget or report a process's resident memory | `VmHWM` is mostly file-backed library pages and the kernel decides how many are resident; say which part of it you mean | instruments |
| 36 | you take a timing figure on a machine other rounds are using | a neighbour takes the *level* without ever queueing, and the run-queue wait explains only the *tail* — and `/proc/self` is the wrong thread; ask `/proc/thread-self` | instruments |
| 37 | you claim nothing changed on the strength of a digest | an interpretation is what was drawn **and** what was said; a digest that hashes one of them answers "did anything change" with half a fact | instruments |
| 38 | you pick a resource bound, or read one that already exists | ask whether the standard, or data the standard requires a reader to carry, states a number the bound must be at least as large as — and make the cut say so by name | parsers |
| 39 | a signal, flag or warning in an instrument fires on every run | it has stopped being a signal and it looks like caution; check what it is computed from before trusting that it means anything | instruments |
| 40 | you read a function that **refuses** something, with a good sentence saying why | the capability it says does not exist may be forty lines above it; read a refusing function's neighbours for the construction it denies | pixels |

**Two are not optional for the round they are about.** If this round can change a pixel, **trap 1**
is the one that has paid every session since the tenth. If this round adds a report, **trap 11** is
what stops it firing on a condition the clause does not state.
