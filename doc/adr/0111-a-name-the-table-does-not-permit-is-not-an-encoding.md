# ADR 0111 — A name the table does not permit is not an encoding

Status: accepted, 2026-08-01.

## Context

Two sessions of reading `reported` rows one at a time had emptied that population of findings.
The 242 `partial` rows are the population with no gate at all, and roughly 190 of them have
never been read against the code — too many to read by eye in one session.

So this session built an instrument instead. Every `partial` note was searched for a sentence
claiming an entry is *unread* — "unread", "not read", "nothing reads", "ignored", "not
implemented" — and every `/Name` in such a sentence was grepped for in the tree. A note that
says `/Foo` is unread, in a tree that reads `"Foo"`, is a finding by construction.

Twenty-five sentences matched, most of them naming keys that belong to some other clause and
are read there. Three were real.

## The three stale notes

- **§9.6** said it was `partial` "only because the bare Type 1 format of `/FontFile` is not read
  (reported, and no corpus page one reaches one)". `type1.rs` has read it since the thirty-first
  session, and that session's own finding was that the "no corpus page one" half was wrong too:
  **57 documents embed one.** Both halves of one sentence, wrong for ninety sessions.
- **§9.7.5.1** said "§9.7.4.3's `/W2` is unread" and that vertical writing "is refused rather
  than drawn horizontally". `Vertical::read` has read `/W2` and `/DW2` since the thirty-sixth
  session, and the row's *own named test* is `a_vertical_cmap_takes_the_second_set_of_metrics`.
- **§11.6.5.2** said "`/Matte` is not read". `image.rs` undoes §11.6.5.2's pre-blending where the
  arithmetic is exact and reports where it is not — one of trap 5's five deliberate
  report-and-draw cases, and named as such in the handover.

The middle one had spread. The same retired claim was in `composite_cmap`'s own doc comment
("`/W2` and `/DW2`, which nothing here reads"), and in two places in `doc/HANDOVER.md` — the
composite-font paragraph's "what is left is … vertical writing (§9.2.4's `/W2`)" and the corpus
font row's "the 4 asking for vertical writing". **A retired claim in four places**, which is
exactly ADR 0101's shape and the reason that ADR exists.

Recounting from the gate's own output settles it: of the 42 documents on the font row, **13
report a predefined `CMap` and exactly one of those names a vertical one** (`90ms-RKSJ-V`).
Nothing in the corpus is refused for its writing mode; they are refused for the registered data
§9.7.5.2's Table 116 names, horizontal and vertical alike.

## The defect the sweep led to

The font row's census turned up a report nobody had read: `bug859204.pdf`, "font /F1 uses
unsupported encoding **NULL**". The file writes `/Encoding /NULL` on a simple font with an
embedded Type 1 program, and the whole font was refused for it — so a page whose entire content
is one line of text drew nothing at all.

Table 112 settles it in the cell that states the entry:

> ( Optional ) A specification of the font's character encoding **if different from its built-in
> encoding**. The value of Encoding shall be either the name of a predefined encoding
> ( MacRomanEncoding, MacExpertEncoding , or WinAnsiEncoding …) or an encoding dictionary

The entry is optional, and the same sentence says what its absence means: the built-in encoding.
A name outside the permitted set is not a value this table admits, so the font has stated
nothing about its encoding and the built-in one stands. **Refusing the font instead is ADR
0106's error** — an optional entry erasing what the clause states — one clause family over from
where that ADR found it.

**`MacExpertEncoding` keeps its refusal, and the difference is the whole argument.** That name
*is* one Table 112 permits, so a font stating it means it, and drawing that font through some
other encoding would put the wrong glyphs on the page in silence. A name the table does not
permit carries no meaning to lose. The two questions — *may a font say this* and *does this
crate have the table* — are now two lists, and `MacExpertEncoding` is the name where they
differ.

`StandardEncoding` is accepted although Table 112 omits it: Annex D defines it, §9.6.5.1 makes
it the base a nonsymbolic font falls back to, and producers write it. A deliberate extra,
recorded as one.

## Consequences, measured

`bug859204.pdf` draws its line — "• Bug 859204", in the NewsGothicStd-Bold its file embeds — and
the page was read back before anything else was believed about it (trap 1). Corpus documents
drawing incompletely: **90 → 89**. The oracle's agreeing set: **840 → 841**, and the page agrees
with all three references rather than merely joining the comparison. Contradicted stays 65;
`pdftotext`'s share stays 97.9% with two more words matched out of two more found. Tests 889 →
890, the new one confirming both halves — the fallback *and* the refusal that is not one.

## The method is the durable part

A sweep that turns a class of prose claim into a grep is worth more than the three notes it
corrected, and this is the second of them: session 118 swept for reasons that had expired
("while §X does not exist"), this one for entries claimed unread. Both took twenty minutes and
both found live findings in a population nothing else watches.

**The obvious third is not available**, and saying so is the honest end of this ADR: a note
claiming an entry *is* read cannot be checked by grepping for its name, because the name being
present is what the check would look for. That class — a note whose "what IS done" half is
wrong in the other direction — still needs reading.
