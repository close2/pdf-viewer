# ADR 0184 — A character with no code, given one

Status: accepted, 2026-08-04 (session 284).

## Context

`bug1865341.pdf` is a free text annotation whose value is **Załącznik** and whose `/DA` says
`/Helv 10 Tf`. Its `/DR` defines no `/Helv`, which is the document breaking §12.7.4.3's own
`shall`; since the two-hundred-and-fifty-eighth session that name is answered from the binary
anyway, because the fourteen four-letter abbreviations are a bijection with §9.6.2.2's list.

And the value still did not draw. The report said the `/DR` did not define the font, which had
stopped being the reason, and this project's own todo file said the stand-in "has no glyph for
their characters", which was never the reason:

- the missing set is **one character**, `ą`;
- `ł` was never missing, because Adobe's `StandardEncoding` happens to include `lslash`;
- every Helvetica has an `aogonek`, the compiled-in Liberation Sans included — glyph 197, named
  in its `post` table.

**What was missing is a code.** A simple font reaches a glyph only through §9.6.5's encoding, and
neither `StandardEncoding` nor `WinAnsiEncoding` has an ogonek. The two-hundred-and-eighty-third
session corrected the report to say both halves; this one closes it.

## Decision

**A font this module invents may state its own encoding, and it names the glyphs it needs.**

§9.6.5.1:

> the value of the `Differences` entry [is] an array of character codes and glyph names

so where the stand-in cannot encode a character, the dictionary gets

```text
/Encoding << /Type /Encoding /Differences [1 /aogonek] >>
```

and the value is drawn. The glyph name comes from the **Adobe Glyph List**, which is already in
this tree: `read-fonts` carries the generated table and `LoadedFont::text_from_program` reads it
in the other direction for §9.10.2's third method. Nothing is vendored, and the GPL trap the
handover's §1 records — poppler's `nameToUnicode` — is not approached.

Four constraints, each of them a line of code:

- **Only for a font this module invented.** A `/DR` font's encoding is what the document says its
  field is set in; rewriting it would answer a different question from the one the file asked.
- **Only if it is an improvement.** The rebuilt font is loaded and the value re-encoded, and the
  result is kept only when strictly fewer characters are missing. `freetext_no_appearance.pdf`'s
  Arabic gets `afii` names from the AGL, no Helvetica has those glyphs, and that page's refusal is
  therefore exactly what it was rather than a different refusal.
- **Codes at the bottom of the range**, 1 upwards: §9.6.5.2's two Latin encodings both begin at
  32, and 0 is `.notdef` by every font format's convention.
- **All or nothing.** More than 31 distinct characters, or one the AGL cannot name, and the array
  is not built at all — a partial one would draw some of the value and leave the rest absent
  without saying so.

## What it changed

| | |
|---|---|
| corpus incomplete | 75 → **74** |
| oracle agrees, complete pages | 855 → **856** |
| text gate | 22 931 of 23 349, still 98.2%, and one document held by name |

**And the text ratchet fired on the improvement**, which is this gate's second time. `pdftotext`
reads `bug1865341.pdf` back as **`Zacznik`**, and poppler *draws* it that way — both diacritics
silently dropped:

```text
ours     Załącznik
poppler  Zacznik
```

So the document scores 0 of 1 words against a reference that is wrong, and it is held in
`TEXT_BELOW_FLOOR` with that argument — the only entry on that list whose readback is better than
`pdftotext`'s rather than worse. Principle 5 in its plainest form: agreement would have been
evidence, and disagreement sends the question to the clause, which answers it.

## The lesson

**"No glyph" and "no code" are different sentences, and one of them was in three places.** The
report said one thing, the todo file said another, and both were about the font rather than about
the encoding — so the fix looked like font substitution (`doc/todo/21`, per-character fallback,
no witness, an open design question) when it was one dictionary entry the standard defines for
exactly this. What separated them was counting: *which* characters are missing, printed rather
than assumed, and the answer was one.
