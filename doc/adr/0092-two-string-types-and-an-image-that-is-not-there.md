# ADR 0092 — Two string types and an image that is not there

Status: accepted, 2026-08-01.

## Context

With the ledger's silence at zero, the specification track is its 53 `reported` rows. Three of
them are small, and two of the three are in clause 7 and clause 8 rather than in the interactive
half where the last several sessions have been:

- **§7.9.4, dates.** The row said "[n]othing here parses one, and nothing here needs to". The
  corpus disagrees: **1542 date strings** across the 974 documents, on annotations, signatures and
  embedded files, and `viewer-ui` already prints a signature's signer and reason and could not
  print *when*.
- **§7.9.3, text streams.** The row named the wrong reason.
- **§8.9.5.4, alternate images.** A clause-8 algorithm, stated as four numbered steps, that
  decides whether a picture appears on a page.

## §7.9.4: a grammar made entirely of defaults

The parse is small and two properties of it are not obvious.

**A conforming date may be four characters of payload.** "[T]he year field (YYYY) shall be present
and all other fields may be present but only if all of their preceding fields are also present.
The default values for MM and DD shall be both 01; all other numerical fields shall default to
zero values." So `D:1998` *is* a date and means 1998-01-01T00:00:00, and a parser that demands
fourteen digits rejects valid files.

**The numeric part is one run of digits and the zone starts wherever it stops.** The clause's own
example is `D:199812231952-08'00`, which has no seconds — so a reader indexing two bytes at offset
12 for the seconds field reads the sign of the offset. The first draft of this parser did exactly
that and failed on the clause's own example, which is the cheapest possible demonstration of why
a clause that states an answer is the test to write.

**An absent zone is decided, not unknown.** "If no UT information is specified, the relationship
of the specified time to UT shall be considered to be GMT." `Date::offset` still records that the
producer claimed nothing, because a viewer showing a date should be able to say "as written" — but
`Date::instant`, which the ordering rests on, takes the clause's answer.

An ordering exists because §12.3.5.1's Table 156 collection sort may name a field whose `/Subtype`
is `D`, and sorting those as *strings* is wrong in exactly the case a zone exists for:
`D:20240101120000+05'00` sorts after `D:20240101090000-05'00` and is two hours earlier.

## The corpus is 1542 independent attempts at one grammar

`crates/pdf-model/tests/dates.rs` runs the parser over every `/M`, `/CreationDate` and `/ModDate`
in every object of every document — the same move §12.3.3's `/Count` audit made, and for the same
reason: **a clause that states an algorithm is a clause that can audit a corpus.**

**1511 of 1542 conform — 97.98%** — and every one of the 31 that do not breaks a rule the clause
states in as many words:

| kind | count |
|---|---|
| an offset minute outside `mm (00-59)` — `+00'112'`, `'144'`, `'208'`, `'224'`, `'240'` | 26 |
| an offset written `-0100`, with the minutes present and the apostrophe absent | 1 |
| no `D:` prefix at all | 2 |
| a month of `00`, and a thirteen-digit numeric part | 2 |

Nothing is clamped and nothing is guessed at. A non-conforming string stays available as the bytes
the file wrote, and `viewer-ui` prints it as "at `<string>` (not a §7.9.4 date)" — which is the
same shape as every other refusal in this tree, and the only honest one when 2% of real dates are
a producer's mistake a person can read perfectly well.

The gate ratchets both ends: the conforming count may only rise, and the non-conforming one may
only fall.

## §7.9.3: the row named the wrong reason

The note said the type's only uses are `/RV` and "metadata streams, which are clause 14 beyond
this tree". Metadata streams are XML (§14.3.2) and are not text streams at all. Measured against
the standard's own tables, **every entry ISO 32000-2 types `text string or text stream` is
excluded by principle 5**: `/RV` (Tables 228 and 249) and `/RC` (Tables 172 and 174) are XFA rich
text, and `/JS` and the FDF ECMAScript dictionary's four entries are scripts. Each is refused by
name where it is met, which is what makes the row `reported`. The row is unchanged in status and
its reason is now true.

## §8.9.5.4: an algorithm that contradicts itself, and what to do about that

Three of the clause's four steps reach a screen, and the interesting one is step c): where a base
image's `/OC` hides it, its `/Alternates` are examined in order and one is drawn in its place.
Step b) is drawing a visible base image, which is what already happens. Step a) —
`/DefaultForPrinting` "shall be ignored on all Alternates entries" when the base has an `/OC` — is
satisfied *by construction*, because that entry is never read at all: step d) addresses printing
and this device is a screen. That is the third time this project has answered a clause by asking
what it requires of this device rather than by implementing it.

**Step c) contradicts itself.** Its selection sentence:

> the first entry not containing an OC key, or containing an OC entry specifying that the
> alternate image should be visible, shall be selected

and four sentences later:

> If none of the alternate image dictionaries have an OC key, or none of the alternate image
> dictionaries with an OC entry specify that that alternate image is visible, then nothing shall
> be shown.

An alternate with **no** `/OC` is selectable by the first and unshowable by the second. The habit
for this is to ask which reading makes a file's own words mean nothing: under the second, the
phrase "the first entry not containing an OC key" could never lead to a mark, so the clause would
be naming a case it had already excluded. **The selection sentence is taken**, and the one place
the two readings differ — a selected alternate with no `/OC` of its own — is *reported*, so a
document relying on the other reading is named rather than silently drawn. That is the fifth
place in this tree where a report accompanies drawing rather than replacing it, and it carries the
same kind of argument the other four do: two true statements, and suppressing either loses
information.

The clause's "Further" sentence is read as being about the **image**, which is what makes it say
something the selection has not already said: Table 89's `/OC` is on the alternate image
*dictionary* and Table 87's is on the image `XObject` the `/Image` entry names, so a dictionary may
be selected and its image still hidden.

**0 of the 964 openable corpus documents carry an `/Alternates` entry**, measured. Five synthetic
fixtures in `tests/optional_content.rs` are the whole of the evidence, which is trap 8's ordinary
case.

## Consequences

`reported` falls 53 → 51; `implemented` rises to 359 and `partial` to 231. 833 tests. No gate
moved — 89 corpus documents incomplete, 65 contradicted pages, 97.8% text readback — which is
expected: no corpus document has an `/Alternates`, and a date reaches no pixel.

What the session added that a gate *can* see is the fourth corpus census this project keeps, and
the first one whose denominator is a grammar rather than a page.
