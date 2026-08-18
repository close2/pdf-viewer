# What every round does

Status: **standing** — this one is never done.
Priority: 02

A "round" here is one session's worth of work. `CLAUDE.md`'s two tracks decide *what* it
contains; this file is what it does around that, in order.

**This file states commands and rules, never counts.** `tools/state.sh` prints the counts, and
that separation is the point: a round that can read a gate's number here can write it down
without running the gate. Every figure this file used to carry went stale at least once (ADR
0281).

## 1. Take from both tracks

Demand-driven is what the corpus and the oracle name (todos `10`–`29`); spec-driven is the
ledger's `reported` rows and the notes on its `partial` ones (todos `00`–`09`). A project
running only the first finishes when the corpus goes quiet, which can happen with much of the
standard unimplemented and nothing able to say which parts; one running only the second ships
features no file exercises. This is a principle-5 rule, not a suggestion.

**But the map is not the territory.** Four of the six findings in the ten sessions from the
hundred-and-twentieth were on no list at all: a `shall` hiding behind a silence about artwork (ADR
0109), a clause with two populations where the row named one (0110), a malformed optional entry
that erased a font (0111), and a font cache keyed by a name that drew wrong glyphs in silence for
thirty-one sessions (0115). None was `silent`, none was `reported`, and no gate could see the
last. Three were found by reading the clause beside the code; the fourth by measuring something
else.

**The six shapes a refusal takes when it has outlived its reason** — a reason that names a
vocabulary, a reason that names an architecture, a capability that arrived and announced nothing,
a capability that reached the crate and never reached the program, a row that would have
survived the capability arriving, and a row *corrected* by naming the capability that arrived while
the entry it turns on stayed unread — are in [`../habits.md`](../habits.md)'s ledger section, beside
the sweeps that find each. They are the highest-yield reading this project has.

## 2. Run the gates that can see what you touched

**The whole sequence is below, and the map after it says which of it a given change needs.** Two
rules bound the map and are the whole of its safety: **the full sequence runs every fifth round,
and on any round that can change a pixel**; and the merge rule two paragraphs down is not relaxed
at all. `tools/round.sh` says whether this is a fifth round.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets      # must be silent of lints
cargo nextest run --workspace
cargo test --workspace --doc                # the one doctest nextest does not run
cargo build --profile gates -p pdf-sandbox --bins   # trap 10: Cargo will not do this for you
cargo test  --profile gates -p pdf-model      --test corpus          -- --ignored --nocapture
cargo build --profile gates -p hayro-compare --bin pdfref-hayro      # trap 10 again, see below
cargo test  --profile gates -p pdf-model      --test oracle          -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test text_extraction -- --ignored --nocapture   # three gates
cargo test  --profile gates -p viewer-core    --test selection_census -- --ignored --nocapture
cargo test  --profile gates -p viewer-core    --test accessibility_census -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test dates           -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test xmp             -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test jpeg2000        -- --nocapture
cargo test  --profile gates -p render-quorra  --test corpus          -- --ignored --nocapture
cargo test -p conformance -- --nocapture
```

`tools/state.sh` runs the same sequence and prints each gate's own summary lines; run that when
what you want is the state, and run the list above when what you want is a gate to fail. Either
way the numbers come off the run, never off a document.

**This section owns the sequence, and nothing else states it.** `doc/HANDOVER.md` used to carry a
second copy under "Verify it" and the two drifted — one said 1369 tests where the gate printed
1371, and it never listed `render-quorra`'s corpus gate at all (ADR 0232 §4). Two documents stating
one command is how they drift, so one owns it.

### The change → gate map

**The first four lines are the core and every round runs them**, whatever it touched — they are
about a tenth of the sequence's cost and they are the only thing that sees a lint, a broken
doctest or a test somewhere else in the workspace. The rest is chosen by what the change can
reach, and *reach* is the crate graph rather than the file's own crate:

| a change in | is under | so run, beyond the core |
|---|---|---|
| `pdf-render`, `pdf-syntax`, `pdf-font`, `pdf-model`, `pdf-spec`, `pdf-sandbox`, `render-cpu` | everything | **everything** — these are what draws the page and what every gate rasterises with |
| `render-quorra` | the third rasteriser only | the quorra gate, and its second coverage lane where the change is a quorra release or the zoom path |
| `render-gpu` | no gate at all | the workspace tests are the only judge (`headless_gpu`); say so, and consider `doc/verify.md`'s cross-backend runs |
| `viewer-core`, `viewer-accessibility` | the two censuses | `selection_census`, `accessibility_census` |
| `viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-ffi`, `viewer-host`, `viewer-confined` | no corpus gate | the core, which builds and tests them; §5 rebuilds what a person runs |
| `tools/conformance`, `doc/conformance/ledger.toml`, a doc comment citing a clause | the conformance gate | `cargo test -p conformance` |
| `raster-compare`, `test-scenes`, `pdfref` | whichever gate names them | the core, plus the gate whose harness they are — `raster-compare` and `pdfref` are the oracle's and quorra's |
| **documents only** (`doc/`, `CLAUDE.md`, a `tools/*.sh`) | nothing the gates rasterise | the core, **and `cargo test -p conformance`**, which reads citations and quotations out of the tree; plus `--bin quotations` and `--bin pointers` where the change moved a document or a pointer |

Three things the map does not license:

- **A round that can change a pixel runs everything.** That is any change to the crates in the
  first row, and it is not a judgement about how small the diff looked: trap 1's whole subject is
  that a change nobody expected to draw differently did.
- **Every fifth round runs everything**, whatever it touched, because a map is a claim about the
  crate graph and a claim decays. `tools/round.sh` says which round this is.
- **A merge runs everything, always**, and the paragraph below is not relaxed by any of this.

**A merge is a round of its own, and it runs this sequence on `main`.** Green in a worktree
establishes nothing about `main`: a parallel round's gates are the truth about a tree that
branched before its neighbours' files existed. The proof is in this tree — eleven
`clippy::pedantic` warnings lived on `main` for five rounds while four rounds truthfully recorded
the lint run silent in their own worktrees, and two more parallel rounds broke the quotation gate
the same way, on merge (`doc/history.md`'s 455–484 block summary). So whoever merges a worktree
round into `main` owns this section for the merged result, before the next round branches from
it. There is no exemption for a merge that "only touched docs" — the five-round breakage was in
an example file nobody thought about either.

**This sequence used to be more than twice as slow**, until the three-hundred-and-eighty-fifth
session measured every step of it and changed four things; ADR 0222 has the table and the
argument, and `Cargo.toml`'s profiles carry the reasoning beside the settings. Five notes bind
here:

- **`--profile gates`, not `--release`.** Release-grade optimisation with cheap linking, because a
  fat whole-graph link *per gate binary* was most of a round. All eight gates were run under
  both profiles and their output compared line by line — every verdict, every page, every
  citation, **every field identical**. `--release` still works and is still the same gate; it is
  only slower. **`[profile.release]` did not change**, and §5's binaries are still built with it.
- **`cargo nextest` is a user-local install** — `cargo install cargo-nextest --locked`, or the
  prebuilt from `https://get.nexte.st/latest/linux` into `~/.cargo/bin`. Without it,
  `cargo test --workspace` is exactly the same gate at three times the wall clock, and that is
  what CI runs. `nextest` skips doctests, which is why the line after it is there.
- **A round that adds a test writes down nothing about the count.** This file carried the test
  count and its arithmetic for dozens of rounds and each was separately wrong at least once — the
  sharpest instance being a round that copied the gate's own number into the line it was told to
  update and left the *sum* beside it untouched, seven rounds behind. Two rules survive that, and
  they are about running rather than about writing: a number is current only for a round that ran
  the gate **last**, after its final edit; and a round that writes a number it did not watch print
  writes the previous round's.
- **One of these commands runs a C compiler**, and it is the only gate in this sequence that does.
  `viewer-ffi::a_c_program_drives_the_abi` builds `crates/viewer-ffi/c/open_a_page.c` against the
  crate's own header with `-Wall -Wextra -Werror`, links it against the `cdylib` — which it asks
  cargo to build, because `cargo test` does not — and runs it on a document. It **skips** where
  there is no `cc` or `gcc`, printing why: a machine without a C compiler cannot run it, and
  failing there would make the gate a coin toss. CI has one, so on CI it is not a skip.
  **A C++ compiler runs too and is not a gate**: `clippy --workspace` and `test --workspace` build
  `viewer-qt`, whose `build.rs` compiles `cxx-qt`'s generated bridge, and on a **cold** build that
  prints `cargo:warning=` lines beginning `viewer-qt@0.1.0:` — `-Wmaybe-uninitialized` inside
  `rust::cxxbridge1::Vec<T>::Vec()`. They are **gcc's, about generated code, and not clippy
  lints**, and a warm build prints none of them, which is exactly what makes them easy to read as
  a regression.
- **`-- --ignored` runs every ignored test in the binary, which is not always what the line wants.**
  It is a switch on the whole binary rather than a filter, so a test that carries `#[ignore]` to mean
  *run me explicitly* is run by every gate line that names its file. The oracle's binary held one for
  thirty-nine rounds — a derivation whose own doc comment says it "is not itself a gate" — and the two
  walked the corpus side by side under `rayon`, which doubled the line's wall clock and inflated the
  per-page spans it prints (ADR 0282). **The rule that came out of it is where the fix goes, not what
  the fix was**: a test in a gate binary that must not run in the gate declines *by itself*, because
  an invocation can be copied without its guard and a test cannot be run without itself. Nothing here
  changes when one is added.
- **The `selection_census` line is the one that clicks**, and it is here rather than in
  `doc/verify.md` because two of its three properties are exact and it is what catches a defect in
  the loop from a press to a selection — which is the loop `doc/traps/the-interactive-loop.md`'s trap 12a is about and
  which nothing gated until the five-hundred-and-eighty-sixth session. Its *drag fraction* is
  printed and **not** ratcheted, by `doc/todo/05`'s standing rule; what fails the line is a
  selection that is not the interpreter's readback, a caret whose own point lands somewhere else,
  or a panic. Six seconds with a warm extraction cache, which it shares with the line above it.
- **The `accessibility_census` line is a *ratchet* and says so**, which is the third of ADR 0323's
  instruments and the shape it was designed with: no other implementation puts a comparable tree on
  AT-SPI, so there is nobody to disagree with us and a count that cannot fall is what is honestly
  available. It entered this list in the five-hundred-and-ninetieth session rather than in the one
  that built it (ADR 0425), on `doc/todo/05`'s own rule — the counts had to hold across rounds
  first, and they did. Twenty seconds. What fails the line is a capability count falling, a defect
  class growing, a panic, an untagged page given a structure it does not state, or a line whose
  characters disagree with its own text. **A tree without the `doc/pdf.js` submodule prints why it
  is not ratcheted instead of failing**, because a smaller population is the one reason a floor can
  break that is not a regression.
- **The `text_extraction` line is three gates and gains no line**, which is the other side of the same
  mechanism and is correct: `the_text_we_draw_agrees_with_pdfboxs_frozen_extraction` is a gate. It
  compares documents against the `PDFTextStripper` output Apache PDFBox checked in beside them — a
  *frozen* second reference, which cannot drift under this tree the way the machine's poppler can
  — and it costs a fraction of the pdf.js gate, because its reference is a file rather than one
  `pdftotext` invocation per document. That is what "earned a place" was supposed to mean. ADR
  0259.
- **`pdfref-hayro` is the oracle's fourth reading and nothing built it.** It is a *program*, found
  beside the running test binary, and its absence costs no verdict — `Reference::Hayro` never
  votes — but it is what a person looks at on a page the three references cannot settle. It
  existed under `target/release/` only because some earlier round happened to run
  `cargo build --release -p hayro-compare --bins`. Its line is placed *after* the corpus gate
  rather than in front of it, which is worth several seconds: the corpus gate compiles
  `pdf-model`'s rlib and its own test target in one graph, and `-p hayro-compare` on its own has
  nothing to overlap.

- **The quorra gate runs one of two coverage lanes, and the other one is a round's to ask for.**
  `PDFVIEWER_QUORRA_COVERAGE=gpu` points the same gate at the lane `viewer-ui` switches to past ten
  times magnification; paired with `PDFVIEWER_QUORRA_SCALE=4` it is the population that lane
  actually draws for. Both turn the ratchets off, and the run says so. **Two kinds of round owe
  this run**: one that takes a quorra release, because the release may be entirely inside a lane
  §2 does not exercise — which `74c4994d` was, and it took 24 refusals off that lane at 4× while
  moving nothing at all on the default one (ADR 0283) — and one that changes the zoom path.

**The fuzz targets are `doc/verify.md`'s**, and `tools/state.sh counts` says how many there are.
Three rules bind a round rather than a count:

- **A round that touches a parser runs the target that covers it**, and a round that touches
  `pdf-font`'s glyph-table repairs runs `sfnt` **with its corpus seeded**, because unseeded it
  never forms a table directory and tests nothing. Five of the targets need a seeded corpus;
  `doc/verify.md` says which.
- **A target is only as good as the code its binary contains, which is a question for `nm`.** The
  `page` target exists because `nm` found `pdf_model::interpret` — clauses 8, 9 and 11 — in one of
  the thirteen binaries that preceded it, and that one calls it on a page with no `/Resources`
  (ADR 0264).
- **`page`'s corpus is the expensive part, and the merge that reduces it has been spent.**
  libFuzzer's fork mode merges the corpus before it fuzzes, one execution per seed, and on the
  seeds `fuzz/seed_page.py` produces that merge had become most of every run's wall clock — one
  round spent fifty minutes inside it and got nothing back. `cargo fuzz cmin page` writes the
  reduced set back, and the five-hundred-and-ninety-third session ran it: **the corpus fell to
  about a quarter of its files and a seventh of its bytes, at no cost in coverage at all** — a
  `cmin` keeps exactly the distinct-coverage set, so the reduced corpus carries the same edges and
  the same features the whole one did — `cmin`'s own `MERGE-OUTER` line says so, and a fork-mode
  run over the reduced corpus reports the same two figures back, with no crash, timeout or OOM.
  `du -sh fuzz/corpus/page` and `ls | wc -l` are where the level is.

  **What it bought is less than this file expected, and the reason is worth having.** This bullet
  said a `cmin` would "make the stated invocation an hour's job rather than an afternoon's". A
  fork-mode start over the reduced corpus is about **a third** of what the same pass costs over the
  whole one — not a quarter, which is where the *file count* went. **`cmin` throws away the cheap
  seeds**: what distinguishes a seed is coverage, and the ones with distinct coverage are the large
  slow documents. Its own rate says it, falling from 256 executions a second at the start to 14 at
  the end. So the merge is still most of a short run's wall clock, and a round with an hour rather
  than three still passes a smaller `-runs`.

  **The merge that does the reducing is not free and is not a round's default**: about three
  quarters of an hour, one execution per seed. A round `cmin`s when the corpus has grown rather
  than as a habit. **And a `slow-unit-` artefact is the sanitiser's, which is checkable rather than
  assumed**: render the largest of them under `examples/render_at` against a release build before
  believing one.
- **`cargo-fuzz` is installed and always was**; it is in `~/.cargo/bin`, which is not on `PATH`,
  which is what two rounds read as its absence.

`doc/verify.md` has the rest — `cargo deny`, the fuzzers, the cross-target checks, the callgrind
counters and the census examples — and says which of them a change needs.

## 3. Leave the ledger non-`unreviewed`

Every clause a change touches gets its row in `doc/conformance/ledger.toml` brought up to date.
This is `CLAUDE.md`'s rule and not a courtesy: a row that describes what the code *should* do is
how this project has been wrong four times.

## 4. Sweep, after a round that adds a verb

The sweeps live in [`01-ledger-partial-rows.md`](01-ledger-partial-rows.md), which says what each
one asks and what its first run found; that file is the reading, and this is the rule. **Run them
over `crates/`, `tools/` and `fuzz/` as well as over `ledger.toml`** — `SOURCE_ROOTS` reaches all
three — and run the grep-shaped ones over `doc/adr/` too, for the one thing an unmaintained
document can get wrong: a claim a later round disproved and left standing. The ledger has a gate
and the source does not, which is why one session found four claims in the code false for between
forty and two hundred sessions, including `pdf-model`'s own crate documentation and a doc comment
that had *predicted* its own expiry.

What each sweep is worth is in that file with its evidence. What belongs here is the shape they
share and the two that break it:

- **Most sweeps read a row's stated *reason*** — a blocker that has expired, a capability the tree
  now has, a string a correction retired — and are therefore blind to a row with no reason at all.
  The sweep for that one prints every `partial` row whose note names nothing owed, which breaks
  the ledger's own definition of the status.
- **Two check a *number* rather than a claim.** One is arithmetic on the ledger: every row that is
  `partial`, `reported` or `unreviewed` while every one of its direct children is settled. The
  other is every `Table NNN`'s `/Key` citation against the entries ISO 32000-2 actually puts in
  that table — `tools/conformance` verifies a cited table *exists* and prints its title, so a
  number that exists and names the wrong table reads exactly like a right one, and those arrive in
  **blocks**, a run of consecutive rows written in one sitting against the older standard. That one
  is `--bin tables`, and the catalogue below says how to read it.
- **The rest are a catalogue rather than a rule**, and it lives with the reading:
  [`01-ledger-partial-rows.md`](01-ledger-partial-rows.md)'s *The sweeps as commands* holds every
  one of them unchanged — what it asks, the command that runs it, what its output's noise looks
  like and which hits to read first. Most are `cargo run --release -p conformance --bin <name>` and
  seconds apiece; the errata ones are `tools/spec-errata`'s `check`, `emit`, `moved` and `applied`,
  and **a round implementing a clause runs `emit` on that document *before* it writes, rather than
  `check` afterwards alone**. `tools/state.sh counts` is where a population goes, not a sentence
  here.

## 5. Put the binaries where a person can run them — every fifth round, and before any measurement

**The agent builds into `/home/AI/cargo-target/pdf-viewer/`, which the human's shell never looks
at.** So what a person would run has to be copied into the project's own `target/`.
`tools/state.sh binaries` says what is there and how old it is; `tools/round.sh` says whether this
round owes the rebuild.

**Its cadence is stated as a *rule about staleness* rather than as a habit**, and the rule is the
one this section has always argued from: **a stale binary is a measurement of the past.** The
hundred-and-forty-second session was reported as "still lags" against a binary three hours and six
commits old, one of which was the 40× page-turn fix. So:

- **before any measurement, always** — of the launch path, a page turn, a frame, a memory
  high-water, anything §2's gates do not print. There is no round that may measure against
  whatever was last linked;
- **every fifth round otherwise**, which is the same cadence as §2's full sequence and is bounded
  by the same argument: what a person picks up should never be more than a handful of rounds
  behind `HEAD`, and the link is the single largest item in a round (`doc/todo/43` §1).

A round in between may still run it and nothing goes wrong if it does — Cargo skips what did not
change. What is no longer required is paying for a whole-graph fat link at the end of a round that
moved a document.

```sh
cargo build --release --bin pdf-viewer --bin pdf-sandbox-worker --bin pdf-view-worker \
                     --bin pdf-viewer-gtk --bin pdf-viewer-qt --bin pdf-retrieve
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-viewer          target/pdf-viewer
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-sandbox-worker  target/pdf-sandbox-worker
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-view-worker     target/pdf-view-worker
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-viewer-gtk      target/pdf-viewer-gtk
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-viewer-qt       target/pdf-viewer-qt
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-retrieve        target/pdf-retrieve
cargo build --release -p viewer-ffi          # a library, so not in the invocation above
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/libviewer_ffi.so   target/libviewer_ffi.so
```

**One invocation, not three.** Each of these is a whole-graph fat link and Cargo runs three of
them beside each other where three commands run them one after another — measured both ways after
touching one file in `pdf-model` (ADR 0222). `--release` here is deliberate and is the one place
in a round that still pays for `lto = "fat"`: these are what a person runs and what every launch
measurement is taken from, and `--profile gates` above exists so that the *gates* stop paying for
it.

**`libviewer_ffi.so` is the exception that proves what this section is for**: it is not something
a person *runs*, and it is here because it is what a person *links against* — a C program with
`include/pdf_viewer.h` and no `-L` pointing at `/home/AI` is the only way somebody outside this
tree can try the ABI at all. It is a separate `cargo build` because it is a library and the
invocation above names binaries.

All the rest beside each other: `pdf_sandbox::WORKER_PROGRAM` is a separate executable the viewer
spawns for JBIG2 and JPEG 2000, and a viewer that cannot find it refuses those images rather than
falling back (there is deliberately no in-process fallback — see "the sandbox is a flag and the
default is the safe one"); `pdf-view-worker` is the whole viewer confined, which
`viewer_confined::Confined` spawns and `pdf-viewer` does not yet (ADR 0218); `pdf-retrieve` is not
a window but a program a person runs, and the only one whose whole output is text a caller pipes
(ADR 0257).

Build them first, in release: `cargo test` only ever builds the debug binaries, which is why the
cadence above exists at all.

**`viewer-confined`'s two binaries used to be built in release *before* the gates**, on a note
saying the gates needed them. They do not: those tests run under `cargo test --workspace`, which
builds the debug worker itself, and no release or gates binary in this tree names
`viewer-confined` — checked by grep over `pdf-model`'s and `render-quorra`'s manifests and test
sources. That was half a minute a round in the wrong section.

## 5a. Sweep the build directory when it passes a hundred gigabytes

`tools/state.sh disk` says what it is. A *clean* tree is about 17 GB of dev artefacts plus about
1 GB per release-grade profile; the rest is superseded output that Cargo on stable has no command
to remove — `cargo clean --gc` is nightly-only. So it is swept by hand, and `target/tmp/` is what
the sweep must **not** take.

```sh
rm -rf /home/AI/cargo-target/pdf-viewer/{debug,release,gates}   # never tmp/ — see below
```

`target/tmp/pdfref-cache` is the reference-render cache (ADR 0020), and deleting it costs the next
oracle run about a thousand seconds of `pdftoppm`, `mutool` and `gs`. `cargo clean` takes the
whole directory including that, which is why the sweep names its subdirectories instead.

The cost of the sweep is one cold build, measured on the swept tree at about three minutes for
`cargo test --workspace --no-run` plus the whole gates profile, with `release` on top of that only
when §5 runs. It buys no speed — the warm no-op build was 0.42 s with the directory at 311 GB — so
it is hygiene, on its own schedule rather than every round.

## 6. Write it down, then commit

**Check the file, not the script's exit status.** Twice, in the three hundredth and
three-hundred-and-first rounds, a Python edit put its `assert` *after* the replacements and
*before* the write, so a failed assertion left the file untouched while every other file in the
same commit moved. `grep` what you wrote back out of the file before committing it, which is the
same rule as trap 1 one directory over: the instrument that says a change happened is not the
change.

- The ADR, if the round made a decision. The argument goes there, not in the handover.
- **The session's record is one new file in [`doc/history/`](../history/README.md)**, named
  `<session>-<slug>.md`, and nowhere else. **Created, not appended to**: a round adds a file that
  did not exist, and edits no other round's — not `doc/history.md`, whose table is closed at 445,
  and not the neighbouring file. A number, a date or a session reference in any other document is
  bookkeeping and belongs in that file; a citation of an ADR for an *argument* is a pointer and
  stays where it is. (ADR 0281.)
- `doc/HANDOVER.md`, `doc/state-of-play.md`, the `doc/traps/` group the round was in,
  `doc/todo/README.md` and this file: only if what they *claim* stopped being true. None of them
  holds a number, so a round that only moved numbers writes nothing here. **A new trap goes in the
  group whose rounds would spring it**, keeps the next free number, and gains a row in the
  handover's index — the numbers are consecutive across the five files, not inside one.
- The todo file: delete it if the item is done, correct it if the round changed what it owes.

## 7. Three habits these rounds added, which belong here rather than in a trap

- **A closed form taken from one renderer is not a limit.** `doc/todo/00`'s step 6 climbs a
  reference to eight times the resolution because its departure from the geometry shrinks with
  the pixels — and on a tiling pattern `poppler` goes the other way, its strokes thinning rather
  than its edges sharpening. Take two ladders: one cannot tell convergence from drift, and two
  also say when *neither* has converged.
- **A count that improves is not a picture.** The two-hundred-and-eighteenth session took the
  corpus's incomplete list down by two and both documents were still wrong — one of them blank.
  Trap 1's oldest sentence, and the second finding was three steps beyond the first, in a
  function neither document was about. **The inverse holds too**: a count that does *not* move is
  not evidence that nothing happened, which is what a round finding a defect no corpus document
  carries discovers.
- **A round that changes what gets drawn re-runs `doc/todo/00`'s step 7.** Our ink minus the
  lightest reference's, over **every ambiguous page** — the gate's own output, not
  `ambiguous_undiagnosed.txt`, or diagnosing a population would take its pages off the one
  instrument that sees content this tree is *not drawing*. Three minutes, from artefacts already
  on disk. Drop a reference whose ink is zero first, and read the result beside the corpus's
  incomplete list: a page this tree reports is expected to be light. It found its first defect in
  the two-hundred-and-sixty-fifth session — a text annotation attached to a point, drawn as
  nothing — on a page the ranking rated harmless because a nearly blank page resembles a nearly
  blank page.
