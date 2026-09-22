# 1274 — A file that cites a clause, against that clause's own `code` list

Status: accepted and **built**, report-only with its calibration gated. Session 1218.
Context: `tools/conformance/src/cited.rs`, `tools/conformance/src/bin/cited.rs`,
`tools/conformance/tests/cited.rs`, `tools/state.sh`, `doc/todo/01`,
`doc/conformance/ledger.toml` (§12.7.3, §7.9.6 and §14.13.2's `code` lists).
Builds on: trap 11 (a report is only as good as the condition it fires on), trap 13 (a sweep is
calibrated against the defect before it is believed), trap 25 (a hand-written population decays),
`doc/adr/0372` (`--bin pointers`, which ranks rather than fails for the same reason).

## 1. The shape, found by reading

Session 1210 read `pdf-transform`'s archive converter and found it reads §12.7.3's interactive form
dictionary, §7.9.6's name trees and §14.13.2's embedded associated files — citing all three by
number, as `CLAUDE.md` principle 5 requires — while not one of the three rows named a file of that
crate. The converter had been there for batches.

**A `code` list decays in one direction only, and that is what makes it sweepable.** A round adding
a reader writes the citation beside the code, because the citation is required and the code is what
it is writing. Editing a row in `doc/conformance/ledger.toml` is a second file, in a second format,
about work already finished — the step a round forgets. So the citation is the live half and the
list is the half that goes stale, and the sweep runs from the live half to the stale one.

Nothing asked this before. The ledger gate checks that every path a row names **exists**, and its
`CitedButUnreviewed` check asks a narrow version of the reverse — a clause the code cites may not be
left `unreviewed` — but neither asks whether a row that claims work names the files that read it.

## 2. The discriminator is two decidable things, and neither is the citation's meaning

A `§` in a comment is not a claim to implement the clause. This tree cites a clause to say what a
value is *not*, to point at a neighbour, to explain a refusal. Nothing mechanical separates those
from an implementation, so the sweep ranks on what is decidable:

- **What the row claims.** Only `implemented`, `partial` and `departed` have a `code` list that is
  supposed to be complete. A row claiming no work has nothing for a file to be missing from, and a
  citation under one is usually a comment saying why.
- **How often the file cites it.** One `§` is a cross-reference as often as a reading; three or
  more is a file reading the clause. `REPEATED` is the rank's only number and it moves nothing
  else.

Two rungs, closest first:

- **`UnknownCrate`** — the row's `code` names no file of the reading crate **at all**. Session
  1210's own shape, and the one that says a whole reader is invisible to the row.
- **`UnknownFile`** — the row names the crate and not this file.

A test, an example, a bench and a fuzz target are not read at all: a `test` array is where those
go, and reporting them would ask a row to name the same file twice.

**The member a path belongs to is chosen by the longest member directory**, derived from the
manifest, not by looking for Cargo's `src/` and `tests/` segments. The first draft did the latter
and put `crates/viewer-qt/src/host.rs` on the wrong rung, because that row's listed site is
`crates/viewer-qt/cpp/window.cpp` — the C++ half of the same crate, which a segment list reads as
no crate at all. Trap 25 in miniature.

## 3. Why it prints rather than fails

`--bin pointers`' reason in the other direction. The sweep cannot tell an implementing citation
from a cross-reference, so a build that failed on a hit would teach rounds to drop the citation
principle 5 asks for — which would cost the project the live half of the comparison and leave the
stale half alone. It prints, `tools/state.sh cited` shows it, and a person reads.

What **is** gated is that the instrument still works, which is trap 13's rule rather than a claim
about the tree.

## 4. Calibration

A row loses one of its own citing files from its `code` list, in memory, against the real tree, and
the sweep is asked whether it now names it. **The pair is chosen by the sweep rather than written
down**: the first library file the ledger already names for a clause it reads `REPEATED` times or
more. A hand-written pair rots into a calibration that has quietly stopped calibrating, which is
trap 25 pointed at the instrument's own fixture. `tests/cited.rs` fails unless the plucked pair
lands on a rung and the intact pair lands on none.

## 5. Its first ten hits, read

Eight of ten were the rule:

- §11.6.6 read 24 times by `pdf-colour/src/colour.rs`, §7.3.3 21 times by
  `pdf-colour/src/function.rs`, §11.4.7 18 times by `pdf-model/src/content/transparency.rs`,
  §8.9.5 17 times by `pdf-transform/src/redact.rs`, §12.7.7 16 times by
  `pdf-signature/src/revision.rs`, §8.5.4 14 times by `render-cpu/src/lib.rs`, §11.3.5.2 14 times
  by `raster-scene/src/blend.rs`, §11.6.5.1 13 times by `pdf-render/src/soft_mask.rs` — each under
  a row whose `code` names no file of that crate.
- The two that were not: `pdf-archive/src/table/graphics.rs` citing §8.6.5.5, where the validator
  names a clause it **checks a document against** rather than implements; and
  `viewer-qt/src/host.rs` citing §12.3.4, which section 2's member rule then took off the top rung.

Those rows are other rounds' to edit. The three this round owned are closed: §12.7.3 and §7.9.6
now name `crates/pdf-transform/src/archive/sites.rs` and
`crates/pdf-transform/src/archive/rewrite.rs`, which read both, and §14.13.2 names `rewrite.rs`,
which is the only one of the two that reads it. That is session 1210's finding closed.
