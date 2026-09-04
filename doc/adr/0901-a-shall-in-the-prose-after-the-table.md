# 0901 — A `shall` in the prose after the table, and a screen annotation that reports a clause it has

Session 930. Status: **accepted**. What reading the rows ADR 0900's sweep flagged actually found,
and the one of them that reached code.

## Context

ADR 0897 recorded a shape and could not say how common it was: **a row that enumerates a table has
already chosen which half of its clause to read**, and the standard's reader-facing `shall` is as
often in the prose *after* the table as in it. §14.7.4.2 was `partial` for Table 356's `/Schema`
— a permission — while the clause's real requirement, that a namespace name identifies an
attribute object's owner, sat in the closing paragraph and had never been named.

Five rows were read this session. Three kept their status; two moved, and one of the two moved a
report off a page.

## Decision

### §12.5.6.18 screen annotations — `partial` → `implemented`, and a false report deleted

The row had been `partial` for one thing for the whole of its life: Table 190's `/MK` "reaches
this crate only through `/AP`". Table 190 states that entry as "(Optional) An appearance
characteristics dictionary … The I entry of this dictionary provides the icon used in generating
the appearance referred to by the screen annotation's AP entry" — Optional, and with no `shall` on
a reader anywhere in it. ADR 0896's shape, and the row's five-entry enumeration is ADR 0897's.

**The clause's two reader-facing `shall`s are both in the prose after the table**, and the row
named neither. The first was not met:

> If AP is not present, the screen annotation shall not have a default visual appearance and shall
> not be printed.

`annotation::construct` handed a `Screen` with no appearance dictionary to
`appearance::construct`, whose catch-all answered `Refusal::NotDerivable("its clause states no
geometry")` — **a claim about a clause that states the absence of one.** That is trap 11 exactly,
and it is the third time the same arm has been corrected the same way: §12.5.6.11's caret and
§12.5.6.23's redaction were each moved out of it by an earlier session for the identical reason.
The subtype now returns `Decision::Nothing` beside `Projection`, so the requirement is met by
drawing nothing and reporting nothing.

The second `shall` is vacated rather than executed: "The P entry shall be used for a screen
annotation referenced by a rendition action" binds a processor performing §12.6.4.14's action,
which is `out-of-scope` on principle 5's clause 13 exclusion and whose own row says so. That is
the line §12.11.1 keeps — a `shall` met by construction where the condition it is written for
cannot arise — rather than §12.11.5's, where the whole clause was vacated.

**The row's cited test asserted a different clause** — the third such citation in two sessions,
after the two ADR 0896 records. `an_unknown_subtype_still_draws_its_normal_appearance` builds a
`/Subtype /SomethingFromThePDF3Era` and asserts Table 167's `Invisible` row — §12.5.1's subject,
and not one word of §12.5.6.18. `a_screen_annotation_without_an_appearance_draws_nothing_and_is_not_a_gap`
states a `Screen` in both directions, and was calibrated by planting the arm it replaces: with it
gone the first assertion fails carrying the sentence above, and the second — that a stated `/AP`
is still drawn — passes, which is what makes the pair discriminate against dropping the subtype
outright.

### §12.7.7 named pages — `partial` → `implemented`, and the sweep could not have said so

The row was `partial` because the clause gives naming a page two uses and one of them is a script
adding it. Both uses are stated with *can*:

> - An import-data action can add the named page to the document into which FDF is being imported,
>   either as a page or as a button appearance.
> - A script executed by an ECMAScript action can add the named page to the current document as a
>   regular page.

Neither is a requirement on a reader. Every `shall` the clause states is about the **file** — a
visible named page "shall be left in the page tree", an invisible one "shall have an object type
of Template rather than Page and shall have no Parent or B entry" — and all four run as invariants
in `NamedPages::disagreements`. The first use is performed anyway (§12.7.8.3.3's import); the
second is a permission declined on principle 5's closed exclusion, which is a decision recorded
rather than a debt outstanding.

**ADR 0900's sweep does not flag this row**, and that is the honest limit of the instrument
written down where it was met: §12.7.7's note quotes the two `shall`s it *implements*, so the
sentence half calls it silent, and its debt is a *can* in prose that no table entry carries.
It was found by reading the band `doc/todo/01` named, not by the program.

### Three read and kept, each on a sentence

- **§8.11.1**, flagged rank 1 for Table 98's `/Configs`. The note names §8.11.4.5's "shall be
  reapplied" beside it, which is a requirement and is measured rather than met. The row is an
  aggregate and follows §8.11.4, which is `partial`. **The three-word quotation is below
  [`quote::MIN_WORDS`]**, which is why the sweep saw only the permission — a bound, not a bug.
- **§11.4.8**, flagged rank 1 with 0 `shall` sentences in its prose. The clause says of itself
  "This subclause is a restatement of the group compositing formulas", its only prose outside the
  formulas is a NOTE, and it states no requirement §11.4.4 and §11.4.6 do not. Its status is
  theirs, and both are still `partial`.
- **§11.7.4.4**, the pixel-changing row of the next band. The departure is live and reported — a
  filled-and-stroked path whose portions this renderer cannot state a shape for, or one of whose
  elements blends, is pushed flat and named — and moving it is §11.4.6's work rather than this
  row's, because §11.4.4's NOTE 3 is what lets a non-isolated knockout group be composited onto
  transparency and a blending element is where that cancellation stops holding. **Its
  read-and-kept sentence was wrong about its own evidence**: it named `knockout_is_drawable`, a
  function this tree has not had for hundreds of sessions, where the gate is
  `content::transparency::knockout_group_elements`, and `--bin pointers` cannot see it — that
  sweep resolves a path and a `file::symbol` fragment, and a bare identifier in a note is neither.

## Consequences

- **Two rows left the owing statuses and neither on the evidence a sweep produced.** One was
  flagged and read; one was not flagged and was read because a `doc/todo/01` band named it. The
  pair is the argument for taking work from the instrument *and* from the list, in the same round.
- **A catch-all refusal is a claim about every clause it catches**, and this arm has now made the
  same false claim three times. What each correction has in common is that the clause did speak
  and the arm had not been asked: the caret's artwork, the redaction's overlay, and now the screen
  annotation's absent appearance. A round adding a subtype to that arm owes its clause a reading.
- **A bare identifier in a ledger note is unchecked by anything.** The eighth sweep resolves
  pointers with a file in them; a function name written alone decays in silence. The cheapest
  repair is the one this round used — grep the name — and it is worth one command per row read.
