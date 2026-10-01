# 1475 — A command a live document writes names what the tree has, and a trap's command is kept current while its story is not

Session 1320. Status: accepted. Extends ADR 1403's population (`tests/variables.rs`) from a knob to
the rest of the command line, and states for traps and habits what ADR 1023 left implicit. Code:
`tools/conformance/tests/commands.rs`, `tools/state.sh` (`prose` gains its `commands` line). Prose:
`doc/todo/01`, `doc/todo/48`, `doc/ledger-and-claims.md`, `doc/traps/instruments-and-reports.md`,
`doc/traps/pixels-and-rasterisers.md`, `doc/verify.md`, `doc/PLAN.md`.

## 1. The question

ADR 1463 moved the conformance sweeps to the `dev` profile and named five documents that still
wrote `cargo run --release -p conformance`. They ran — slower to build by two minutes after any
edit to the crate — and nothing could see them, because no test reads a document's command. The
general shape is `variables.rs`'s: a command in a document is copied, not read, and a copied line
that names a renamed crate, a deleted example or a profile nobody prescribes either fails at cargo
or spends what nobody asked for.

## 2. What the sweep asks

Every `cargo run|test|nextest run|bench|build|check|clippy` in a live document (`variables.rs`'s
population: `CLAUDE.md`, `doc/` less records and `doc/questions/`, `raster/`'s two documents,
everything under `tools/`) is read to the end of its code span — across the prose's line wrap, the
case the first draft missed on a dozen lines of `doc/todo/01` — or of its shell line, `\`
continuing it. `-p` names a workspace member; `--example`, `--test` and `--bin` name a target of it
by cargo's own resolution (a file, a directory with `main.rs`, a declared table, or the package's
own name for an unclaimed `src/main.rs`); a `--release` on a `run`/`test`/`bench` line beside a
package `doc/verify.md` runs only under the default profile is a finding, since `doc/verify.md`
is where a command's profile is prescribed. A `build` keeps its `--release`: it is asked for a
binary a person or a script runs, and `tools/drive-windows.sh`'s three windows are that. A
`tools/<name>.sh|.py` anywhere in the population must exist. One document is excused by name:
`doc/JPEG2000_FEEDBACK.md`, written to `hayro-jpeg2000`'s maintainer, whose `-p hayro-jpeg2000` is
that repository's package. It is a gate — zero findings — with a planted twin (trap 13).

## 3. What it found, and what was done

42 findings on the first full read, every one fixed. 40 were `cargo run --release -p conformance`
(`doc/todo/01` 35, a dozen of them wrapped across a line; one each in `doc/todo/48`,
`doc/ledger-and-claims.md` and `doc/traps/instruments-and-reports.md`; two in
`doc/traps/pixels-and-rasterisers.md`), and `doc/todo/01`'s base-commit recipe also wrote `cargo
build --release -p conformance --bins`, made `dev` beside them for the same reason. The other two:
`doc/verify.md`'s `outline_census` under `viewer-gtk`, which lives in `viewer-host`; and `doc/PLAN.md`'s `--example spike-window`, an example deleted when `viewer-ui`
became the viewer — the sentence around it, which said the window calls `render_gpu::build_scene`,
was false too and now says what the window is. The first draft also counted `pdf-sandbox`,
`viewer-qt` and `viewer-ffi` as dev-only off `doc/verify.md`'s `check` and `build` lines; the rule
reads `run`, `test` and `bench` lines only, which is what a profile is prescribed for.

## 4. A trap's command is an instruction; its story is a record

`doc/traps/` and `doc/habits/` are incident records with a rule attached, and `CLAUDE.md` keeps
their history because the history is why anybody believes the rule. But the command inside one —
"take the path from the toolchain: `cargo run -p conformance --bin …`" — is not part of the
incident. It is the remedy, and a reader runs it today against today's tree. So the sweep reads
traps and habits, and a finding in one is fixed in the command alone: the sentence saying what
happened, the date, the figure the round saw, stay as written. A command that is itself the
evidence of the incident — the line somebody ran that did the damage — would be quoted as evidence
and not as an instruction; none of the three trap findings was that, and a future one is written
in a code span after "ran" or "typed", which this sweep would still read: such a line is to be
written with the placeholder form (`<crate>`) or outside the population, rather than excused here.
