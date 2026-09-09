# 0929 — The previous revision is in the file, and is not reachable

Session 940. Status: **accepted**.

## What this corrects

`metadata/file-identifier-changes-with-history` — ISO 19005-2 §6.6.5 and ISO 19005-4 §6.7.4 — is
`Check::Unchecked`, and its reason said the rule compares one revision of a file against the
previous one while this crate is handed a single document with no earlier state to compare it
against.

**The second half of that sentence is false, and a reason that is false is worse than a row that is
missing.** A file that has been incrementally updated (ISO 32000-2 §7.5.6) *contains* its previous
revisions: the update is appended, so the earlier cross-reference sections, the earlier trailers and
therefore the earlier `/ID` arrays are all still in the bytes. `pdf_syntax` walks that chain to open
the file at all — `xref::read` follows `/Prev` from the last `startxref` backwards — so the earlier
state has been *read* by the time this crate sees the document.

## What is actually in the way

Two things, and neither of them is the absence of the data.

1. **`XrefTable` merges the chain and keeps the newest trailer alone.** `XrefTable::trailer`
   returns one dictionary, which is the merge §7.5.6's restating rule produces. **This half closed
   during the same session**, from the other side: `pdf_syntax::xref::sections` now walks the
   `/Prev` chain a second time, off the opening path, and hands back a `SectionRecord` per section
   carrying that section's *own* trailer, newest first. So each revision's `/ID` array is now
   reachable, and the reason no longer rests on this.
2. **The condition needs the previous revision's *packet*, not only its trailer.** The rule fires
   when an `xmpMM:History` entry is *added*, which is a difference between two revisions' XMP —
   so answering it needs the earlier revision's `/Metadata` object resolved at the earlier
   section, which is a second `Document`.
And one thing that is not a way in: `Document::bytes` hands out the raw file, so nothing stops this
crate parsing the earlier revision itself. That is exactly what `pdf-archive` is designed not to do
— its stated shape is to add no reader of its own — and two cross-reference readers in one tree is a
correctness hazard with no upside. Where an earlier revision has to be read, `pdf_syntax` is where
the reading belongs.

## The decision

**The row stays `Check::Unchecked`, and its reason now says the true thing**: the earlier state is
in the file, `pdf_syntax` keeps the merged table and the newest trailer, and the row waits on that
crate exposing a revision rather than on the file carrying one.

**It is not implemented as a partial check either.** The decidable fragment a round is tempted by —
comparing `ID[0]` with `ID[1]`, on ISO 32000-2 §14.4's rule that both are set to the same value when
a file is first written — answers a different question from the clause's. It would say the file has
never been updated, not that a history entry was added without the changing identifier moving. A row
whose `asks` said one thing and whose predicate tested another is the failure `doc/questions/A20`
exists to prevent.

## What would close it

**One thing, now that `xref::sections` exists**: a way to resolve an object as of an earlier
section, so that the previous revision's catalog `/Metadata` can be read and its `xmpMM:History`
compared with the current one's. Without it the trailers can be compared and the clause's own
condition cannot be established, and a check that fired on the trailers alone would be testing
something the clause does not say — a revision that adds no history entry is under no obligation
from §6.6.5 at all.

**There is no corpus witness either way**: neither veraPDF profile carries a rule for §6.6.5 or
§6.7.4, and no corpus directory exercises them, so whoever implements it will be working from the
clause alone.

## The thing not to redo

Do not re-derive the reachability. It is written down here: the bytes have it, `xref::read` reads
it, `XrefTable` merges it away, and `xref::sections` gives the trailers back.
