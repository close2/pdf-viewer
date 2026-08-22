# ADR 0491 — A note that does not know what happened after it

Status: accepted, 2026-08-22. Session 665. Adds `tools/conformance/src/overtaken.rs` and
`--bin overtaken`, the nineteenth sweep and the fourteenth to be a program; corrects three
page-list notes in `crates/pdf-model/tests/oracle.rs`; amends §10.7.4's ledger row, trap 1,
`doc/todo/01`'s catalogue and `doc/todo/02` §4. **No pixel moves.**

## What was missing, and it was named

The six-hundred-and-sixty-second session's own last line: *nothing links a group's note to the
code it describes. Trap 1 states the habit and no gate has it.* The consequence it had just paid
for: ADR 0476 made this tree's edge coverage exact, and three sessions later
`CONTRADICTED_TIGHT_CONSENSUS` still said ours was the quarter-quantised form. The correction had
reached `doc/traps/`, §10.7.4's ledger row and `doc/todo/11` item 7 — everywhere except the group
it was about.

`oracle.rs` sorts every non-agreeing corpus page into a named list whose doc comment carries the
measurements, the clause and the argument. Those notes are the most valuable prose in the tree and
they are the only prose here with **no** instrument on them at all.

## Four candidates, and why this one

The round was handed four shapes to build. Three were measured before choosing, which is the
point of writing them down:

| candidate | measured | verdict |
|---|---|---|
| a note naming a symbol the tree lacks | `--bin pointers` **already reads `oracle.rs`** and reports nothing in it — 7043 path pointers, 97 symbol pointers, 13 undefined, none of them here | covered; the notes' pointers are live. What decayed was a *measurement*, and a measurement names no file |
| a note whose measurement disagrees with the gate | the gate's own vocabulary appears **12 times (`ssim`) and 34 (`worst tile`) in 7523 lines of note** | the premise does not hold. A group note rarely writes the gate's words beside its numbers, so there is nothing to anchor a number to and no way to say which page or which metric a bare `0.98591` belongs to |
| a note citing an ADR a later ADR supersedes | 31 of 489 ADRs say "supersede" at all, and **ADR 0476 records no supersession of 0474** | 662's case has no supersession record to find. Building one would mean writing the records first, which is the work, not the instrument |
| a note citing no clause | 662 computed it by hand and calls the criterion spent | a ranking, and it would not have named the planted *sentence*: the plant leaves the group's clause count untouched |

A fifth was tried and abandoned, and it is worth recording because it looked strongest: **the same
measurement stated twice with two values.** It finds the live defect below without a gate run —
`0.98591` and `0.98786` both stand in `oracle.rs` for `colors.pdf` page 1 — but it is *silent on
the plant*, because restoring the stale sentence makes the tree agree with itself. A prototype
over pairs of blocks sharing an anchor number produced 81 hits of which none was the case. An
instrument that goes quiet when the defect is restored is the one thing trap 13 forbids.

## The discriminator

`doc/adr/` is numbered in the order this project took decisions, so **a number is a date**, and it
is the only date the tree keeps in a form a program can read. A note's ADR citations are therefore
a claim about which decisions it has read. The sweep compares two numbers:

> the newest ADR the note cites, against the newest ADR that names one of the documents the note
> is about.

An ADR later than the note's newest citation, naming a page the note diagnoses, is *a decision
taken after the note was last revised, about a page the note explains*. That is a fact about two
files rather than a reading of either.

The population is every **page-list note** — the doc comment above a `const NAME: [&str; N]` of
corpus page names — which is 123 notes over 310 documents, in `oracle.rs`, `render-quorra`'s
corpus lists, `pdf-model`'s corpus lists and `text_extraction`'s floors. The vocabulary of
documents is derived from the lists themselves rather than from a `.pdf` pattern, so the standard's
own file name and a usage line's placeholder cannot enter from either side.

## The rungs, and the one the sweep taught itself

Three rungs, closest first: the later ADR **names the list itself**; it names a document the
note's **prose** argues; it names only a **list member** the prose never mentions.

**All three require a shared page, and that requirement is the first run's finding about the
sweep.** Without it, rung 1 held 24 of 123 notes and ADR 0489 was in every one of them — because a
*census* ADR prints the name of every list it counted. Naming a constant says an ADR mentioned the
list; only a shared page says it disturbed it. So the name ranks a hit and never makes one, and
the rung-1 population fell to 21.

## Calibration — trap 13, and the plant was run before anything was corrected

`git checkout fbe65e72^ -- crates/pdf-model/tests/oracle.rs` restores the tree to the revision
662 corrected, with the pre-ADR-0476 sentence back in `CONTRADICTED_TIGHT_CONSENSUS`. Against that
tree the sweep prints, as the first line of its first rung:

```text
the later ADR is about one of these pages and names this list:
    CONTRADICTED_TIGHT_CONSENSUS — crates/pdf-model/tests/oracle.rs:2510, newest ADR cited 0474
        doc/adr/0489-… [names this list] colors.pdf, issue7891_bc1.pdf
        doc/adr/0476-… [names this list] colors.pdf
```

For comparison, the sweep that would "obviously" find it — `--bin retired`, over the noun a reader
would type — returns **254 mentions of `quarter`, with the planted sentence at about rank 100**,
which is the difference between an instrument and a grep.

## What the first run found on the tree as it stands

48 of 123 notes overtaken. Three were acted on and all three are one page-family:

1. **`CONTRADICTED_ANTIALIASED_EDGES`, head of rung 2.** Its last paragraph still gave ours as
   ssim `0.98591` and `0.97906` against an exact form at `0.98772` and `0.98001` — the
   quarter-quantised raster's figures. ADR 0476 made ours the exact form, so the two rows stopped
   being two things. The gate prints `ssim 0.9879` against a bound of `0.9886` on page 1 and
   `0.9802` against `0.9840` on page 2.

   **The sharpest part is where it sat**: directly *above* it is the ADR 0476 correction, which
   ends "and the paragraph below is unaffected — which it predicted." A correction that scopes
   itself is a claim, and this one was false about the only sentence in scope. The instrument that
   would have caught it is the one that reads a note against what came after it, which is why this
   is the sweep's first finding rather than an anecdote about it.

2. **`CONTRADICTED_UNEXPLAINED`, second on rung 2.** Its `issue7891_bc1.pdf` paragraph ended "which
   is the next thing to measure here" — measured in 662 — and gave mean 0.22, one tile at 10.76 and
   0.52% differing. The gate prints **0.17, 6.73 and 0.54%**. Corrected off the run, pointed at
   `CONTRADICTED_TIGHT_CONSENSUS`, and given §10.7.4's clipping paragraph verbatim, which is what
   the answer turned out to be.

3. **`CONTRADICTED_TIGHT_CONSENSUS`, head of rung 1 on the *current* tree.** Not stale: 662
   rewrote it and did not cite its own ADR in it, so its newest citation is 0476 and ADR 0489 is
   later. One citation added. This is the sweep's commonest benign shape and it has a cheap cure
   that is also correct — **a round that rewrites a note cites its own ADR in it** — so the cure is
   written into `doc/todo/02` §4 rather than filtered out of the program.

The remaining 45 are the reading list this sweep exists to produce, headed by
`AMBIGUOUS_ICC_MATRIX_PROFILE` (cites ADR 0025; ADRs 0251 and 0488 name its two pages). They are
not resolved here and are not claimed to be: each needs the later ADR read against the note, which
is a round's work apiece.

## Why it is not a gate

ADR 0249's ratio argument, and one of its own: a note is allowed to be about one property of a page
while a later decision was about another, and a build that failed on that would teach rounds to
cite ADRs they had not read. It prints a reading list in a fraction of a second and exits zero.

## The clause

§10.7.4 is what both corrected notes cite, read verbatim in `doc/md/` and quoted under its number.
`tools/spec-errata emit doc/ISO_32000-2_sponsored_EC3.pdf` over the family prints one annotation
set for §10.7 — Issue #371, on §10.7.2's flatness — and nothing on §10.7.4's pages, so its
paragraphs stand as printed. Its ledger row was `partial` and stays `partial`; what it gains is the
second home of the claim it had already corrected once.
