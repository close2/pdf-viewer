# 0906 — The arm swept rather than corrected a fifth time

Session 933. Status: **accepted**. Every subtype ISO 32000-2 Table 171 defines, read against the
one sentence `appearance::construct` was giving all of them.

## Context

Four sessions have taken one member out of `crate::appearance::construct`'s catch-all, each after
reading the clause the catch-all was making a claim about:

| session | subtype | what the clause actually said | ADR |
|---|---|---|---|
| 622 | `Caret` (§12.5.6.11) | Table 183 states `/RD` — four numbers of geometry — and a paragraph symbol by code point | 0457 |
| 626 | `Redact` (§12.5.6.23) | Table 195 states `/QuadPoints`, and `/Rect` where that is absent | 0461 |
| 930 | `Screen` (§12.5.6.18) | the prose after Table 190 states the *absence* of an appearance | 0901 |

Three corrections of one construct is not three defects. It is one defect in how the arm was
written: six subtypes were given a shared sentence — *"its clause states no geometry"* — by a
round that had read none of their clauses, and every subsequent round found the sentence false of
whichever member it happened to open. The group's only common property was that nobody had checked
it, so *every* member was the one least like the rest.

**No ledger sweep can find this.** `doc/todo/01`'s sweeps each read a row's stated *reason* — a
blocker, a missing vocabulary, an absent architecture, a capability that arrived — and a catch-all
states none of those. It is invisible to all of them by construction, which is why the answer here
is a sweep of the *population* and a test that holds it, rather than a fifth correction.

## Decision

**Sweep Table 171 whole, give every member an arm, and hold the result with a test whose
population is the table.** Twenty-eight subtypes; `Popup`, `Projection` and `Screen` are answered
before this module is reached and a subtype outside Table 171 is answered by Table 167's
`Invisible` row (ADR 0628), so twenty-five arms and a catch-all whose population is one.

### The two the sentence was false of

**`Movie` (§12.5.6.17).** Table 189 makes `/Movie` required and calls it "[a] movie dictionary
that shall describe the movie's static characteristics"; §13.4's Table 306 gives that dictionary a
`/Poster`, and its stream form is

> it shall contain an image XObject (see 8.9, "Images") to be displayed as the poster

An image `XObject` is a stated shape this crate's caller draws on every page it opens. So the
clause chain does state something displayable, and what refuses it is principle 5's clause 13
exclusion — *a different finding*, because only an exclusion can be revisited by argument. The
entry's other form is the media question the exclusion was drawn for: "if it is the boolean value
true , the poster image shall be retrieved from the movie file". The §12.5.6.17 row asserted the
second of the two about both. `doc/questions/Q33` puts the stream form to the owner; nothing is
drawn meanwhile and the report names the exclusion.

**`Watermark` (§12.5.6.22).** Table 193's `/FixedPrint` and Table 194's `/Matrix`, `/H` and `/V`
are geometry — enough of it that "states no geometry" is false twice over — but it is geometry
about where an appearance *goes*:

> it shall be used in place of the annotation rectangle referred to in steps 2 and 3 of
> "Algorithm: appearance streams"

Steps 2 and 3 of §12.5.5 place a stored stream. So a watermark with no `/AP` has nothing to
derive, for the opposite of a silence: the clause is entirely about the annotation this path is
not being given.

### The two whose refusal got stronger

`PrinterMark` (§12.5.6.20 → §14.11.3) and `TrapNet` (§12.5.6.21 → §14.11.6.2) are the same shape
and it is the strongest form a refusal takes here. The clause does not leave the artwork unstated;
it says the appearance stream **is** the artwork — the visual presentation "shall be defined by a
form XObject specified as an appearance stream in the N (normal) entry" — and then requires the
entry: "The AP (appearances) and F (flags) entries (which is ordinarily optional) shall be
present". A file with none has broken a `shall`, which is what the report now says.

`3D` (§13.6.2) and `RichMedia` (§13.7.2) are that shape one exclusion over: "a 3D annotation shall
provide an appearance stream in its AP entry", and the inactive state "displays the annotation's
normal appearance". The stream is not one of two ways to draw these — it is the only thing a
processor that does not render the artwork can show.

### And the catch-all's own sentence

What is left is an annotation stating **no `/Subtype` at all** — Table 166 makes the entry
required, and `issue7446.pdf` is the corpus witness. It has no subtype clause, so the sentence a
person reads is Table 166's requirement rather than a claim about a clause. That is the finding
worth carrying: after the sweep, the one case the catch-all still fires on is the case its old
sentence was most obviously false of.

### `/FixedPrint` is reported rather than silently skipped

Reading §12.5.6.22 for the arm turned up a live departure that its ledger row explained away.
The row said `/FixedPrint` "waits on printing, which this program does not do". The clause
introduces the entry's effect with a `shall` on **rendering** — "When rendering a watermark
annotation with a FixedPrint entry, the following behaviour shall occur" — and forecloses the
printing excuse twice: "interactive PDF processors shall use the dimensions of the media box" when
one is displayed on-screen, and Table 194's own row says drawing "shall be done relative to the
dimensions specified by the page's MediaBox entry" where the target media are unknown. The media
dimensions a screen needs are stated by the standard.

`annotation::fixed_print_owed` names it. It is **reported rather than applied** because two of the
transformation's three terms are stated and the third is not: `/Matrix` and the `/H`/`/V`
percentages are Table 194's, and the cancellation of "a matrix B that maps a scaled and rotated
page into the default user space" is stated against a media origin whose relationship to this
tree's page space is a derivation nobody here has made. Drawing the mark in the wrong place is
worse than naming the entry (trap 5), and a departure named is a departure a round can take.

## Consequences

- `crates/pdf-model/tests/annotations.rs::no_table_171_subtype_is_refused_with_the_catch_all_s_sentence`
  walks Table 171's own list and fails on any member told the catch-all's sentence. Calibrated by
  planting each new arm in turn, under which it fails naming that subtype and reproduces the false
  report verbatim: `Movie: no appearance stream, and its clause states no geometry`.
- `a_watermarks_fixed_print_is_reported_and_a_plain_one_is_not` holds both halves; calibrated by
  making `fixed_print_owed` answer `None`, under which the first assertion fails and the second
  passes.
- Five ledger rows across two families cited
  `annotations.rs::an_unknown_subtype_still_draws_its_normal_appearance` — §12.5.6.17, §12.5.6.20,
  §12.5.6.21, §12.5.6.22 and §14.11.6.2 — a test that builds a `/Subtype /SomethingFromThePDF3Era`
  and asserts Table 167's `Invisible` row. Not one of the five is about that. `--bin pointers`
  cannot see it: the citation resolves, it just asserts something else. This is the third session
  in a row to find that shape (sessions 928, 930), and the count is now nine.
- §14.11.3's cited test was `the_hidden_and_no_view_flags_draw_nothing_and_report_nothing`, which
  states no printer's mark. Sixth in the same sweep.

## And the module header above the arm had the same disease

Sweeping the arm meant reading the module comment that describes it, and one of its bullets listed
`FileAttachment`, `Sound` and `Stamp` as the subtypes "refused and reported" for an icon whose
artwork no clause states. `symbol_icon` has drawn the first two since `a1f9d43a` — §12.5.6.15's
`Graph`, `PushPin`, `Paperclip` and `Tag` and §12.5.6.16's `Speaker` and `Mic` name **objects**,
which is more of a description than §12.5.6.4 gives its seven mandatory icons — and the bullet has
been false about two of its three members ever since.

`doc/habits.md` already carries this as *a comment that names a refusal outlives the refusal*, and
the instance it carries is **this same module header**, eighty sessions stale about §12.5.6.10's
four text markups (ADR 0105). A header is where a reader learns what a module refuses, and this
one has now been wrong about its refusals twice. It is corrected, and the correction states the
retired claim in words the fourth sweep's grep can still match.

## The rule this leaves

**When a round corrects one member of a group that shares a sentence, it sweeps the group.** The
population is the standard's own list, never a list in the comment, and the sweep leaves a test
whose members are that list — because the next round to open the seventh clause will not know that
five rounds before it each found the sentence false.
