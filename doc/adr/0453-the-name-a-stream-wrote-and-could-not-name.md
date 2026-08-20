# ADR 0453 — The name a stream wrote and could not name

Status: accepted, 2026-08-20. Session 617. Closes the site ADR 0439 left owed with an argument, and
the write-side defect that argument was about. Amends the ledger rows for §7.3.5, §12.5.6.6 and
§12.7.4.3.

## What §7.3.5 states, in both directions

ADR 0438 and ADR 0439 quoted one sentence of this clause — the one about *reading*:

> Uniquely defined means that any two name objects that, after all escaping is expanded (see
> below), and the resulting sequences of bytes are not an exact binary match denote different
> objects.

The clause states the other direction at the same length, and this tree had read it once:

> When writing a name in a PDF file, a SOLIDUS (2Fh) (/) shall be used to introduce a name. The
> SOLIDUS is not part of the name but is a prefix indicating that what follows is a sequence of
> characters representing the name in the PDF file and shall follow these rules:
>
> - a) A NUMBER SIGN (23h) (#) in a name shall be written by using its 2-digit hexadecimal code
>   (23), preceded by the NUMBER SIGN.
> - b) Any character in a name that is a regular character (other than NUMBER SIGN) shall be
>   written as itself or by using its 2-digit hexadecimal code, preceded by the NUMBER SIGN.
> - c) Any character that is not a regular character shall be written using its 2-digit hexadecimal
>   code, preceded by the NUMBER SIGN only.

with two sentences after it that decide the cases rule b) leaves open:

> Whitespace used as part of a name shall always be coded using the 2-digit hexadecimal notation.

> Regular characters that are outside the range EXCLAMATION MARK(21h) (!) to TILDE (7Eh) (~) should
> be written using the hexadecimal notation.

**The two directions are not inverses in the strong sense, and the clause says so itself** — NOTE
1: "There is not a unique encoding of names into the PDF file because regular characters can be
coded in either of two ways." Reading is a function; writing is a choice among functions. So the
round trip that can be asserted is *write, then read* — which is the one that matters, because it
is what a second reader does to a file this program saved.

`spec-errata emit` over clauses 7 and 12 moves none of it. §7.3.5 has one annotation (Issue #731,
`Review/Accepted`) and it is a NOTE about comments in *hexadecimal strings*; §7.3.7 has one that
points the other way and confirms the reading (#438: "Due to 2-digit hexadecimal code escaping in
PDF names, there are different ways to write the same key"); §12.7.4.3's are six `byte` carets from
Issue #318 and one cross-reference correction, and §12.5.6.6's are `array` for `rectangle` and a
version number. Nothing an erratum has moved touches either half.

## The two defects, and why they were one decision

`pdf_model::variable_text` is the only place in this tree that **builds** a content stream, because
§12.7.4.3 requires it to: "the PDF processor shall construct an appearance stream dynamically at
rendering time". It also reads one — a `/DA` is content-stream syntax in a string. So it is the one
module that stands on both halves of §7.3.5, and it had both wrong.

**The read.** `DefaultAppearance::parse` took the `Tf`'s first operand out of the lexer — already
`#xx`-expanded, already the name's own bytes — and made it a `String` with `from_utf8_lossy`.
`resolve_font` then probed `/DR`'s `/Font` with that text, against a clause that says the match is
binary:

> The specified font value shall match a resource name in the Font entry of the default resource
> dictionary

So a `/DA` naming a font whose name carries a byte outside UTF-8 missed the definition the document
had supplied and got the stand-in and a `FontNotInResources` report; and two such names, folding to
the same replacement character, were one font. That is ADR 0439's shape exactly, and it is the site
that ADR listed as owed.

**The write, which is the more serious of the two.** The constructed stream said `/{name} {size}
Tf` with the name written raw — no escaping at all — and the same was true of every name replayed
out of the `/DA`'s own operators, which §12.7.4.3 requires to be carried through: "The default
appearance string ( DA ) contains any graphics state or text state operators needed to establish
the graphics state parameters". A `/DA` saying `/Times#20New#20Roman 10 Tf` produced a stream
saying `/Times New Roman 10 Tf`, which §7.2.3 tokenises as the name `/Times`, the keywords `New`
and `Roman`, the number `10` and `Tf` — a `Tf` naming a resource the document does not have, with
two unrecognised operators in front of it. Meanwhile the stand-in font dictionary this module
invents was registered in that stream's own `/Resources` under the *whole* name, so the operand and
the key it was supposed to match were two different names by the clause's own test.

**Fixing the read alone would have made it worse**, which is why 604 declined it and why this is
one decision. The two halves were consistent while both were folded: the module found the wrong
font and then named the wrong font, and the stream was at least self-referential. Correcting the
lookup while leaving the writer would have found the document's font and then named a different
one.

`CLAUDE.md` permits exactly one kind of writing — §7.5.6's incremental update — and this stream is
written into the file by it. What the write defect produces is therefore not a wrong picture in
this program but **a wrong file, which another reader opens**.

## The one place that knows the rule

`pdf_syntax::Name::escaped` is new. It is §7.3.5's writing direction, once:

- a byte goes out as itself exactly when it is a regular character by §7.2.3, lies inside `!`..`~`,
  and is not the number sign;
- everything else — the nine delimiters, the six white-space bytes, `#`, and every byte above
  126 — goes out as `#xx`.

That predicate is rules a), b) and c) with the two narrowing sentences applied, expressed through
`lexer::is_regular` — §7.2.3's own set — rather than through a hand-written list of delimiters. The
result is always ASCII, which is what lets the content stream this module builds stay a `String`
while the name inside it is bytes.

`write::write_name`, which used to spell the rule, now writes the SOLIDUS and calls it. So the four
places this tree writes a name — a dictionary key, a name object, a `Tf` operand, a `/DA` operand
replayed, and the `Do` in an annotation icon's appearance — all go through it, and the reading
direction stays where it always was, in the lexer. **One rule, two functions, and nothing else in
this tree spells either.** That is 604's precedent (`Document::get_key_by_name`) applied to the
other direction.

Inside `variable_text`, a `pdf_syntax::Name` is now carried from the `Tf` operand to the `/DR`
probe, to the invented font's `/BaseFont`, and to the appearance's `/Resources` key — the *same*
value, so the operand and the key cannot drift. Where a name has to become text it is
`Name::escaped` rather than a fold: the report that names the font, and the label `FontError` puts
in its message. §7.3.5 permits that use in as many words ("occasionally the need arises to treat a
name object as text") and the escaped form is the one that still distinguishes two names.

**The two comparisons that stayed text are named rather than left to be noticed.**
`pdf_font::standard::is_standard_name` and `STANDARD_ABBREVIATIONS` are asked through
`Name::as_str`, and a name that is not UTF-8 simply does not match — which is correct rather than
lossy, because all twenty-eight of those names are ASCII and no name outside UTF-8 can be one of
them. It is the *lookup* the clause binds, and that one is bytes.

## What else the same writer was getting wrong

The question ADR 0439 asks of every fix: one defect of this shape is rarely alone.

- **A second name, with the same hole.** `variable_text::write` replays the `/DA`'s permitted
  operators into the constructed stream, and three of the twenty-seven take a name: `gs`, `cs`/`CS`
  and `scn`/`SCN`. Each was written with `from_utf8_lossy` and no escaping. The resources those
  names are resolved in are the same `/DR` the font is, so the defect is identical and the fix is
  the same call. `a_da_operand_name_that_is_not_a_plain_name_is_replayed_as_itself` is the pin, and
  with the defect planted back it reports `Operator { operator: "Gs" }` — the token ended early and
  the rest of the name became an operator.
- **A third, one crate over.** `appearance.rs` writes `/{name} Do` for an annotation icon's form
  XObject with `name.as_str().unwrap_or("Icon")`. That name is this program's own (`Icon`, or
  `Icon<n>` where `/DR` already has one), so it is ASCII by construction and nothing is wrong
  today — but the `unwrap_or` is the shape: a name that failed to be text would have been *named*
  `/Icon` while a different name was inserted into `/Resources`. It is `escaped()` now and the
  fallback is gone.
- **The strings are already right.** `variable_text::show` writes the value as §7.3.4.2's literal
  string, escaping the two parentheses and the reverse solidus and octal-escaping every byte
  outside `0x20..=0x7e`. That is the whole of the clause's rule and it was already there.
- **The numbers are already right, and the reason is the lexer's rather than the writer's.** Every
  float these two writers format is derived from a `Token::Real` or a `Token::Integer`, and the
  lexer bounds both: `fixed_format_number` refuses a mantissa past fifteen decimal digits, and the
  salvage path maps a parse that is not finite to `Token::Integer(0)`. So no infinity and no NaN
  reaches a `{}`, which would print `inf` and produce a stream §7.3.3 has no spelling for.
  `write::real` guards the same case for objects and says so; this is why the content-stream
  writers need no second guard, and it is written down here because "it cannot happen" is a claim
  that decays.

## What it is worth, measured

`examples/variable_text_census` gained the count, and two things about it are deliberate.

**The census's own reading of the `/DA` was wrong in exactly the way the code was.** `font_of` split
the string on white space and stripped a leading solidus, so `/Lime#20Green 12 Tf` looked like three
tokens and the census would have missed the very construct it was being asked to count. It lexes now.

**The predicate is spelled independently of the fix.** `is_plainly_written` states the byte set in
the census rather than asking `Name::escaped`, because a measurement taken with the instrument
under test is not independent of it (trap 8, ADR 0215).

The example also walks directories now, because the crawl is 66 000 files and no shell expands that
onto one command line — and `xargs` would print one census per batch instead of one census.

```sh
cargo run --release -p pdf-model --example variable_text_census -- doc/pdf.js/test/pdfs doc/corpora
cargo run --release -p pdf-model --example variable_text_census -- corpus-cache
```

The finding, and it is **not** zero, which is where this differs from ADR 0439's five sites: the
corpus states no such name at all, and the crawl states five, over two documents, printed by the
census. Every one of them is a font name with **spaces** in it, so it is the *write* half of the
defect that has the witnesses and the *read* half that has none — not one of the five is outside
UTF-8.

**And no page on this disk drew differently, which is measured rather than assumed.** All five are
§12.5.6.6 free text annotations, and `examples/free_text_census` says all five carry an `/AP` `/N`
stream — so §12.5.5's stored appearance is what is drawn and §12.7.4.3's construction is never
reached for them. `examples/display_list_digest` over both documents is byte-identical with the
defect planted back and with it fixed. The construction those five reach is a **save or an edit**
rather than a render, which is exactly the population ADR 0334's instrument judges and exactly why
the write half is the serious one: the file that comes out is wrong for everybody.

## How it is pinned

Hand-built, all of it, for trap 8's reason — a corpus finds what documents contain, and what a
writer owes is not in any corpus.

`crates/pdf-syntax/tests/name_escaping.rs` is new: the clause's four byte-shapes written as the
rules say; Table 4's ten literal names read as Table 4 says (including `/A#42`, which is in the
reading direction only because NOTE 1 makes the writing direction a choice); the round trip over
**every byte a name may hold** — null excluded because the clause excludes it — asserting both that
what is read back is what was written and that what is written is ASCII; a name written as one
token, with its size and its operator following it; and three pairs differing in one byte, written
differently.

`crates/pdf-model/tests/names_are_bytes.rs` gains the sixth vocabulary, in three tests: the read
direction with a `/DA` and a `/DR` key that match outside UTF-8, the collision direction where they
differ by one such byte, and the write round trip — a field filled, saved by §7.5.6's update,
reopened from its own bytes, and the constructed appearance's `Tf` operand lexed back out and
compared with the `/DR` key, for a space, a number sign, a solidus and a byte outside UTF-8. That
last one asserts the `/Resources` key as well, because a writer that escaped one and not the other
would pass the first assertion and still name a resource that is not there.

**Each was planted back before it was believed** (trap 13). With the lossy read restored, both read
tests fail and the report reads `/A#EF#BF#BD` — the replacement character, escaped, which is the
defect saying its own name. With the raw `Tf` write restored, the round trip fails with
`Some([79, 100, 100])` against `Some([79, 100, 100, 32, 78, 97, 109, 101])`: `Odd` where `Odd Name`
was written. With the raw operand replay restored, the third test fails with an unrecognised
operator.

## What this does not close

`doc/todo/22`'s remaining entry is unchanged and is not this one: `freetext_no_appearance.pdf`'s
paragraph of Arabic, which needs ADR 0348's list whole. What this round takes off that file is the
§7.3.5 entry, which it carried with the full shape since 604.
