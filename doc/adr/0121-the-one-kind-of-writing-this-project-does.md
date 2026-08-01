# ADR 0121 — The one kind of writing this project does

Status: accepted, 2026-08-01.

## What this decides

A document can be saved. `Command::Save` → `Event::Saved { bytes }`, and the bytes are the file
the document was opened from, unchanged, with ISO 32000-2 §7.5.6's incremental update appended:

> The contents of a PDF file can be updated incrementally without rewriting the entire file. When
> updating a PDF file incrementally, changes shall be appended to the end of the file, leaving its
> original contents intact.

That last clause is why this is the one form of writing `CLAUDE.md` permits. Nothing here decides
how a document should be laid out; it says "this object now reads like this", and everything the
producer wrote is still in the file underneath — including whatever was signed, archived or
notarised in it.

## Three pieces

**`pdf_syntax::write::object`** writes one object in clause 7's syntax. The two decisions in it
are about escaping: a name's non-regular characters become §7.3.5's `#xx`, and a string is
written in §7.3.4.3's hexadecimal form rather than §7.3.4.2's literal one — twice the length, and
none of the balanced-parenthesis and non-printable-byte rules. What this writes is a person's
name in a form field, not a page of content.

**`pdf_syntax::write::incremental_update`** writes the section. The new objects, a cross-reference
section covering exactly them, and a trailer whose `/Prev` chains to the offset the file's own
`startxref` names.

**`ViewState::save`** builds the replacements from the edit log: one object per field a value was
typed into, and the interactive form dictionary with `/NeedAppearances` set.

## The decisions, each with its cost

**The cross-reference section is the kind the file already uses.** Nothing in the standard
requires them to match — a PDF 1.5 reader handles both — but a file whose sections are all one
kind is a file whose next reader has one thing to do, and it cost forty lines. A document with
§7.5.4's table gets a table; one with §7.5.8's stream gets an uncompressed stream with Table 18's
three fields at `/W [1 4 2]`.

**The value goes where the document already keeps one.** §12.7.4.1 makes `/V` inheritable, so
writing it onto the widget a person clicked would leave the field's *other* widgets reading a
stale ancestor. `holder` climbs the `/Parent` chain and stops at the first dictionary that states
a `/V` — the one the inheritance is actually reading — or, where none does, at the one that
states a `/FT`, because Table 226 makes the type an entry of the field rather than of its
widgets. The first version of this stopped in the right place and returned the *widget's* object
number with the *field's* dictionary, which wrote a field into a widget's slot; both external
readers still opened the file, and neither showed the value.

**`/NeedAppearances` rather than regenerated appearance streams, and the cost is written down.**
A widget's stored stream still says what the field said before. A writer can fix that two ways:
regenerate every affected stream, or tell the next reader to. Table 224 exists for the second —
"a flag specifying whether to construct appearance streams and appearance dictionaries for all
widget annotations in the document" — and it is what this writes, because regenerating means
writing *this program's own reading* of §12.7.4.3 into somebody else's file. **The cost: a reader
that ignores the flag shows the value the field had before.** Every reader this project compares
against honours it, which is the next paragraph.

**The file identifier changes, and deterministically.** §14.4 requires the second element to be
"based on the file's contents at the time it was last updated" and the first not to change. The
second is a digest of the bytes being appended to — a function of those bytes alone, so saving
the same edit twice produces the same file. That is deliberate: rule 3 says this crate has no
clock, and a test that saves twice and compares is worth more than a timestamp.

**Four documents are refused by name**: one whose cross-reference table was rebuilt by scanning
(there is no offset worth chaining to), an encrypted one (§7.6.2 has every string encrypted with
the document's key, and plaintext in one would decrypt to nonsense), one with no `startxref`, and
one with no `/Root`. Encryption on the way out is a real gap and is named as one.

## The evidence

The tests write an update and then **read it back with this tree's own reader** — the same one
974 corpus documents go through. That fails if an offset is wrong, if the `/Prev` chain is broken,
if a subsection header does not match its entries, or if an object was written in a syntax the
lexer will not take. None of those is visible in a diff.

And then the thing principle 5 asks for. `form_two_pages.pdf` with `Text1` set to `Ada Lovelace`,
saved, and handed to two implementations that share no code with this one:

```
$ pdftotext saved.pdf -     →  Ada Lovelace
$ mutool draw -F txt saved.pdf 1  →  Ada Lovelace
```

Two readers, neither ours, both finding the value through the update's cross-reference section,
the `/Prev` chain, the field hierarchy and Table 224's flag. Agreement is evidence that §7.5.6 was
read correctly — never the definition of correct, and it is not needed for that here: the clause
is four sentences and the test that matters is the round trip.

## Consequences

Tests 925 → 932. Six are `pdf-syntax`'s — the round trip, the chain of two updates, the section
kind, §14.4's identifier, the refusals, and every object type through the writer and the reader —
and one is `viewer-core`'s, which saves from a keystroke and opens the result in a second viewer.

`viewer-ui` writes the bytes beside the document with `.edited.pdf` appended rather than over it:
overwriting somebody's file is a decision this program has not been given, and rule 2 puts the
choice in the host either way.

The four gates are unmoved. Nothing reads a file this writes except the tests.

## What is left

Encryption on the way out, which would close the second refusal. Regenerated appearance streams,
which would close `/NeedAppearances`'s cost. An annotation added or a markup drawn, which
`CLAUDE.md`'s amended exclusion permits and which is the same log and the same writer with a
different object in it.
