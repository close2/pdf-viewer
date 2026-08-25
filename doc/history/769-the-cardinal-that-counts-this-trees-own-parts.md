# 769 — The cardinal that counts this tree's own parts

A sweep round. The seven-hundred-and-sixty-seventh session proposed two instruments, each of which
would have predicted one of its own findings; this round measured both, built one and declined the
other with the numbers written down.

Date: 2026-08-25.
ADR: [0698](../adr/0698-the-cardinal-that-counts-this-trees-own-parts.md).

Touched: `tools/conformance/src/parts.rs` (new), `tools/conformance/src/bin/parts.rs` (new),
`tools/conformance/src/lib.rs` (one module line), `doc/todo/01-ledger-partial-rows.md`,
`doc/todo/02-every-round.md` §4, the ADR and this file. **No library source is touched**, and
`crates/pdf-render/src/paint.rs` was restored byte for byte after the calibration below — no pixel
this tree draws can have moved.

## What already existed, because the briefing asked

`doc/todo/01` line 553 does describe a first run that "found §8.9.6.1 … and a second understating
row beside it", and it is **not** an understating sweep. It is the *fourth* sweep — the retired-string
grep, `--bin retired` — recorded in the two-hundred-and-sixteenth session, and "understating" there is
a description of what one of its hits turned out to be rather than the name of an instrument. So
neither of 767's two proposals was built, and `git log` confirms `--bin overstated` and its module
were byte-identical from the six-hundred-and-forty-fifth session to this round.

## What was built

**`--bin parts`, the twenty-second sweep and the seventeenth to be a program.** A cardinal governing
one of this tree's own parts, against the workspace's own membership. The answer side is a
`read_dir`: member directories under `crates/` and `tools/`, each package's `src/bin/`, and
`.gitmodules`. The claim side has two rules and each throws away more than it keeps — the noun
follows the number *immediately*, and the form must **presuppose** the size rather than count a
subset.

Three rungs, and the first is the one worth the code: **the place is a crate every member of the
population depends on**, derived from the manifests, so no pair can be what the sentence means.
`pdf-render` is that crate for the backends.

## What it prints

52 on the closest rung, 159 in the ledger or an undated document, 323 counted rather than listed.
572 forms presuppose a population at all and the workspace agrees with 38 of them.

**It prints 767's finding.** `crates/pdf-render/src/paint.rs`'s `Image::is_smoothed` doc comment is
rung 1, rank by location, and the two ledger rows §8.9.6 and §8.9.6.2 that still carry "both
backends" are on rung 2. `paint.rs` alone carries eight, about items — `Image::is_smoothed`,
`Clip::admits_nothing`, `collapsed`, `Stroke::device_width`, `Image::area_averaged` — that all three
backends call, checked by grep over the three backend crates.

**The correction is left to a round that can pay for it.** A change to `pdf-render` is a change to a
crate `doc/todo/02` §2 says can move a pixel and therefore owes the whole gate sequence, and three
rounds were running beside this one — §2's own warning is that a gate spawning a reference renderer
on a loaded machine reports a regression in the thing being measured. The reading list is what the
sweep prints, which is the point of having built it.

## What was declined, and with what numbers

**An understating parent — a parent restating a child's refusal without the condition the child
stated it under.** Not ADR 0481's mirror, which was measured at fourteen term-mentions and declined
in the six-hundred-and-fifty-second; here both rows deny and the question is which denial is wider.

The measurement is one line. Restored to what they said before 767, §8.9.6, §8.9.6.1 and §8.9.6.2
state the refusal in **identical words** — "a stencil under a *graphics-state* soft mask, which
would be two masks on one command" — so a program comparing them would count them as agreeing, and
would be right by its own lights. What bounded the refusal was two paragraphs earlier in the child's
note, and the condition was never in the refusal sentence at all; 767 found it by reading
`content::image`'s own `if`. **The sweep would not have printed the finding that motivated it**,
which is session 701's clincher for a second instrument.

## Two things the measurement itself found

- **The plain cardinal is not a claim about a population.** Read as one, it put 293 further
  disagreements into the run — 791 against 534 — and the sample was a count of a subset every time.
  The rule that removes it is grammatical rather than lexical, which is why it is stated as a
  presupposition test and not as a stop-list.
- **The crate graph is a discriminator and nobody here had used one.** Every other sweep's rungs are
  about words. This one's closest rung is a fact about dependencies, and it separates 52 hits from
  534 for about forty lines.

## Calibration, gates and sweeps

**Trap 13, against the live defect rather than a plant**, which was available because 767 recorded
the doc comment and did not correct it: `paint.rs:564` is on rung 1; rewritten to name three
backends the hit is gone and the agreeing count rises 38 → 39 with rung 1 falling 52 → 51; restored
from the copy taken before the plant, both figures return and `git diff --stat` on the file is
empty.

`tools/round.sh` says this is **not** a fifth round. The change→gate map puts `tools/conformance`
and a documents-only change on the core plus `cargo test -p conformance`, so that is what ran:
`fmt`, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`, `nextest --workspace`,
the doctests, the fuzz `check`, and the conformance gate last. §5 was not owed and no measurement
was taken against an installed binary. `PDFREF_CACHE` was not used: nothing in this round renders a
page or invokes a reference renderer.

Fourteen sweeps run before the edits and after them, with `spec-errata applied` beside them. The
new sweep is not in the before-run, because it did not exist. **`entries`, `unread`, `blockers`,
`callers`, `overstated`, `ledger`, `owed`, `capabilities` and `inapplicable` are byte-identical**,
and `overstated` in particular — the sweep whose mirror this round declined — did not move at all.
**Not one defect bucket moved:**

- `tables`, `pointers` and `counts` differ **only in line numbers inside
  `doc/todo/01-ledger-partial-rows.md`**, which this round's insertion moved by sixty. `tables` is
  byte-identical with those lines removed.
- `pointers` 8589 ← 8573 path pointers with live 4944 ← 4932 and unrooted 2811 ← 2807 — the new
  module, binary, ADR and this file, every one of them a live path. **Absent unchanged at 134,
  symbol pointers unchanged at 147 with 13 undefined.**
- `counts` 8393 ← 8377 sentences with 435 ← 432 attributed counts and 151 ← 149 the family agrees
  with. **58 "no such way" and 4 places counting one family twice, both unchanged.**
- `quotations` 6457 ← 6442 over 1009 ← 1007 documents, with **verbatim unchanged at 2729 and
  diverging unchanged at 38** — this round's prose added no claim about the standard.
- `overtaken` 591 ← 590 decision records, **45 overtaken unchanged in all three rungs**.
- `spec-errata applied` 57741 ← 57600 places, **0 naming an erratum and 1272 dropped tokens, both
  unchanged**.

`quoted` and `unpriced` were not run: this round touches no page-list note and both take the
oracle's log as their right-hand side. `retired` was not run either — this round retires no
mechanism, only two counts, and the grep for other statements of them found them all in `doc/adr/`
and in `doc/history.md`, which are dated and are not another round's to rewrite.

**A measurement error caught before it was believed, and it is trap 10a's shape one directory
over.** The first after-run was written into a scratch directory a previous session had used, and
the loop's `[ -s file ] ||` guard therefore compared four fresh sweeps against *another session's*
output — with a diff that looked exactly like a finding. The tell was a file timestamp sixteen
hours old. Redone in a directory created for the purpose, which is the same rule trap 16 states for
a build directory.

**This sweep fires on prose written in this round**, as the ninth does: ADR 0698's own paragraphs
count backends and hosts, and they are on rung 3 where they are counted rather than listed.

## The cardinal this sweep cannot see, corrected by hand

`doc/todo/01`'s header said "**Eighteen sweeps**" and "thirteen of them are committed programs"
while the catalogue below it numbered twenty-one. Both are cardinals about *this project's own
parts* and both had decayed exactly the way "both backends" did — and `--bin parts` is by
construction unable to print either, because no file states how many sweeps there are. That is the
boundary of the instrument, stated where it can be read: the sweep answers a cardinal only where the
workspace answers the noun.
