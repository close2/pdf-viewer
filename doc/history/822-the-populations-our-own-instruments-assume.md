# 822 — Three instruments asked what they were looking at, and two had written it down

Date: 2026-08-29. Branch `round-822`, from `main` at `3ec61db7`. Parallel round, worktree `r822`.
ADR: [0752](../adr/0752-the-populations-our-own-instruments-assume.md).
Touched: `tools/round.sh`, `tools/worktree.sh`, `tools/state.sh`, `doc/environment.md`,
`doc/todo/02-every-round.md` (§5a), `doc/traps/instruments-and-reports.md` (new trap 25),
`doc/HANDOVER.md` (the trap index), and two new files, `doc/adr/0752` and this one.
Two commits: the three instruments, then `state.sh`'s own reading table.

## The subject, and why it was taken

The batch's general-improvement round, told to let the instruments name the subject and to keep off
three siblings' lanes (errata ranking, Annex O's renumbering, the fuzz corpora). The briefing also
carried a small item the merge round before had found and deliberately left: `tools/worktree.sh
list` globs `pdfv-r*`, so two orphaned build directories named otherwise were invisible to it for
hundreds of rounds — **and the interesting half is whether anything else the tools glob for has the
same shape.** That second sentence is what this round took, and it is a subject rather than an
errand: a glob, a hand-written list and a hard-coded line range are one construction — a claim
about the tree standing beside the tree with nothing comparing them.

The other instruments were asked first, over the pristine tree, and none of them named a defect.
The seventeen committed sweeps printed the shapes their own catalogues call noise: `entries`'
reading list is Table 237's and 238's seed-value entries and Table 212's and 213's sound and movie
ones, all inside declared exclusions; `inapplicable`'s hits are the standard's shared vocabulary at
three hundred-odd naming files apiece; `parts`' are the cross-backend pairs its own noise paragraph
names; `overstated`'s eight contradictions carry seven demoting marks between them. The ledger's
`silent` population is zero and its `unreviewed` population is zero. The corpus's incomplete list is
classified exhaustively. Nothing there outranked a question that had already been asked and left
open in writing.

Two negative results were taken on the way and are worth recording, because they are the half of a
survey that never gets written down. **The corpus globs are honest**: `find doc/pdf.js/test/pdfs
-maxdepth 1 -name '*.pdf'` and `find -L doc/corpora -name '*.pdf'` return the same counts as their
case-insensitive, unlimited-depth forms, and every one of the two dozen gate harnesses that walks a
corpus filters on `extension == "pdf"` — one spelling, no disagreement. **And `tools/worktree.sh`'s
`linked` array is complete against `.gitignore`** but for `/doc/errata.md` and `/doc/rasterrocket`,
neither of which a gate reads and the first of which `tools/spec-errata emit` regenerates in
seconds.

## What the three instruments were actually looking at

`doc/adr/0752` has the argument and the calibration table. In one line each:

- **`tools/round.sh`'s build-script check** asked the literal list `'pdf-font' 'conformance'`.
  `tools/conformance` has never had a build script in any commit of this repository, so half of
  every run since the check was written has looked for a thing that does not exist and printed a
  `✓` — while `crates/pdf-sandbox/build.rs`, which bakes its manifest path with `env!` and then
  walks a directory under it, was never asked. There is a `pdf-sandbox` build script in this
  machine's build directory naming a checkout that is gone, and the old check could not see it.
  The population is derived now: every tracked `build.rs` whose source expands
  `env!("CARGO_MANIFEST_DIR")`. `crates/pdf-spec/build.rs` reads the same variable through
  `std::env::var_os` and is therefore *not* in it — the discriminator has to come off the source,
  because the compiled binary carries the path in its debug info either way.
- **`tools/worktree.sh list`** globbed the names it invents itself, under a heading about orphans,
  so the only build directory it could ever report was one of its own. It walks the whole build
  root now, says of each directory whether it is the main checkout's (asked of cargo), a live
  round's, an orphan, or one no checkout here names — and totals them, because that is the number
  `doc/todo/02` §5a's threshold is about.
- **`tools/state.sh disk`** reported the round's own `target-dir`, which is right and stays: the
  literal path it used to carry was trap 15 in a document. But from a worktree that is a few
  hundred megabytes while the root holding it is past §5a's hundred gigabytes, and §5a pointed at
  this command for the second number. Both are printed now, one line apart, with the root's line
  guarded so that an ordinary clone — where the parent of `target/` is the repository — prints
  nothing.

A fourth instance was found while editing and is the same trap at its smallest: `worktree.sh`'s
usage text was `sed -n '3,20p'` of its own header, four lines past the block it meant, printing
`set -euo pipefail` at a reader. It reads comment lines until the first that is not one.

## And one the same question found in `state.sh`'s own table

`tools/state.sh windows` counts how much of `viewer-core`'s vocabulary each window reaches and
carries a hand-written *reading* beside the count — one line per unreached variant saying whether
it is a debt — with a check in both directions, so a variant with no reason is named as owing one.
It was naming two: `Command::View` and `Query::View`, printed twice as **UNREAD — this round owes
a reading, in this table**.

They are not a debt, and the reason is this round's own subject one level down. **`Query::View` is
reached — by `pdf-viewer-confined`, which asks it per frame** and echoes the answer back as
`Command::View` so that a worker that dies is one page's breach rather than the document's end
(ADRs 0734, 0737). That window is *deliberately* outside this section's population, being a second
window inside `viewer-ui`'s crate, and the same exclusion keeps `trace.rs` out. So an exclusion
taken for a good reason — do not let a tracer or a second window inflate a parity count — made a
variant a real window does reach print as reached by nobody, and then the table demanded a reading
of a non-debt. Both lines are written now, each naming the exclusion as the reason, and the
condition for deleting them is stated: a *counted* window gaining a worker it can lose.

Nothing else in that table was `UNREAD`, and nothing was `SPENT`.

## What the tree already knew

The argument for all three is written down in the file that turned out to hold two of them.
`tools/worktree.sh`'s gitlink guard was a hand-written list of two corpora while four paths were
being symlinked, and the comment that replaced it says a list written by hand "goes stale the next
time something is linked, which is exactly how this one did". The lesson had not travelled two
functions down the same file, which is why it is a **trap** now rather than a comment: trap 25, *a
hand-written population can name a thing that never existed, and finding nothing there reads as a
pass*. Its sentence is the one that generalises — a narrow population and a clean tree produce the
same output — and it is trap 23's failure with the instrument's input wrong instead of its scope,
and trap 24's with a list instead of a corpus.

## Calibration

Trap 13, on the one check that gained a population, in a scratch build directory with the defect
planted and both versions run over it. A `pdf-sandbox` build script naming a gone checkout: silent
under the old population, failing under the derived one. The same defect in `pdf-font`: failing
under both — the half that proves the derived population did not lose the case the written-down one
held. Both naming live checkouts: silent under both. Reverted afterwards.

No gate is added, deliberately, and `doc/adr/0752` says why: `tools/conformance/tests/workspaces.rs`
exists because §2's command block is prose and cannot derive anything, and none of these three is
prose any more.

## Gates

Documents and `tools/*.sh` only, which `doc/todo/02` §2's map puts under the core plus the
conformance gate, plus `--bin quotations` and `--bin pointers` because documents moved. Not a fifth
round. `cargo fmt --all --check`, `cargo fmt --manifest-path fuzz/Cargo.toml --check`,
`RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`, the same for `fuzz/`,
`cargo nextest run --workspace`, `cargo test --workspace --doc`, and `cargo test -p conformance`:
all green, no lint, ledger unmoved and no row owed — this round touched no clause. The gcc
`-Wmaybe-uninitialized` lines under `viewer-qt` are §2's documented cold-build noise.

## The sweeps, before and after

§4, by the second method: a detached checkout of `3ec61db7` with its own build directory and the
gitignored data symlinked in, the sweep binaries built inside it, and thirteen of the committed
sweeps run on both sides — every one whose population it can derive for itself. `retired` takes its
nouns as arguments and this round retired no mechanism; `quoted` and `unpriced` take an oracle log
this round had no reason to produce.

Eight identical. Five moved, every difference attributable: `counts` gained sentences governing a
ledger word and no attributed count; `parts` gained trap 25 in the handover's index row and shifted
one line number in `doc/environment.md`; `quotations` gained this round's own documents with no new
divergence;
`overtaken` gained one decision record and no overtaken note; `pointers` gained live pointers, one
unrooted path and two metavariable forms, with `absent` unchanged.

**`pointers` earned one correction before that was true.** The ADR's first draft wrote
`tools/conformance/build.rs` out in full to say the file has never existed, and the sweep read it as
a pointer to a missing file — a permanent hit whose subject is that the thing is missing. The
sentence names the directory and asks `git log` for the file instead, and the ADR says why in a
parenthesis. That is the sweep working as designed and it is worth writing down: **a document that
denies a path still contains one.**

The errata sweeps were not run: this round implemented no clause, and `tools/spec-errata` is a
sibling's lane this batch.

The baseline checkout and its build directory were removed together afterwards, which is
`tools/worktree.sh close`'s own rule applied by hand to a detached checkout the script did not make
— itself an instance of this round's subject, and the reason the widened `list` now prints such a
directory instead of passing over it.

## Owed, carried forward

- **`target/` still holds none of §5's binaries in this worktree**, and `tools/round.sh` says
  `target/pdf-viewer is older than HEAD` in the main tree. No measurement of the program was made
  here — the only numbers this round took are `du`'s — so §5's rebuild was not owed and was not
  run. It is the merge round's.
- **The build root is past `doc/todo/02` §5a's hundred gigabytes** and has been for some time with
  no instrument saying so. Both instruments say so now. **The sweep itself is not this round's**:
  §5a's command names the main tree's subdirectories, four sibling rounds are building beside this
  one, and most of what is up there belongs to directories no checkout in this repository names —
  `quorra`, `quorra-main`, `quorra-mask-round`, `probes-round`, `hayro`. A round that sweeps them
  is deciding about somebody else's work and should say so first.
- **`doc/rfc/` and `doc/todo/56` untouched**, both awaiting the owner.
