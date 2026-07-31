# ADR 0060 — The property list that was never a dictionary

Status: accepted, 2026-07-31.

## Context

§14.9.4's `/ActualText` is "a replacement, not a description, for the content" — a character
substitution a reader applies when *extracting* text, and the one row of §14.9 whose consequence
this project can already measure, because `text_extraction.rs` exists and is held to an
independent extractor.

Five corpus documents write one. `issue13226.pdf` is the clearest: it shows `Mit`, then a space
glyph wrapped in `/Span <</ActualText <FEFF00AD>>> BDC … EMC`, then `arbei`, then another such
space, then `terinnen`. The soft hyphen U+00AD is §14.8.2.3's "visible hyphen that is introduced
through the incidental division of a word", and the document is saying: *this space you see is a
hyphenation artefact; the word is `Mitarbeiterinnen`*. We read back `Mit arbei terinnen`.

## What stopped it

The obvious implementation — read the `BDC` operand's dictionary, look for `/ActualText` — does
nothing, and finding out why produced the more interesting half of this session.

**The content lexer yields tokens, not objects.** `token_to_object` maps `<<` and `>>` to
`Object::Null`, with a comment saying so: "Recognising the brackets is enough for the operators
this interpreter implements; a full re-parse would duplicate the object parser for no present
gain." An inline `/Span <</ActualText …>> BDC` therefore reached the operator dispatch as *five
loose operands*, and there was no dictionary to look in.

Optional content never noticed because §8.11.3.2's property list must be a **named** resource —
an optional content group is an indirect object, and §14.6.2 forbids indirect references inside a
content stream, so `/OC` always arrives by name. The one form that was implemented was the only
form that one caller could use.

**And the ledger's row for §14.6.2 said "content.rs takes both forms".** That is the third row in
four sessions found claiming behaviour the code did not have (ADR 0056, ADR 0057), and the third
written from the clause during a review. The handover carried the same claim in its own words —
"content.rs already parses that property list because §8.11.3.3's `/OC` arrives the same way" —
which is exactly the reasoning that makes this mistake: the two *do* arrive the same way, and
only one of the two ways was read.

## Decision

**Assemble an inline dictionary from the token stream, at the point operands accumulate.**
`inline_dictionary` reads name-value pairs until the matching `>>`, recursing for a nested
dictionary and skipping over an array's tokens, bounded in depth. Arrays are deliberately left
flattened: `TJ` and `d` have read their elements as separate operands since the beginning, and
grouping them would be a second change wearing this one's clothes.

`true`, `false` and `null` are handled inside it, because they lex as **keywords** — which is how
two corpus documents came to report `true` and `false` as *unknown operators*. §7.3.2 makes them
objects wherever an object belongs, and inside `<< … >>` an object belongs.

**`/ActualText` is applied at `EMC`, by truncation.** The section records where the page's text
stood when it opened; at the close the text is cut back to that point and the replacement
appended. Nesting falls out for free — an outer replacement discards whatever an inner one
produced — and so does the case of a sequence that draws nothing.

## What it was worth

- `issue13226.pdf` reads back its soft hyphens instead of spaces.
- **`MAX_INCOMPLETE` falls 97 → 96** without anything being drawn differently: `issue7821.pdf`
  draws completely and `issue17069.pdf` reports only its knockout, because their inline property
  lists' booleans are no longer operators.
- The oracle is unchanged at 821 agreeing and 76 contradicted, over one *more* judged page.
- The extraction gate still finds 100% of `pdftotext`'s words across the specification PDFs.

## Consequences

- §14.9.4 is `partial`: the marked-content half is implemented, the structure-element half needs
  §14.7's tree, and the rule about consecutive sequences and word breaks is named on the row.
- §14.6.2 is corrected and names a test that fails if the inline form stops being read.
- The door is now open for the other three entries §14.9 puts in a `Span` property list —
  `/Lang`, `/Alt` and `/E` — none of which is implemented and each of which now needs only its
  own semantics.
