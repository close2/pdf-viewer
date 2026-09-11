# Q59 — May a file that departs from the standard still claim the target?

Source: `doc/rfc/0007` §4.7.2, raised by session 954 on the owner's request for departures.
Status: **open** — answered when `A59-may-a-departed-file-still-claim-the-target.md` exists beside
this file.

## Why it needs the owner

The owner asked for an option to depart from the standard by name — *accept xml (and only xml)
attachments when targeting pdf/a 2*. RFC 0007 §4.7 proposes it: named per requirement, narrowed by
a predicate, reasoned, reported, and recorded in the file's own history.

One thing in that design is not the round's to decide. **Does the output still write the PDF/A
identification schema?**

- **Omit it**, which the RFC proposes as the default. The file then meets PDF/A-2 in every respect
  but the ones the configuration named, and says nothing false about itself. A validator reports it
  as an ordinary PDF rather than as a failed PDF/A-2.
- **Write it anyway.** The file then states `pdfaid:part 2` while carrying something part 2
  forbids, which is an assertion that is not true. A validator fails it — and the operator may not
  care, because their archive's intake may check the identification rather than run a validator.

The second is what an operator asking for this feature most likely wants, and it is also this
converter producing a file that lies about itself. Everything else in this program is built the
other way: no file is written "wearing a claim it has not earned", and a verdict says what it did
not check rather than implying coverage it lacks.

## What the tree does meanwhile

No departure exists; a document that cannot be made to conform is refused by name and nothing is
written.

## Recommendation

Omit the identification by default, and allow an operator to demand it with a second, separate
switch that is not implied by the first — so that "depart from a clause" and "claim conformance
anyway" are two decisions, taken twice. A downstream validator fails the file either way; the only
difference is whether the file lied before it failed, and that difference is the operator's to own
explicitly rather than to acquire as a side effect.
