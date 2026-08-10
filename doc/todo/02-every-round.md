# What every round does

Status: **standing** — this one is never done.
Priority: 02

A "round" here is one session's worth of work. `CLAUDE.md`'s two tracks decide *what* it
contains; this file is what it does around that, in order.

## 1. Take from both tracks

Demand-driven is what the corpus and the oracle name (todos `10`–`29`); spec-driven is the
ledger's `reported` rows and the notes on its `partial` ones (todos `00`–`09`). A project
running only the first finishes when the corpus goes quiet, which can happen with much of the
standard unimplemented and nothing able to say which parts; one running only the second ships
features no file exercises.

## 2. Run the gates that can see what you touched

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets      # must be silent of lints
cargo nextest run --workspace               # 1525 tests, 10 skipped
cargo test --workspace --doc                # the one doctest nextest does not run
cargo build --profile gates -p pdf-sandbox --bins   # trap 10: Cargo will not do this for you
cargo test  --profile gates -p pdf-model      --test corpus          -- --ignored --nocapture
cargo build --profile gates -p hayro-compare --bin pdfref-hayro      # trap 10 again, see below
cargo test  --profile gates -p pdf-model      --test oracle          -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test text_extraction -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test dates           -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test xmp             -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test jpeg2000        -- --nocapture
cargo test  --profile gates -p render-quorra  --test corpus          -- --ignored --nocapture
cargo test -p conformance -- --nocapture
```

**This sequence is 268 s where the same gates were 608 s until the three-hundred-and-eighty-fifth
session**, which measured every step of it and changed four things; ADR 0222 has the table and the
argument, and `Cargo.toml`'s profiles carry the reasoning beside the settings. Three notes bind
here:

- **`--profile gates`, not `--release`.** Release-grade optimisation with cheap linking, because a
  fat whole-graph link *per gate binary* was 175 s of every round. All eight gates were run under
  both profiles and their output compared line by line — 1794 oracle page verdicts, 957 quorra
  pages, 974 corpus documents, 4990 citations, **every field identical**. `--release` still works
  and is still the same gate; it is only slower. **`[profile.release]` did not change**, and §5's
  binaries are still built with it.
- **`cargo nextest` is a user-local install** — `cargo install cargo-nextest --locked`, or the
  prebuilt from `https://get.nexte.st/latest/linux` into `~/.cargo/bin`. Without it,
  `cargo test --workspace` is exactly the same gate at three times the wall clock, and that is
  what CI runs. `nextest` skips doctests, which is why the line after it is there: 1525 + 1 = the
  **1526** `cargo test --workspace` reports. (This line said 1314 until the
  three-hundred-and-eighty-eighth, which counted the tree with `nextest list` before and after its
  own seven and found the number two behind — the count is the gate's and not this file's. It said
  **1398** until the four-hundred-and-second, which added six tests to a gate that printed 1410:
  the number was six behind again, over the eight rounds since it was last written down. The
  four-hundred-and-third added four and this line was **not** behind, which is what happens when
  the round before it wrote the gate's own number down; the four-hundred-and-fourth added four
  more and it was not behind either; the four-hundred-and-fifth added **two** and it was not behind for the third round running, which is what happens when each round writes the gate's own
  number down. The four-hundred-and-seventh added one **ignored** test and moved the second number
  instead — 9 skipped to **10** — which is the one the line beside it had never been checked
  against. The four-hundred-and-eighth added **ten**, a whole new host crate's, and this line was
  not behind for the fourth round running; the four-hundred-and-ninth added **five** and it was not
  behind for the fifth; the four-hundred-and-tenth added **nineteen**, a second host crate's, and it
  was not behind for the sixth; the four-hundred-and-eleventh added **nineteen** — a third host
  crate's seventeen and two more — and it was not behind for the seventh; the four-hundred-and-twelfth
  added **eight** — five in `pdf-model` for §12.7.5.4's two `/V` shapes and Table 234's `/I`, one
  apiece in `viewer-core`, `viewer-confined` and `viewer-qt` — and it was not behind for the eighth;
  the four-hundred-and-thirteenth added **one**, in `viewer-ui`, and it was not behind for the
  ninth; the four-hundred-and-fourteenth added **nine** — three in `viewer-core`'s own `search`
  module, three in its headless harness, one apiece in `select`, `viewer-ui`'s panel test and
  `viewer-core`'s fragment tests — and it was not behind for the tenth; the four-hundred-and-fifteenth added **two** — both in
  `pdf-model`'s `transparency_groups`, for §11.6.6's inheritance and for what compositing in
  `/DeviceCMYK` costs — and it was not behind for the eleventh; the four-hundred-and-sixteenth
  added **two** — both in the new `tools/spec-errata`, for which of Table 172's `/RT` values makes
  an annotation a reply and for a strikeout that covers no glyph — and it was not behind for the
  twelfth. **The four-hundred-and-eighteenth added one** — §7.8.3's first step for a Type 3 glyph's
  resources — **and the gate printed 1498, so the line was two behind**, which is the first time in
  seven rounds it has been. Where the two came from was not chased; what the run shows is the rule's
  precondition rather than the rule: this line is current only for a round that *ran* the gate and
  copied its number, and a round that writes the number without running it writes last time's. **The
  four-hundred-and-nineteenth added eight** — a whole `pdf-model/tests/missing_resources.rs` for a
  name §7.8.3's resource dictionary does not define — **and the gate printed 1506, so this line was
  not behind**, which is what happens when the round before it copies the gate's own number. The four-hundred-and-twentieth added **nine** and the gate
  printed **1515**; the four-hundred-and-twenty-first added **ten** — four in the new
  `tools/pdf-retrieve`, one in its JSON writer, two in `pdf-model`'s new `retrieval` module, one for
  the structure-tree walk that used to see half of ISO 32000-2 and two more in the tool's own tests —
  and the gate printed **1525**, so this line was not behind for the third round running.)
- **One of those eighteen runs a C compiler**, and it is the only gate in this sequence that does.
  `viewer-ffi::a_c_program_drives_the_abi` builds `crates/viewer-ffi/c/open_a_page.c` against the
  crate's own header with `-Wall -Wextra -Werror`, links it against the `cdylib` — which it asks
  cargo to build, because `cargo test` does not — and runs it on a document. It **skips** where
  there is no `cc` or `gcc`, printing why: a machine without a C compiler cannot run it, and
  failing there would make the gate a coin toss. CI has one, so on CI it is not a skip.
- **`pdfref-hayro` is the oracle's fourth reading and nothing built it.** It is a *program*, found
  beside the running test binary, and its absence costs no verdict — `Reference::Hayro` never
  votes — but it is what a person looks at on a page the three references cannot settle. Until
  this session it existed under `target/release/` only because some earlier round happened to run
  `cargo build --release -p hayro-compare --bins`. Its line is placed *after* the corpus gate
  rather than in front of it, which is worth 7 s: the corpus gate compiles `pdf-model`'s rlib and
  its own test target in one graph, and `-p hayro-compare` on its own has nothing to overlap.

**Twelve fuzz targets, not five** — the handover's list had never included `object` and
`document`, `sfnt` arrived in the two-hundred-and-forty-first, `xmp` in the two-hundred-and-ninety-fourth, `fragment` in the three-hundred-and-sixty-ninth, **`confined_wire` in the three-hundred-and-eighty-sixth** — the confined viewer's four decoders, whose input is a *process* rather than a document (ADR 0223) — and **`x509` in the three-hundred-and-ninety-second**, the signer's certificate and the RSA arithmetic that runs on the key inside it (ADR 0229), seeded by `fuzz/seed_x509.py`. A round that touches a parser
runs the one that covers it; a round that touches `pdf-font`'s glyph-table repairs runs `sfnt`
**with its corpus seeded**, because unseeded it never forms a table directory and tests nothing.

`doc/verify.md` has the rest — `cargo deny`, the twelve fuzzers, the two cross-target checks, the
callgrind counters and the census examples — and says which of them a change needs. **This sentence
said "the five fuzzers" while the paragraph above it said twelve**, and the three-hundred-and-ninety-fifth
corrected it while moving the list out of `doc/HANDOVER.md`.

## 3. Leave the ledger non-`unreviewed`

Every clause a change touches gets its row in `doc/conformance/ledger.toml` brought up to date.
This is `CLAUDE.md`'s rule and not a courtesy: a row that describes what the code *should* do is
how this project has been wrong four times.

## 4. Sweep, after a round that adds a verb

**Four** greps, two pieces of arithmetic and five more that are neither, twenty lines of Python apiece, each of which has paid on its first run: a
note whose stated blocker has expired ("while §X does not exist", "needs §Y"), a note claiming an
entry is unread where the tree reads it, a note whose reason is a *capability* — "this program
has no ___", "no panel", "which this is not" — and the string a correction retired, grepped over
every *other* row. The third found a `shall` binding for fifty-six sessions; the fourth found
§8.9.6.1 still carrying a sentence §11.6.4.3 had retired fourteen sessions earlier.

**The arithmetic one needs no round to justify it**: print every ledger row that is `partial`, `reported` or `unreviewed` while every one of its direct children is settled. Its first run found five and four were wrong, three of them in a shape no grep can see — the note corrected and the status left behind.

**Eleven now, and the ninth checks a *number* rather than a claim**: every `Table NNN`'s `/Key`
citation, against the entries ISO 32000-2 actually puts in that table. `tools/conformance` verifies
a cited table *exists* and prints its title; a number that exists and names the wrong table reads
exactly like a right one, and it arrives in **blocks** — a run of consecutive rows written in one
sitting against the older standard. Its first run, in the three-hundred-and-eighty-seventh,
corrected nine ledger rows and five source comments; its fourth, in the four-hundred-and-thirteenth,
found five more and **one of them was a correction that had replaced a wrong number with another
wrong number**. It is not a gate and `doc/todo/01` says why.

**The tenth is arithmetic on a parent's *prose*** — "three of the twenty" against what the rows
below it say — and it has paid on both of its runs; the four-hundred-and-thirteenth's was §12.7.6,
wrong for 280 sessions.

**The eleventh reads the ledger's own quotation marks**, which no gate in this project does: the
checker verifies every rustdoc blockquote in `crates/` and nothing at all in `ledger.toml`, whose
notes hold 977 quoted spans. Report only the misses that match the standard for at least five words
and then diverge — a claim this project invented shares no words with it and a misquotation shares
most of them — and the first run found six. ADR 0249, and it is not a gate for a reason the ADR
prices.

**A twelfth since the four-hundred-and-eighteenth, and it is not run from here** — it is
`cargo run --release -p spec-errata -- check doc/*.pdf`, seven seconds, and it asks the same 977
spans a *different* question: does one of them quote a sentence Errata Collection 3 struck out? That
needs none of ADR 0249's syntax, because the erratum supplies the other side. Its first run found four
stale quotations in three ledger rows and, beside them, six more in a population nobody had counted
at all — quotation marks inside ordinary rustdoc prose, which `CLAUDE.md` binds exactly as hard as a
blockquote and which the gate's `> ` scanner walks straight past. ADR 0254.

**And run all four greps over `crates/` as well as over `ledger.toml`.** The ledger has a gate and the
source does not, which is why the two-hundred-and-twenty-first session found four claims in the
code false for between forty and two hundred sessions — including `pdf-model`'s own crate
documentation and a doc comment that had *predicted* its own expiry. See
`01-ledger-partial-rows.md`.

## 5. Put the binaries where a person can run them

**The agent builds into `/home/AI/cargo-target/pdf-viewer/`, which the human's shell never looks
at.** So at the end of every round, copy what a person would run into the project's own
`target/`:

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

**One invocation, not three**, since the three-hundred-and-eighty-fifth: each of these is a whole-graph
fat link and Cargo runs three of them beside each other where three commands run them one after
another — **109.7 s to 79.3 s**, measured both ways after touching one file in `pdf-model` (ADR 0222).
`--release` here is deliberate and is the one place in a round that still pays for `lto = "fat"`:
these are what a person runs and what every launch measurement is taken from, and `--profile gates`
above exists so that the *gates* stop paying for it.

**And one library since the four-hundred-and-eleventh**, which is the exception that proves what
this section is for: `libviewer_ffi.so` is not something a person *runs*, and it is here because it
is what a person *links against* — a C program with `include/pdf_viewer.h` and no `-L` pointing at
`/home/AI` is the only way somebody outside this tree can try the ABI at all. It is a separate
`cargo build` because it is a library and the invocation above names binaries.

**Six since the four-hundred-and-twenty-first**, which added `pdf-retrieve` — not a window but a program a person runs, and the only one whose whole output is text a caller pipes (ADR 0257). **Five, not three, since the four-hundred-and-tenth** — `pdf-viewer-gtk` is the GTK4 host
(ADR 0244) and `pdf-viewer-qt` is the Qt 6 one (ADR 0246), and each is a program a person runs, so
they belong in the same invocation and for the same reason as the other three. This section named two for three rounds after the third arrived, which the
three-hundred-and-eighty-third flagged and the three-hundred-and-eighty-fourth fixed. All three
beside each other: `pdf_sandbox::WORKER_PROGRAM` is a separate executable the viewer spawns for
JBIG2 and JPEG 2000, and a viewer that cannot find it refuses those images rather than falling
back (there is deliberately no in-process fallback — see "the sandbox is a flag and the default
is the safe one"); `pdf-view-worker` is the whole viewer confined, which
`viewer_confined::Confined` spawns and `pdf-viewer` does not yet (ADR 0218).

Build them first, in release. `cargo test` only ever builds the debug binaries, and a stale
executable is a measurement of the past — the hundred-and-forty-second session was reported as
"still lags" against a binary three hours and six commits old, one of which was the 40×
page-turn fix.

**`viewer-confined`'s two binaries used to be built in release *before* the gates**, on a note
saying the gates needed them. They do not: those tests run under `cargo test --workspace`, which
builds the debug worker itself, and no release or gates binary in this tree names
`viewer-confined` — checked by grep over `pdf-model`'s and `render-quorra`'s manifests and test
sources in the three-hundred-and-eighty-fifth. That was 31 s a round in the wrong section.

## 5a. Sweep the build directory when it passes a hundred gigabytes

`/home/AI/cargo-target/pdf-viewer` was **311 GB** in the three-hundred-and-eighty-fifth session,
and a *clean* tree is 17 GB of dev artefacts plus about 1 GB per release-grade profile. The rest
is superseded output that Cargo on stable has no command to remove — `cargo clean --gc` is
nightly-only. So it is swept by hand, and `target/tmp/` is what the sweep must **not** take:

```sh
du -sh /home/AI/cargo-target/pdf-viewer
rm -rf /home/AI/cargo-target/pdf-viewer/{debug,release,gates}   # never tmp/ — see below
```

`target/tmp/pdfref-cache` is the reference-render cache (ADR 0020), **1.5 GB**, and deleting it
costs the next oracle run about a thousand seconds of `pdftoppm`, `mutool` and `gs`. `cargo clean`
takes the whole directory including that, which is why the sweep names its subdirectories instead.
Run in the three-hundred-and-eighty-fifth: **334 GB → 8.1 GB**, the cache untouched.

The cost of the sweep is one cold build, measured on the swept tree: **87.9 s** of
`cargo test --workspace --no-run` and **97.2 s** of the whole gates profile, with `release` on top
of that only when §5 runs. Three minutes for three hundred gigabytes. It buys no speed — the warm
no-op build was 0.42 s with the directory at 311 GB — so it is hygiene, on its own schedule rather
than every round.

## 6. Write it down, then commit

**Check the file, not the script's exit status.** Twice in the three hundredth and
three-hundred-and-first rounds a Python edit put its `assert` *after* the replacements and
*before* the write, so a failed assertion left the file untouched while every other file in the
same commit moved — `doc/todo/00`'s own counts disagreed with the gate that had just printed them.
`grep` the number back out of the file before committing it, which is the same rule as trap 1 one
directory over: the instrument that says a change happened is not the change.


- The ADR, if the round made a decision. The argument goes there, not in the handover.
- `doc/HANDOVER.md`: the gate numbers if they moved. **The session row goes in `doc/history.md`**,
  which is where "How the project got here" moved in the three-hundred-and-ninety-fifth.
- The todo file: delete it if the item is done, correct it if the round changed what it owes.

## 7. Three habits these rounds added, which belong here rather than in a trap

- **A closed form taken from one renderer is not a limit.** `doc/todo/00`'s step 6 climbs a
  reference to eight times the resolution because its departure from the geometry shrinks with
  the pixels — and on a tiling pattern `poppler` goes the other way, 34.15 → 16.32, its strokes
  thinning rather than its edges sharpening. Take two ladders: one cannot tell convergence from
  drift, and two also say when *neither* has converged.
- **A count that improves is not a picture.** The two-hundred-and-eighteenth session took the
  corpus's incomplete list from 80 to 78 and both documents were still wrong — one of them
  blank. Trap 1's oldest sentence, and the second finding was three steps beyond the first, in a
  function neither document was about.
- **A round that changes what gets drawn re-runs `doc/todo/00`'s step 7.** Our ink minus the
  lightest reference's, over **every ambiguous page** — the gate's own output, not
  `ambiguous_undiagnosed.txt`, or diagnosing a population would take its pages off the one
  instrument that sees content this tree is *not drawing*. Three minutes, from artefacts already
  on disk. Drop a reference whose ink is zero first, and read the result beside the corpus's
  incomplete list: a page this tree reports is expected to be light. It found its first defect in
  the two-hundred-and-sixty-fifth session — a text annotation attached to a point, drawn as
  nothing — at −1.783, on a page the ranking rated 0.73 because a nearly blank page resembles a
  nearly blank page.
