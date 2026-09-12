# 1000 — The signature stack leaves `pdf-model`, and the measurement says two call sites keep its packages there anyway

Date: 2026-09-12. ADR: 1020. Round 1000, in the worktree `/home/AI/pdf-viewer-rounds` on branch
`batch-999-1004`, with five sibling rounds live in the same tree.

The first half of ADR 1005 §1, which `doc/reviews/984-direction-and-boundaries.md` finding 1
ranked first: `crates/pdf-signature` exists, holds ten modules and 9,968 lines, depends on
`pdf-syntax` and on nothing else of this tree, and takes the twelve cryptographic packages with
it. The finding's own prediction about the ledger was exact — **72 rows** name a path under the
moved `src/` modules; two more name only the test file that moved with them, so 74 lines were
rewritten and `the_ledger_agrees_with_the_standard_and_with_the_tree` held all of them.

The finding's prediction about the *dependency graph* was not exact, and that is this round's
finding. `tools/spec-errata` still compiles `p521`, by
`p521 -> pdf-signature -> pdf-model -> spec-errata`: `pdf-model` keeps two call sites in §12.8 —
`restriction.rs` for §12.8.2.2's `/DocMDP` level and its field locks, `view.rs` for §12.8.6's
usage rights — because what a document says may be changed without invalidating its author's
signature is a restriction on the reader and `Restriction` is where this tree collects those.
A third site was an accident and is gone: `icc.rs` reached into `cms::Digest::Md5` for ICC.1
section 7.2.18's profile identifier, which is a content hash with no signature near it, and it now
calls `md-5` directly. What a second extraction would buy is measured rather than guessed —
**33 packages**, and **three** of the thirteen the review named do not leave, because `pdf-syntax`
needs `const-oid`, `sha2` and `md-5` for §7.6. ADR 1020 §2 names the type that blocks it.

## Files

New: `crates/pdf-signature/{Cargo.toml,src/lib.rs}`,
`doc/adr/1020-the-signature-stack-leaves-and-two-call-sites-keep-its-packages.md`, this file.

Moved by `git mv`, so `--follow` still reaches every decision above them:
`crates/pdf-model/src/{signature,cms,x509,der,bigint,pkcs1,pss,dsa,ecdsa,eddsa}.rs` →
`crates/pdf-signature/src/`, `crates/pdf-model/tests/signatures.rs` →
`crates/pdf-signature/tests/`, `crates/pdf-model/examples/signature_algorithm_census.rs` →
`crates/pdf-signature/examples/`.

Changed: `Cargo.toml` (the workspace's dependency table), `crates/pdf-model/Cargo.toml` (eleven
cryptographic dependencies out, `sha2` down to a dev-dependency, `pdf-signature` in),
`crates/pdf-model/src/lib.rs` (ten module lines out, nothing re-exported),
`crates/pdf-model/src/{icc,restriction,view,attachment,integer_entry,named_page,form}.rs` (the
reach into §12.8, one line each but `restriction.rs`'s four),
`crates/pdf-model/{tests/forms_data.rs,tests/restrictions.rs,examples/absence_audit.rs,examples/open_cost.rs}`
(imports), `crates/{viewer-core,pdf-transform,pdf-archive}/Cargo.toml` and the six consumer files
in them, `fuzz/Cargo.toml` and `fuzz/fuzz_targets/{cms,x509}.rs`,
`doc/conformance/ledger.toml` (74 lines of paths and 22 of prose), `doc/crate-map.md` (three
rows), `doc/stack.md`, `doc/todo/01-ledger-partial-rows.md` (one path),
`doc/todo/02-every-round.md` (§2 rule 2's list of crates under everything, seven to eight).

## The crate map had two defects and both are fixed

The review found them and this round owed the rows: `doc/crate-map.md` opens "one row per crate"
and had **no row for `pdf-archive`** (31,207 lines, the validator) while its row for the third
rasteriser named **`render-quorra`, a crate that does not exist** — the crate is `render-raster`.
Both are the same defect pointing in opposite directions and both are found by one command,
`cargo metadata --no-deps` against the file. Running it leaves **five** rows still owed, all under
`raster/`: `raster`, `raster-gpu`, `raster-pages`, `raster-scene`,
`raster-function-conformance`. They were folded into this workspace from their own and no round
has written their rows; that is a residue this one names rather than one it hides.

## What was in the way, and what it says about a batch

Five siblings edited this tree while this round ran, and three of them broke a build this round
had to pass through: `pdf-model/src/content/transparency.rs` (1002), `pdf-transform/src/archive/`
(999) and `pdf-archive/src/reach.rs` (1001, an unclosed delimiter in a new file). Each cleared on
a re-run, which is the rule `doc/todo/02` states and it held. One of them was worth more than the
inconvenience: the `raster_golden` gate failed on **231 moved pages** at the first attempt and
passed with **974 held, 0 moved** twenty minutes later, which is exactly the evidence that this
round's change is pixel-neutral — and is also a reminder that a gate run beside a neighbour's
half-finished commit measures the neighbour.
