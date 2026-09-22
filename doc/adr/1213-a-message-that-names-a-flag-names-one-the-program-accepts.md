# 1213 — A message that names a flag names one the program accepts

Status: accepted and **built**. Session 1188.
Context: `tools/conformance/src/flags.rs`, `tools/conformance/src/bin/flags.rs`,
`tools/conformance/tests/flags.rs`, `tools/state.sh`, `doc/todo/01`.
Builds on: trap 11 (a report is only as good as the condition it fires on), trap 13 (a sweep is
calibrated against the defect before it is believed), trap 25 (a hand-written population decays in
both directions), `doc/adr/1024` §4 and `tools/conformance/src/roots.rs` (derive the population
from the manifest).

## 1. The defect, and why nothing could see it

Session 1181 found five refusal messages of `quorra-transform` telling a user to supply a missing
font with `--font`, and two documents saying the same. The program's `KNOWN` list did not hold that
flag, so the command each message asked for was refused as a usage error.

Nothing checked it because the two halves are not in one place. The accepted set is a `const` array
in `crates/pdf-transform/src/bin/quorra-transform.rs`; the messages are string constants in
`crates/pdf-transform/src/archive/{fonts,decision,prepare}.rs`, which is a different file and, for
other programs of this tree, a different crate. A message that names a flag the program refuses is
worse than no message: it spends the reader's next attempt and reads exactly like one that works.

## 2. Both populations are derived

- **The programs**, from the workspace's members and each member's own manifest: every `[[bin]]`
  block, plus cargo's automatic targets — `src/main.rs` named after the package and each
  `src/bin/<name>.rs` or `src/bin/<name>/main.rs` named after its file or directory. Two of this
  tree's binaries are renamed by a `[[bin]]` block (`quorrafs`, `quorra-retrieve`), so a list taken
  from filenames would name two programs that do not exist — trap 25's shape, and the reason
  `roots.rs` exists.
- **The accepted flags**, from each binary's own source set: its root file and every module it
  declares, followed through `mod x;` and `#[path = "…"] mod x;`. A comparison against argv is
  written nowhere else. The cost is stated rather than hidden: a flag named in the binary's own
  source and never compared reads as accepted, so the sweep errs towards silence exactly where the
  program's own file mentions the flag.

An empty derivation is [`Error::NoBinaries`] rather than a clean answer, and the gate asserts that
sources and documents were actually read.

## 3. What makes a mention a *message*, and what each condition cost

The first run attributed every flag in every string literal of a crate to that crate's binaries and
produced 80 findings, of which none was the rule. Three conditions narrow it, and each was added
because the run without it produced findings that were not the rule:

1. **A `#[cfg(test)]` module is not the program.** A test builds command lines for `qpdf` and
   asserts on sample TOML; neither is a message a user is shown. This alone took 80 to 24.
2. **A literal with no words around the flag is an argument, not a message.** `.arg("--silent")` is
   what this tree hands `curl`; "supplying the font with --font resolves it" is a sentence
   addressed to a person. Twelve of the remaining findings were curl's flags.
3. **A command span belongs to the program it names.** Where the flag stands in a run of
   command-like tokens — backtick-quoted, or a literal that is all words and flags — the program is
   the span's **first** token, unless a bare `--` stands before the flag, in which case it is the
   last binary of this tree named before that `--`. `cargo build -p pdf-sandbox --bins` is cargo's
   line; so is `cargo +nightly build --release --bin quorra --unit-graph`, all of it, which is why
   the rule is not "the last binary named"; and `cargo run -p pdf-transform --bin quorra-transform
   -- archive --remedy-sites` hands the rest to ours at the `--`.

**Attribution outside a command span is by crate, not by name on the line.** Name-on-the-line was
tried first and is trap 11's sixth instance: several of this tree's binaries are named for ordinary
English words, and `"--every counts from 1"` — a `quorra-transform` usage message — was attributed
to the `counts` binary on the word *counts*. A message a program prints is written in its own
crate, so a flag in prose in crate C is asked of the binaries C declares, and it is a finding only
when not one of them accepts it.

**A document has no crate, so there condition 3 is the whole of the attribution.** A `doc/*.md`
line is asked about a flag only where the command span names one of these programs, and a
`\`-continued fence line is read with the line it continues so the program is in the span.
Attributing a document's line to every binary a fenced block mentions was tried and is the same
trap: `doc/verify.md`'s command listing names two dozen programs in one fence, so every cargo and
busctl flag in it became a finding against all of them. The cost is stated: a document that names a
flag in prose without naming the program is not asked about — which is where the two `--font`
documents sit, and both of them say in as many words that the flag is not accepted yet.

## 4. The gate may fail rather than ratchet, and it is green

There is no version of this tree in which a message naming a refused flag is acceptable, so the
gate holds the population at **zero** and prints file, line, flag and message on a failure. That is
affordable because the population *is* zero: round 1186 added `--font` to `VALUED` and `KNOWN`
while this sweep was being built, and the sweep went from naming six sites to naming none — which
is the instrument's own before-and-after and is better evidence than a plant.

**The plant is still run, every time.** `flags::calibrate` puts a flag no program accepts into both
attribution paths — a message of the defect's own shape, and a documented command line naming a
real binary — and `tests/flags.rs` fails unless the sweep names both. It plants into the functions
rather than into a file of the tree, because a plant written into the tree is a plant somebody has
to remember to remove; and the flag is assembled from two pieces so that this crate's own source
does not carry a message this sweep would, correctly, report.

## 5. What is deliberately not done

- **Flags are not run.** The accepted set is read out of the source, never by invoking a binary
  with `--help`: running 38 programs in a gate is a cost and a hazard, and a program that refuses to
  start would read as a program with no flags.
- **Short flags are not checked.** `-o`, `-p`, `-h` are one character and collide with prose,
  hyphens and every other program's arguments; the discriminators above are all about words.
- **`doc/`'s subdirectories are not read.** ADRs, histories and reviews are records, and a record
  states what was true when it was written. A flag named in one is not a promise to a reader today.
