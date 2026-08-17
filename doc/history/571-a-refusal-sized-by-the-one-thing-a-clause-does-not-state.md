# 571 — a refusal sized by the one thing a clause does not state

**Finding:** §12.7.5.4's list box had been refused whole for the project's life because the clause
states no appearance for a *selection* — and the same clause states the list outright, in the same
paragraph, one sentence along. Reading past the sentence that justified the refusal turned "draws
nothing" into "draws the options and reports the mark".

Date: 2026-08-18.
Argued by: [ADR 0405](../adr/0405-a-refusal-sized-by-the-one-thing-a-clause-does-not-state.md).
Files touched: `crates/pdf-model/src/{variable_text,appearance,form}.rs`,
`crates/pdf-model/tests/variable_text.rs`, `crates/pdf-model/examples/variable_text_census.rs`,
`doc/conformance/ledger.toml` (§12.7.4.3, §12.7.5, §12.7.5.4), `doc/todo/{22,30,README}.md`,
the ADR and this file.

## Why this item, and what was rejected for it

`tools/state.sh quick` and `doc/todo/README.md` first. The demand-driven side had nothing loose that
was not another round's — 570 holds the corpus census and an earlier round holds `render-quorra` —
so the choice came off the spec-driven side, and out of `doc/todo/30`'s last open entry, which had
the shape `CLAUDE.md` warns about most loudly: a refusal whose reason is a *claim about the
specification*.

Rejected on the way: `doc/todo/33`'s `/BS` border colour (genuinely unstated, and a colour this tree
may not invent — the same argument that keeps the list box's highlight unbuilt), `doc/todo/31`'s
accessibility tails (large, and one is a measurement on a thousand-page document), and
`doc/todo/22`'s Arabic free text (already read, priced and pinned).

## What the clause turned out to say

The refusal was one line of code and one sentence of comment. Beside it, §12.7.5.4 says the `/Opt`
array's entries are "each of which shall be represented by a text string that shall be displayed on
the screen"; Table 233 bit 20 says "PDF readers shall display the options in the order in which they
occur in the Opt array"; Table 234's `/TI` names the first option visible; and §12.7.4.3's own NOTE
gives "scrollable list boxes whose contents are determined interactively at the time the document is
displayed" as its first example of what a processor "shall construct … dynamically at rendering
time".

ADR 0106's test — is the refused entry additive or substitutive — settles the rest: a selection mark
is drawn *over* an item that is drawn either way.

## The measurement that says where the debt was, and the one that said it was nowhere

Every list box in the pdf.js corpus states an `/AP`, and none of the eight documents holding one
sets `/NeedAppearances` — so the corpus gate reported no list box before this round and reports none
after it. `doc/todo/22` had recorded exactly that count in the four-hundred-and-third session, read
it as "this costs nothing", and told the next round to "revisit it with a document in front of you …
there is none in 974 files".

That instruction is why the item sat for a hundred and sixty-eight rounds, and it was looking for
the wrong document. The construction is only ever reached under a *regeneration* — `/NeedAppearances`
or a value a person changed — and ADR 0248 gave three hosts a multi-selection list control in the
four-hundred-and-twelfth. So from that round on, a person could choose an item, the page went on
showing the producer's stream for the previous value, and `save` wrote Table 224's flag where a
stream belonged. **A census of files cannot count a population defined by what a person did to one.**

## What the picture rejected

The first version gave the list box an auto-sizing rule of its own, on the reading that its rectangle
is a scrolling window rather than a bound, so the size to find fits one line. Rendering
`listbox_actions.pdf` with the size zeroed produced 34-point type for a 120-point-wide list of six
short labels, showing two and a half of them. The rule went; the window reading is answered one step
earlier, because `/TI` decides which options are laid out, so the run the shared auto-sizer measures
already is the visible one. Trap 1 applied to a rule.

## Two things to hand on

**A fixture that appends a second `/DA` states neither.** A test here spelled the widget's entries
into a `format!` that already carried `/DA (/Helv 12 Tf 0 g)`, so a case meant to exercise
auto-sizing was laid out at 12 points and passed while measuring nothing. `choice_field_sized` takes
the `/DA` as a parameter now. The tell was a break-the-code check that did not fail.

**A parallel round re-initialising submodules in the shared main checkout breaks every worktree that
symlinked to them**, transiently: for about a minute `doc/pdf.js` and `doc/arlington-pdf-model` in
`/home/cl/projects/pdf-viewer` were self-referential symlinks, and a build in this worktree failed
with "the Arlington PDF Model is missing". Nothing here was wrong and nothing needed fixing; the
minute is the cost. The six gitlinks in this branch's index were checked before and after the commit
and are unchanged.

## Also noted, not fixed

`cargo fmt --all --check` is red on `main` at `14a81f0d` — one line in
`crates/render-quorra/examples/viewport_refusal.rs`, from `a932e203`, untouched by this round and
inside another round's crate.
