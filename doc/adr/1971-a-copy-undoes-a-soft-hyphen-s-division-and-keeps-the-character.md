# 1971 — A copy undoes a soft hyphen's division and keeps the character

Session 1567, slot 4 of batch ninety, a clause round. Status: **accepted, built**. Context: ISO
32000-2 §14.8.2.3, §14.8.2.5, §14.8.2.6.2; the Unicode Standard's `UnicodeData.txt` and UAX #14;
ADRs 1100, 1948.

## The question

ADR 1100 made search skip U+00AD and the whitespace after it, so a word an incidental line division
split is found by its spelling, and left the copy as §9.10.2 produced it. A paste of
`transparen\u{ad}\ncy` therefore broke the word where the page's line did, and the word pasted into
the find bar found nothing, because the needle's soft hyphen was compared and the page's line break
then stood in its way.

## What the standard says, and does not

§14.8.2.3 says what the character marks: "the visible hyphen that is introduced through the
incidental division of a word at the end of a line but which would not be present otherwise, may be
represented as a soft hyphen , mapped to the Unicode value U+00AD". It states nothing about what a
processor does with it, and §14.8.2.2.1 NOTE 3 says that silence is deliberate. So this is a choice.
The Unicode Standard says what the character means wherever it is pasted. `UnicodeData.txt` makes it
`Cf`, a format character, and UAX #14 makes it `BA`, a break opportunity, displayed as a hyphen only
where a line does break there.

## The choice

**The break after a soft hyphen is the division, and it goes. The character is the producer's, and it
stays.** `select::rejoined` takes out the whitespace after each U+00AD in both of a copy's orders. In
the logical copy it does so inside a piece and across two pieces the structure does not divide; a
block's or a cell's end stands, because the structure stated it. The pasted word reads whole, and a
program that reflows it can divide it again at the place the producer marked. `find` also skips the
page's break after a soft hyphen the needle states, so a word pasted from a copy finds the page it came
from. NOTE 1's hard hyphen U+002D is untouched, and `Interpretation::text` is unchanged.

**Rejected: folding the character out of the copy.** That would make the paste equal to the search's
comparison. It would also drop a character §9.10.2 produced and whose meaning is Unicode's, and with
it the hyphenation point.
