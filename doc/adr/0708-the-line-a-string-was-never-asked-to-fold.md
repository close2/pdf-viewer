# 0708 — The line a string was never asked to fold

Status: accepted.
Context: the successor selection rule's eighth use, the first time its two rankings tied at the
head, and the first end-of-line rule this lexer has been asked for.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step and
ADR 0691's writing rule:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree names
> nowhere. Rank once over the live rows and once over **every** row, take the head of the two, and
> prefer the settled row where they tie. Reassemble the issue from every clause `emit` files it
> under, and read the issue whole.

Of the 302 issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret, 115
were named nowhere in this tree at this round's base.

## The head, and why it was a clause-13 row

Over live rows: **§7.6.4.1 and §7.6.6, six annotations apiece** — the same pair the seventh use
named, unmoved because no round has taken either. Over every row: **the same two and §12.6.4.17 at
six, `out-of-scope`**. That is the first tie the two rankings have produced at the head, and step
4's last clause settles it in the settled row's favour.

The six annotations turned out to be **§12.6.4.16's**, filed a subclause late: `emit` attributes an
annotation by the outline section for its page, and page 541 opens §12.6.4.16 and reaches
§12.6.4.17 before it ends. It changed nothing — both rows are `out-of-scope` under
`clause-13-multimedia` and `script-behaviour` respectively, and the tie-break wanted a settled row
— but it is worth the sentence, because a round that reads the *heading* rather than the annotation
text would have read the wrong clause. The strike says which sentence it is over. The heading only
says which page it was on.

Read whole, the two are inside the exclusion from end to end. Issue #265 strikes `transition` from
Table 220's `/S` row, which described the action type it defines as somebody else's; Issue #282
widens the action to a RichMedia annotation and lets `/V` name a view in a `/RichMediaContent`
dictionary's `Views` array. `action.rs` refuses the keyword `GoTo3DView` by name whatever the target
annotation is, which neither amendment touches. **The row's claim is confirmed rather than moved**,
which is a legitimate outcome for this rule and the reason to record it: the population decays by
two and nobody reads §12.6.4.16 for this reason again.

**So the round carried on down the ranking**, and the next settled row is where the work was.

## §7.9.2.4, `implemented`, five annotations under two issues

**Issue #276 tells the row which syntax its bytes may arrive in.** It strikes §7.9.2.4's old
file-identifier paragraph and writes in its place that *unless otherwise stated in this document, a
byte string may be either a literal string (see 7.3.4.2, "Literal strings") or a hexadecimal string
(see 7.3.4.3, "Hexadecimal strings")*, with the identifier demoted to an EXAMPLE and a NOTE added
that a signature dictionary's `/Contents` can be required to be hexadecimal. Issue #161 corrects
§7.9.2.2.1's NOTE 4, which named `dieresis` where the UTF-8 marker's EFh is `idieresis`; a NOTE is
informative and nothing here decodes from one.

§7.9.2.4's whole test list was `hex_strings_ignore_junk_and_pad_an_odd_digit`. The erratum makes the
row's `implemented` a claim about **both** written forms, so the question went to §7.3.4.2.

## The defect: §7.3.4.2's end-of-line rule

> An end-of-line marker appearing within a literal string without a preceding REVERSE SOLIDUS shall
> be treated as a byte value of (0Ah), irrespective of whether the end-of-line marker was a CARRIAGE
> RETURN (0Dh), a LINE FEED (0Ah), or both.

`Lexer::read_literal_string` implemented all eight of Table 3's escapes, the octal form with
§7.3.4.2's own "high-order overflow shall be ignored", and the backslash line continuation. An
**unescaped** end-of-line marker fell through to the byte-for-byte arm: a bare CARRIAGE RETURN
became 0Dh, and a CARRIAGE RETURN with a LINE FEED behind it became two bytes where the clause
states one.

**Why the row could not find it.** §7.3.4.2 is `implemented` and its note *enumerates* what the
reading side takes — "all eight escape sequences, the line continuation, and the rule that an
unbalanced parenthesis is a lexical error". A list is only as good as its closure, and nothing
compared that list against the clause's own `shall`s. This is 765's shape one clause family over,
which is why the sweep it suggested is worth more than the instance: there Table 6, here Table 3
and the three paragraphs around it.

**Why it is not cosmetic.** The rule binds the bytes a string object *holds*, and one lexer serves
the file body and the content streams alike:

- §7.6's algorithms hash and compare `/O`, `/U`, `/OE`, `/UE` and `/Perms` **by length and by
  byte**. A revision-6 `/U` is 48 bytes; one written as a literal string with an unescaped CARRIAGE
  RETURN in it arrived 49 bytes long and would not authenticate.
- §14.4's `/ID` is compared for equality, and §7.5.6's incremental update writes the second half of
  it. A string that does not equal itself across two readings is the failure this makes possible.
- Inside `Tj` or `TJ` a byte is a **glyph code**. 0Dh and 0Ah select different glyphs, and there is
  no report in either direction — the wrong glyph is simply drawn.

The fix is one arm, and it consumes a LINE FEED that follows a CARRIAGE RETURN so that the pair is
one byte. A LINE FEED followed by a CARRIAGE RETURN is *two* markers and therefore two bytes, which
the test states rather than leaving to be inferred.

## Calibration

`an_unescaped_end_of_line_in_a_literal_string_is_one_line_feed` asks all four unescaped forms and
the two escaped controls that must not move — Table 3's `\r`, which keeps its own byte, and the
backslash-before-a-marker continuation, which contributes nothing. Trap 13, twice: each of the four
unescaped assertions fails against the code that preceded the fix, and the whole test fails against
a plant that consumes the pair correctly and writes 0Dh instead of 0Ah.

## What else the reading corrected

§12.6.4's note listed the nine refused action types and called one of them `/ECMAScript`. Table 201
has no such type: §12.6.4.17 says the keyword is `JavaScript` and says why — "For backwards
compatibility reasons the term JavaScript is retained in keywords." `action.rs` matched the keyword
and always did, so this was the ledger naming a key the standard does not state rather than the code
missing one. No sweep is placed to print it: `--bin tables` reads a `Table NNN` citation with a key
beside it, and this key was attributed to no table number at all.

## What this says about the rule

Four uses of the fourth step, four settled rows, four different ways for a row's evidence to be
weaker than its claim: a round trip that could not fail (ADR 0671), a sentence about a sibling row's
status (0681), a set with no closure check (0691), and now a row asserting two written forms with a
test of one. They share only the status, which is the fourth step's whole argument.

And a tie at the head is worth having decided in advance. The settled row won it and paid nothing;
either live row would have paid something, because a `partial` row's erratum names a debt by
construction. The tie-break is not buying a better round — it is buying that the only signal this
project has for a decayed settled claim is read at all. The round's practice is the one to keep:
read the head to a verdict, then carry on down the ranking until a row pays.
