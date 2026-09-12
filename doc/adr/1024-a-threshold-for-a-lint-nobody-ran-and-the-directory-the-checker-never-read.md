# 1024 — A threshold for a lint nobody ran, and the directory the checker never read

Session 1004. Status: **accepted**. Four items of the direction review's small list
(`doc/reviews/984-direction-and-boundaries.md` Finding 7; ADR 1005 §7 is the proposal) taken, one
judged and two handed on; and the population question answered against four candidates, three of
which a number eliminated. Amends `clippy.toml`, `crates/viewer-ui/src/bin/quorra.rs` and
`quorra/arguments.rs`, `raster/CLAUDE.md` and 114 Rust files under `raster/crates/`,
`doc/verify.md`, `tools/fuzz.sh`, `tools/round.sh` and `fuzz/seed_page.py`.

`§N` is ISO 32000-2 and nothing else. Every figure below was produced in this session by the
command beside it.

## 1. The cognitive-complexity threshold: enabled first, then deleted

`clippy.toml` carried `cognitive-complexity-threshold = 20` for `clippy::cognitive_complexity`,
which is a **nursery** lint that no manifest and no crate root names — so the setting configured an
instrument that had never run once, and its comment promised that irreducible cases would take "an
explicit `#[allow]`", which is trap 7 written into the file that configures the linter.

ADR 1005 §7 offers two roads, enable or delete. **The lint was enabled and run before either was
chosen**, because deleting a setting on the argument that it finds nothing is a claim nobody had
checked:

```sh
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- --force-warn clippy::cognitive_complexity
```

`--force-warn` rather than `-W` because the gate's `-D warnings` would otherwise turn the first
finding into an error and stop the crate — the flag exists for exactly this, and clippy prints
`the clippy::cognitive_complexity lint ignores -D warnings` when it takes effect. Every workspace
member compiled under it — `crates/`, `tools/` and `raster/crates/` alike, and the run names each
one it checked — and it found **nothing, in any crate**.

**Nothing is not a result until the instrument is calibrated** (trap 13), and calibrating it is
what settled the decision. Against planted functions, under this tree's own `clippy.toml`:

| planted function | score |
|---|---|
| 60 sequential `if … else` | `61/20` — fires |
| a `match` of 75 arms | nothing |
| four levels of nested `for`/`if`/`else` | nothing |

The lint counts `if` expressions and subtracts returns. **A `match` of any width and a nesting of
any depth score zero**, and that is what this tree's long functions are made of — the interpreter's
`run_reader` is 908 lines and 75 arms, and it is invisible to this lint by construction. So the
threshold is deleted rather than enabled: a number that cannot move for the shape it is aimed at is
not a bound, and enabling a lint that can never fire is trap 39's other half — a signal with
nothing to say, wearing the look of care.

The measurement is not written down as a count. The **command** is, in `doc/verify.md` beside the
other instruments that are not gates, with what the metric actually counts, so that the next round
to ask can print the answer instead of quoting this one (`CLAUDE.md`, *Where knowledge lives*).

## 2. Two `#[allow]` become `#[expect]`

`crates/viewer-ui/src/bin/quorra.rs` and `quorra/arguments.rs` each carried
`#[allow(clippy::too_many_lines)]` with the reason in a trailing `//` comment. Both are now
`#[expect(…, reason = "…")]` with the same words in the attribute, which is where the rest of the
tree puts them. Trap 7's whole point is that the exception expires by itself; an `allow` on the
program's own entry point was the one place it could not.

## 3. The two ADR series: a path, not a prefix

ADR 1005 §7 proposes a filename prefix (`R-0053`) for `raster/doc/adr/`. **This round chose the
path instead**, and the reason is a measurement the proposal did not have. The collision is not
confined to the ADRs:

| token written in `raster/crates/` Rust | resolves in `raster/doc/` | resolves in `doc/` |
|---|---|---|
| `doc/adr/0001`, `…0003`, `…0005`, `…0037`, `…0045`, `…0051`, `…0053` | yes | yes — a different decision each time |
| `doc/PLAN.md` | yes | yes |
| `doc/HANDOVER.md` | yes | yes |
| `doc/RENDER_LIBRARY.md` | yes | yes |

A prefix on 94 filenames fixes the first row and leaves the other three exactly as they were. A
path fixes all four, renames nothing, and is the form every other citation in this tree already
takes. So: **a pointer written in Rust under `raster/crates/` names its file from the repository
root** — `raster/doc/adr/0053`, never `doc/adr/0053` — and a pointer at one of the *caller's*
documents keeps the bare `doc/` it always had, which is what makes the two distinguishable at a
glance. `raster/CLAUDE.md` principle 4 states it.

209 tokens in 114 files were rewritten, mechanically and with the discriminator derived rather than
listed: a token is raster's own if `raster/doc/<rest>` exists. Every rewritten pointer was then
resolved against the disk, and the nine pointers at the caller's documents
(`doc/QUORRA_ENCODE_THREADS.md`, `doc/HAYRO_ISSUES_FOR_QUORRA.md`, `doc/todo/44` and the rest) were
left alone by that same rule.

**The divergence worry is answered by the history rather than by this decision.** `raster/` is not
a checkout beside this one: it was committed into this repository as a sub-project
(`git log -- raster/`: three commits), its workspace was folded into this one
(`Cargo.toml:2` — `members = ["crates/*", "tools/*", "raster/crates/*"]`), and its crates are path
dependencies. There is no upstream tree in which the bare `doc/` was ever going to stay right.

## 4. The population question: what an instrument's own denominator hides

The question asked of four candidates, each eliminated or kept by a number.

**`tools/state.sh`'s section list — eliminated.** The script holds three hand-written lists of its
sections: `$all`, the dispatch arms, and the `section_*` functions themselves. `--list` prints
`$all`, so a section function absent from it is invisible in both directions. Measured: 31 / 31 /
31 today, and **0 drifts across all 37 commits that have ever touched the file**. A check here
would be one that has never had anything to say.

**The fuzz workspace's targets — eliminated as a population, kept as a defect.** `tools/fuzz.sh`
already derives its targets from `fuzz/Cargo.toml`'s `[[bin]]` entries rather than from a list of
its own (16 entries, 16 files, no discrepancy either way), which is the right shape. But see §5:
what it *reports* about them was never read.

**`doc/verify.md`'s instruments against the binaries — eliminated.** 197 runnable targets exist
under `crates/`, `tools/` and `raster/crates/`; 109 of them are named in none of `doc/verify.md`,
`doc/todo/02`, `tools/state.sh` or `doc/performance.md`. A predicate that fires on 55% of a
population is not a defect finder, and nearly all of the 109 are one-round census examples that
print rather than assert — of the 152 examples under `crates/` and `tools/`, only **7** carry an
assertion at all, and only 2 of those are named nowhere.

**`raster/`'s own gate set — the premise was stale, and what it was hiding is the finding.**
`raster/` is not a second workspace: it was folded into this one, so `--workspace` reaches its five
crates, and §1's run checked them alongside everything else. Its examples are
covered better than the main tree's — `raster-gpu/tests/example_checks.rs` reads `.github/`'s
workflow and fails naming any example no line runs, which is a derived population and the right
construction (raster's ADR 0060).

What is *not* covered is everything else in it, and this is the answer the question was after.
**`tools/conformance`'s scan reads a hand-written list of directories**:

```rust
pub const SOURCE_ROOTS: [&str; 3] = ["crates", "tools", "fuzz"];   // tools/conformance/src/lib.rs
```

The workspace's members are a glob and the checker's roots are a list, and only one of the two grew
when `raster/crates/*` was folded in. So **1,884 clause citations across 243 of that sub-project's
283 Rust files have been outside the gate that checks citations and quotations since the day it
arrived** — and the checker's output looked exactly as it always had, because a directory nobody
reads produces no findings. `pointers.rs` has the same shape one constant along: its
`ROOTED_HEADS` are `doc, crates, tools, fuzz, data`, so no pointer written under `raster/` is
resolved at all — which is why §3's collision could sit there unreported rather than
mis-resolving loudly.

This is trap 25 with the population on the **instrument's** side rather than the tree's, and
session 985's `--workspace`-reaches-one-workspace lesson in a second shape: a scope that is a list
beside a scope that is a glob.

`tools/conformance/` is another round's this batch, so the fix there is handed on (§6). What this
round built is the check that makes the omission visible from the place every round already looks:
`tools/round.sh` now derives the roots from the checker's own source and the members from the
workspace manifest, compares them, and prints the count of unchecked citations under any member
that is not covered. It fires today, naming `raster/crates`. Calibrated both ways: with
`raster/crates` added to a scratch copy of `SOURCE_ROOTS` it passes, and an unreadable side is a
`✗` rather than a silent pass.

## 5. A sweep that printed its finding and exited 0

`tools/fuzz.sh --list` prints one row per fuzz target with its seed count and the invocation
`doc/verify.md` gives it, and refuses to run a target that file does not name — a good
construction, and ADR 0742's. What it did not do is **fail**. `serialize`, the target that fuzzes
§7.5's structure on the way *out* through RFC 0002's serializer, arrived with that serializer and
`doc/verify.md` never named it. So every run of `--list` since has printed

```
serialize               0   NO INVOCATION — this target is in fuzz/Cargo.toml and not in doc/verify.md
```

above an exit status of 0, and `tools/fuzz.sh serialize` has refused to start. The target has
never been run.

Trap 25's own prescription is the fix and it is one line of shell: *a member the sweep cannot
measure is a non-zero exit rather than a row it quietly drops.* `--list` now exits 1 naming the
undocumented targets. A **seed** count of zero is deliberately not one of those — `fuzz/corpus` is
gitignored, so on a fresh clone every target reads zero, and that is a fact about the disk rather
than about the tree.

Calibrated against the defect it looks for (trap 13): with the new `doc/verify.md` line removed it
exits 1 naming `serialize`; with it restored it exits 0. The line itself is written, with the
seeding recipe `document` and `crypt` already use — `fuzz/seed_page.py` names the target now, and
1,133 seeds are on this disk.

### The first run of it found a crasher

Run once the line existed, `serialize` aborted during its **seed corpus pass**, before a single
mutation:

```
thread '<unnamed>' panicked at fuzz_targets/serialize.rs:90:14:
a file whose trailer names /Root must reach it: TrailerMissing { key: "/Root (not a dictionary)" }
```

The property that failed is the target's own and it is the one RFC 0002 §11.3 named: **a file this
serializer wrote, this reader could not open.** The input is 184 bytes and reproduces:

```
%PDF-1.4\n1 0 obj \n<<\n/Pages 2 0 R\n2 0 obj \n<<\n/Resources \n>>\n/Contents 811 0 R\n811 0 obj
\n<<\n/Length 17863\n>>\nstream\nBI\n/W 62\n/H 62\n/D[1\n0]\n/F/CCF\x8a/DP<</K -1\ntrailer\n<<\n/Root
1 0 R\n>>\n
```

— a file with no `endobj`, no `xref` and an unterminated stream, recovered by scanning, whose
trailer names object 1 and whose object 1 *is* a dictionary on the way in. Written out and read
back, `/Root` no longer reaches one. §7.5.5 makes the trailer's `/Root` "[t]he catalog dictionary
for the PDF file", so a file whose `/Root` names something else is not one this project may emit.

**Not diagnosed or fixed here.** The site is `pdf-syntax`'s serializer, which is a rule-2 crate
(`doc/todo/02` §2), so the change costs the whole gate sequence and belongs to a round that owns it
— and this round's tree could not be taken green in any case, with four siblings mid-edit in it.
`CLAUDE.md` principle 3 says every crasher becomes a permanent regression test; the bytes are
above because `fuzz/artifacts/` is gitignored and would otherwise be this disk's alone.

### And a fuzz target cannot run under `tools/bounded.sh`

Worth one line so the next round does not spend the twenty minutes this one did. `cargo fuzz`
builds with AddressSanitizer, which reserves about 15 TiB of shadow memory at start-up;
`bounded.sh` sets `RLIMIT_DATA`, which counts exactly those private anonymous mappings, so the run
dies with `ReserveShadowMemoryRange failed` before libFuzzer prints anything — and `fuzz.sh`
correctly reports that as "no coverage line at all", which is trap 24's answer and looks identical
to a build failure. A fuzz target bounds itself with libFuzzer's own `-rss_limit_mb`, which is what
`doc/verify.md`'s invocations already carry where they need one.

## 6. What this round did not take, and why

- **`viewer_core::Secret` to `pdf-syntax`.** `crates/pdf-transform/` is session 999's this batch.
  One line for a later converter round: `pdf-transform` depends on the viewer's *application* crate
  for a password buffer, and the buffer belongs beside §7.6's decryption.
- **The refusal citing Annex C.4 as a clause** (`pdf-transform/src/archive/mod.rs:206`,
  `optimize.rs:341`). Same crate, same round, handed on: Annex C is titled "(informative)", so the
  refusal is this project's choice and should say so.
- **`CUTOFF = 0.0005` in the two rasterisers.** `render-gpu`'s side is this round's;
  `render-cpu`'s is session 1002's. There is no home for a shared constant that is not also 1002's:
  `render-cpu` and `render-gpu` share exactly one internal dependency, `pdf-render`, and that is
  where the constant belongs — beside `Ramp`, since what it describes is the width of the
  transparent stop a non-extended `/Extend` end takes in a ramp, which is a property of the display
  list's vocabulary rather than of either rasteriser's library. Reported rather than taken, because
  a constant moved in one crate and left in the other is the defect the review named, made worse.
- **`pdf-font -> pdf-render` for geometry — judged, not moved.** It is not one struct and it is not
  an inversion. `pdf-font` uses `Path`, `PathCommand`, `Point` and `Transform`, all four from
  `pdf-render/src/geom.rs` (1,323 lines), and it uses them as its **output type**: a loaded font's
  cached glyph outline is an `Arc<pdf_render::Path>` that goes into a display list unchanged
  (`loading.rs:311`, `:468`, `:1688`). `pdf-render` has no internal dependency of its own, so this
  is a leaf vocabulary crate being used as one, not a font crate reaching up into a rasteriser.
  What is fair in the review's complaint is the *name*: the crate that holds the tree's geometry
  also holds the `Rasterizer` trait, so a reader sees a font crate depending on rendering. The move
  that would answer it is `geom.rs` out into a `pdf-geom` crate — one file, no dependencies,
  18 dependents to re-import — and it is a change to a rule-2 crate, so it costs a whole gate
  sequence. Worth doing only beside another reason to open `pdf-render`, never on its own.
- **The crate map's rows** are session 1000's.

## 7. Consequences

- `clippy.toml` no longer configures a lint nobody runs, and no longer promises `#[allow]`.
- A round reading `tools/round.sh` — which every round does first — is told when a workspace member
  is outside the conformance checker's reach, with the number of citations that costs.
- `tools/fuzz.sh --list` is a gate on its own population rather than a report about it.
- A pointer written in `raster/`'s Rust names one series. A pointer written in `raster/`'s own
  *documents* still does not, and that is deliberate: those are read from inside the sub-project
  and nothing scans them. If `SOURCE_ROOTS` grows to reach `raster/crates/`, the documents are the
  next question and not the same one.
