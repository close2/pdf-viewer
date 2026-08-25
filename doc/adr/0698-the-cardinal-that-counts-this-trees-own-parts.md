# ADR 0698 — The cardinal that counts this tree's own parts, and the mirror whose two sides agree

Status: accepted.
Session: the seven-hundred-and-sixty-ninth, a sweep round under `doc/todo/01`'s binding rule.

## 1. What this decides

1. **`doc/todo/01` gains a twenty-second sweep, `cargo run --release -p conformance --bin
   parts`** — the seventeenth of them to be a program, and the first whose right-hand side is not
   the standard, the ledger or the tree's prose but **the workspace's own membership**.
2. **The other sweep the seven-hundred-and-sixty-seventh session proposed is not built**, and the
   reason is a measurement rather than a preference: its two sides state the same sentence, word for
   word, so it would have counted 767's own finding as agreement.
3. **A round that adds a part to this tree runs the new sweep**, because that is the one moment at
   which every sentence counting the old population goes wrong at once.

## 2. The shape the sweep exists for

ADR 0697 found §8.9.6.2's interpolation `shall` answered in two ledger rows by naming the raster —
correctly — and then written down as **"both backends"** when there are three, with the third
departing by 131 of 255 on the painted channel. The same two words stood in `pdf-render`'s
`Image::is_smoothed` doc comment, which is the item all three backends call.

**No instrument could see it, and the reason is structural.** `--bin counts`, the tenth sweep,
reads a cardinal only where it governs one of the ledger's own words for a **row** — `row`,
`subclause`, `child`, `below`. "Backends" is not one of them and was never going to be: the tenth
sweep's answer side is a *family of ledger rows*, and the population "both backends" counts is a
family of **crates**. Nothing in `tools/conformance` had ever asked the workspace a question.

It is worth naming what kind of defect this is, because it decides how the output is read.
**"Both backends" was true when it was written.** The tree then grew a third rasteriser and every
sentence that had counted two became wrong without anybody touching it. So this is a **decay
detector** rather than a mistake detector, and the population it walks is mostly correct sentences.
That is not a flaw to be engineered away; it is what the instrument is, and the design question is
therefore *ordering* rather than *filtering*.

## 3. The two sides, and why both are derived

**The answer** is the workspace, read off the files: the member directories under `crates/` and
`tools/`, each package's `src/bin/`, and `.gitmodules`. A backend is a member whose package name
begins `render-`; a host is a member with a `src/bin/` program named `pdf-viewer…`; a worker is one
named `…-worker`. Not a number in the program: ADR 0397's lesson is that a right-hand side written
from memory measures the session that wrote it, and this one is a `read_dir` away.

**The claim** is the harder half, and two rules decide it. Each removes a larger population than it
keeps, which is the point.

- **The noun follows the number immediately.** "Both **native** hosts" is right about two of three
  hosts, and a reach of even one word would call it wrong. Adjacency is the whole of what keeps a
  restrictive modifier out, and it is cheap because this project writes its modifiers in front.
- **The form presupposes the size.** `both`, `neither`, `either`, or a cardinal under a definite
  article. "Two backends draw the seam" *counts two of them* and claims nothing about how many
  there are; "the two backends cannot answer it differently" says the set has two members. Only the
  second is a claim the workspace can answer.

The second rule is worth a number, because it was measured rather than assumed. Reading bare
cardinals as well put **293 further disagreements** into the first run — 791 against 534 — and the
sample was a count of a subset every time: a host pair, a crate pair, the two rasterisers a
comparison names. Judging one would be a guess about what an English sentence governs, which is the
judgement every sweep in `doc/todo/01` refuses by construction.

## 4. The rungs, and the one derived from the crate graph

Three rungs, closest first.

1. **The place is a crate the whole population depends on.** A comment there cannot be talking
   about a chosen pair, because every member of the population is downstream of the sentence.
   `pdf-render` is exactly that crate for the backends — all three depend on it and it depends on
   none of them — and `paint.rs`'s `Image::is_smoothed` is on this rung.
2. **The ledger, or one of this project's undated documents.** A ledger note describes what *the
   tree* does, so a count in one is a claim about the whole population.
3. **A dated record, or a place inside the population.** `doc/adr/` is dated by its own numbering,
   which is the nineteenth sweep's rule, and an ADR written before a part existed counted correctly
   at its date. A comment inside one backend is usually about the pair under comparison.

**The first rung is the one worth the code**, and it is derived rather than declared: the sweep
walks each member's manifest for the other members it names and asks whether every member of the
population reaches the place's crate. That is a fact about the dependency graph, so it moves when
the graph moves. On the first run it separates **52 hits from 534**, and 767's defect is among the
52 — a factor of ten for about forty lines.

**The third rung is counted rather than listed**, on the nineteenth sweep's own precedent for its
last rung. Listing 323 correct sentences above the two rungs a round can act on would be the ratio
argument ADR 0249 makes, paid in full.

## 5. Its first run

52 on the closest rung, 159 in the ledger or an undated document, 323 counted. 572 forms presuppose
a population at all and the workspace agrees with 38 of them.

`crates/pdf-render/src/paint.rs` alone carries eight, including the one ADR 0697 named and left
standing. Reading the rung, the shape repeats: `pdf-render` is the crate that exists so that *a
decision either backend can make alone is not made alone*, and its doc comments say "both backends"
about items — `Image::is_smoothed`, `Clip::admits_nothing`, `collapsed`, `Stroke::device_width`,
`Image::area_averaged`, `SoftMask::values` — that all three backends call, checked by grep over the
three backend crates' sources. **The correction is not this round's**: a change to `pdf-render` is a
change to a crate `doc/todo/02` §2 says can move a pixel, so it owes the whole gate sequence, and
three rounds were running beside this one on a machine where a gate that spawns a reference renderer
is a measurement of two programs. The reading list is what the sweep prints.

**Three noise shapes, named because a sweep that does not state its own noise gets switched off:**

- **A modifier that *follows* the noun.** Adjacency stops "both native hosts"; it does not stop
  "four submodules under `doc/corpora/`", which is right about the four there. This is the one
  direction the sweep is loose in, and it is left to the reader exactly as the eighteenth sweep
  leaves a partitive with no table to divide it.
- **This project's own aphorism.** Trap 2's "a decision either backend can make alone is a decision
  neither has made" is a *rule*, written verbatim in six files, so it arrives as six hits that are
  one sentence.
- **A round's own record of running it.** ADR 0697's own paragraphs are on rung 3 and this one's
  will be too, which is the ninth sweep's oldest habit arriving unchanged.

**Calibrated per trap 13 against the live defect rather than a plant**, which was available because
767 recorded the doc comment and did not correct it: `paint.rs:564` is rung 1 today; rewritten to
name three backends it leaves the rung and the agreeing count rises by one; restored, it is back.

## 6. The sweep that is not built, and why the measurement is one line

767's second proposal was **a parent restating a child's refusal and dropping the condition the
child stated it under**. It is not ADR 0481's mirror — there the two rows take opposite stances and
the population was fourteen term-mentions. Here both rows deny, and the question is whether the
parent's denial is the wider.

Restored to what they said before 767, the three §8.9.6 rows read:

| row | its refusal |
|---|---|
| §8.9.6 | "§8.9.6.2 refuses a stencil under a *graphics-state* soft mask, which would be two masks on one command" |
| §8.9.6.1 | the same clause, word for word |
| §8.9.6.2 | "One case is still refused by name: a stencil under a *graphics-state* soft mask, which would be two masks on one command." |

**There is no widening to detect.** The three sentences state one claim in identical words, so a
program comparing a parent's denial with a child's would count them as agreeing — and by its own
lights it would be right. What bounded the refusal was two paragraphs *earlier in the child's note*,
about the pattern recomposition that needs the mask slot the graphics state is already using, and
the condition was never in the refusal sentence at all. 767 found it by reading `content::image`'s
own `if`.

So **the sweep would not have printed the finding that motivated it**, which is session 701's
clincher arriving for a second sweep and is a stronger reason to decline than a small population is.
The correct answer lived in the *code*; no sweep whose two sides are both ledger rows can reach it.

It is revisitable on one condition, stated so that a later round need not re-derive the argument:
if a note is written that states a refusal's condition **inside the refusal sentence**, the two
sides differ and there is something to compare.

## 7. What this costs

The sweep is a fraction of a second and is not a gate, for ADR 0249's ratio argument sharpened by
what a decay detector is: a parent row is allowed to summarise and a comment is allowed to be about
a pair, so a hit is a reading list.

Two populations it does not read, each stated rather than quietly skipped. `window` is left out
although the workspace states the hosts, because a host may open as many windows as a person asks
for — offering the membership there would be a *wrong* right-hand side rather than a missing one.
`module`, `panel`, `gate` and `sweep` are populations this project counts in prose and no file
states, so a cardinal governing one of them is judged by nothing. The last of those is not an
academic example: this file's own `doc/todo/01` header carried "eighteen sweeps" and "thirteen of
them are committed programs" while the catalogue below it numbered twenty-one, and the sweep this
ADR builds is by construction unable to see it. Corrected by hand, in this round.
