# 1049 — Table 257 ranks what an update *wrote*, not what the object *is*

Session 1032. Status: **accepted**. Completes ADR 1043 §3's named refusal: `pdf_signature::revision`
now ranks each changed object against §12.8.2.2's Table 257 level. Adds `Kind`, `Verdict`, `Ranked`
and `Ranking`; `Judgement` gains `WithinWhatIsPermitted` and `NotPermitted` and keeps
`NotClassified` for what is still refused. Moves six §12.8.2 ledger rows' notes.

`§N` is ISO 32000-2 and nothing else.

## 1. The answer a level needs, and why type is not it

Table 257 states three values of `/P` and what each permits. Level 1 needs no analysis — "1 No
changes to the document shall be permitted; any change to the document shall invalidate the
signature." — and levels 2 and 3 need one, because they permit *operations*: "filling in forms,
instantiating page templates, and signing" for 2, those "as well as annotation creation, deletion,
and modification" for 3.

**An object's type does not name an operation.** A widget is an annotation (Table 166) and a field
(Table 226) at once, and level 2 permits filling it in while forbidding annotating it. So the
classifier reads *which entries the update changed*: `/V` with `/AP`, `/AS` and `/M` are what
filling in writes, and a widget that moved no further than those is a field filled in, while one
whose `/Rect` also moved is an annotation modified.

`prefilled_f1040.pdf` is where this is a real file rather than an argument. Its three updates
rewrite eight widgets: two were filled in, six also nudged their rectangles by a thousandth of a
unit, four appearance streams came with them, and three objects are the updates' own
cross-reference streams. So the file is **inside level 3 and outside level 2** — and a reader
ranking by type would have got both answers wrong, in opposite directions.

§7.3.3 decides the last part of that: "Wherever a real number is expected, an integer may be used
instead." One of those eight rectangles differs only in which form the file wrote it, so `396` and
`396.0` are compared as one number and the object is `Kind::RestatedUnchanged` — a change to the
file and not to the document, which even level 1 permits.

## 2. Two things the standard settles that convenience would have got wrong

- **§7.5.8's cross-reference stream is the update's own table.** §7.5.6 gives every update a
  section, §7.5.8 lets that section be an object, and Table 257 disregards an update carrying only
  DSS data — which it could not if its own table counted. So a level that permits any change
  permits the table recording it.
- **The DSS carve-out is asked of a whole update, and "only" is the whole of the test.** An update
  holding validation material and nothing else is `Disposition::Disregarded`; one holding it
  *beside* other objects is `Mixed`, and every object of it is `Unrankable`. Not forbidden —
  refused. §12.8.5.2's document timestamp is why: it "shall be determined by examining signature
  fields", so a real one touches the interactive form too, and this reader does not follow it that
  far. Reading "only" as "at least" would hide a page rewritten under cover of a DSS; ranking a
  genuine timestamp update as forbidden would be a false alarm. Refusal is the answer that is
  neither.

## 3. What is still refused, by name

The rule the module is written to is unchanged: a `/DocMDP` answer of *permitted* that was never
arrived at is worse than a refusal. `Judgement` therefore has three answers where something changed and no fourth, and
**none of them says a signature is valid** — step one is the digest and §12.8.1's third question
has no trust store (ADR 1039).

- **"Instantiating page templates" has no classifier.** §12.7.6's operation is a change to the page
  tree this reader does not tell apart from any other, so a document that instantiated one comes
  back `Unclassified` and therefore `Unrankable` at levels 2 and 3.
- **A replaced catalog is refused at 2 and 3** and forbidden at 1, for the same reason: Table 257
  ranks operations, and this reader does not resolve a whole catalog to one.
- **A `/P` outside 1 to 3** is `Unrankable`; guessing which way an unreadable statement leans is the
  lenient default the module exists to refuse.
- **Nothing in the program reports any of it.** The debt ADR 1043 named is unchanged: the ranking is
  reached by `pdf-signature`'s tests and by no host.
- **No transform parameter selects an object** (§12.8.2.1's `shall`). The ranking is over every
  changed object, so a `FieldMDP` whose `/Fields` names one field is still answered over all of them.
