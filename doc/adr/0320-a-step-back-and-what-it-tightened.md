# 0320 — A step back, and what it tightened

**Status.** Accepted.
**Context.** The project owner asked the question this file answers: whether the tree is still on
its stated path, or has drifted into patching for missing features without anybody seeing the
whole picture. The round read the last fifteen commits at the diff level, surveyed the workspace's
shape (sizes, layering, lint posture, special-casing), and read the planning documents against the
gates. The verdict first, because it is the context for every decision below: **the drift the
question fears is not happening.** Where this tree disagreed with the references, the fix was
derived from the clause (ADR 0307's backdrop); where a census found zero witnesses, the clause was
implemented anyway from hand-built fixtures (ADRs 0315, 0316); where a bound was wrong, the unit
was corrected and the value deliberately left alone (ADR 0308's neighbour, session 471). No
production path branches on a document's name; the three `#[allow]`s in the tree are the two
documented unsafe-module exemptions and their test. What the review *did* find is below, one
decision per finding.

## The decisions

**1. A merge runs the gate sequence on `main`** — `doc/todo/02` §2 now says so. The 455–484 block
ran eleven rounds in parallel worktrees and the closing round found `clippy --workspace` had been
broken on `main` for five of them, while every parallel round truthfully recorded its own worktree
silent. The block summary stated the rule; nothing bound it. Now the file every round opens does.

**2. A sweep round commits one prose sweep as a program before running any** — `doc/todo/01` now
says so. ADR 0319 established that a described sweep decays into a reconstruction; thirteen of
the fifteen are still descriptions. One per sweep round retires the backlog without a marathon.

**3. A commit that lands on `main` keeps its body** — `doc/environment.md`'s working agreements.
Four commits arrived by cherry-pick carrying a title and trailers only. The history is not
rewritten for it; the agreement binds the next pick.

**4. No further zero-witness algorithm family before a witnessed one** — `doc/todo/51`. DSA
(zero corpus signatures) landed before RSASSA-PSS (six documents, the commonest thing the
program declines). ADR 0314's work is sound; its *ordering* inverted the two-track rule inside
one family, and the file that ranks the family now says so. Whether `bigint.rs` itself stays
in-tree or yields to a reviewed dependency is the owner's call and is deliberately not decided
here.

**5. Three files grown past their shape are split, as pure moves.** `pdf-model`'s `content.rs`
was twice the size of the workspace's next file; `viewer-ui`'s `pdf-viewer.rs` held a whole host
in one binary file while GTK and Qt were decomposed through `viewer-host`; `pdf-font`'s crate
root had kept what its siblings gave away. Principle 4 is the argument and the whole argument —
no behaviour, name or bound changes, which the unchanged gate output is the proof of. Each root
file keeps its name and becomes its module's front door, so every ledger `code = [...]` entry and
doc citation of those paths stays true — chosen over a rename exactly because sweep 8 reads
paths in prose.

**6. The interactive surface gets an instrument, and the design comes first** —
`doc/todo/05-an-instrument-for-the-interactive-surface.md`, new. The block summary's sentence —
the gates measure a raster and the work has moved off the raster — is the strategic finding of
the review, and the one thing in it that no round-sized fix answers. The todo file holds the
three candidate instruments and what a design round must settle; nothing is built before that
round ends in an ADR, because an instrument built casually becomes a corpus worshipped
accidentally.

**7. `hayro-jpeg2000`'s `rev` moves to `1dc833f7`** — `doc/todo/24`'s step 0, unblocked the day
the owner pushed the branch. The JPEG 2000 gate's output is unchanged to the line, including
`issue19517.pdf`'s refusal: nothing asks for a reduced resolution level yet, and the edits that
will are `doc/todo/24`'s, in order, no longer blocked on anything.

## What was reviewed and deliberately not changed

The two-track selection, the ledger, the raster oracle, the closing-round mechanism, the
dependency posture, and the refusal ratchets — the review looked for drift in each and found the
design working as stated. A step back that changes nothing it examined is a result too, and this
paragraph is where it is recorded.
