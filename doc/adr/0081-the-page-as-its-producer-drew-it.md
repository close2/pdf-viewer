# ADR 0081 — The page as its producer drew it

Status: accepted, 2026-07-31.

## Context

§12.3.4's thumbnail images were `silent`, and the row said the missing piece was a panel rather
than a decoder because "the decoder it would need is `image.rs`, which exists". That was true of
the *bytes* and wrong about the clause: §12.3.4 does not reuse §8.9.5's meaning, it **subtracts**
from it.

> It has the usual structure for an image dictionary (8.9.5, "Image dictionaries"), but only the
> Width , Height , ColorSpace , BitsPerComponent , and Decode entries are significant; all of the
> other entries listed in "Table 87 -Additional entries specific to an image dictionary" shall be
> ignored if present.

A `/Thumb` carrying an `/SMask` is to be decoded as though it had none. Handing the stream
straight to `image::decode` would have applied it.

## Decision

`pdf-model/src/thumbnail.rs` copies the stream dictionary, removes the eighteen entries of Table
87 the clause does not name, and decodes what is left by the ordinary route.

**The line is between tables, not between keys.** `/Filter`, `/DecodeParms` and `/Length` are
Table 5's — "entries common to all stream dictionaries" — and the sentence names Table 87, so
they survive. The clause's own EXAMPLE settles it by writing `/Filter [/ASCII85Decode
/DCTDecode]` on a thumbnail: a reader that kept only the five named entries could not decode the
clause's own example. Reading the sentence as a whitelist is the failure this module is shaped to
avoid, and both directions are guarded by tests that were confirmed to fail when the drop is
removed.

The two producer-side constraints — a `/Subtype` that is not `Image`, a colour space that is not
`DeviceGray`, `DeviceRGB` or `Indexed` on one of those — are **recorded rather than enforced**.
They bind whoever writes the file; refusing a decodable `ICCBased` miniature would lose a picture
to enforce a rule about writing, and one corpus document (`issue19517.pdf`) writes exactly that.

`Dictionary::remove` is new in `pdf-syntax`, and its doc comment says why a crate that models
what a file says now has a way to take an entry away: because here the standard does.

## A thumbnail is a second statement about a page

This is the part worth keeping. Every other check in this tree compares us against the
specification or against another implementation. A `/Thumb` is neither: it is **the producing
program's own picture of the page**, stored in the same file as the page. `tests/thumbnails.rs`
therefore renders page one at the thumbnail's scale, reduces both to an 8×8 luminance grid, and
prints the mean difference — a deliberately weak instrument, since a thumbnail may be 76 pixels
wide and was resampled by a filter nobody states.

All 11 corpus thumbnails decode. Three differ by more than 15 of 255, and **every one of the
three is the thumbnail's fault**:

- `issue11144_reduced.pdf` — pdf.js cut this file down from a larger one; the miniature still
  shows the tables the page no longer has.
- `issue19326.pdf` — a 1×1 thumbnail, which states one average colour and can agree with nothing.
- `transparency_group.pdf` — the thumbnail draws four petals and the page draws two overlapping
  ellipses. **All five renderers including us draw the ellipses** (the oracle calls the page
  ambiguous at worst mean 0.87 against a bound of 1.00), so the file disagrees with itself and
  the picture in it is stale.

So the instrument's finding so far is about the instrument: where a page and its own miniature
disagree, it has been the miniature every time. The three are a ratchet, and a *fourth* name
appearing is the case worth a person's eye — which is the only reason to keep the comparison at
all.

## Consequences

- `silent` falls 89 → **86**: §12.3.4 becomes `partial` (the panel, and generating a missing
  thumbnail, which the clause permits and principle 2 forbids on the launch path), and §12.3 and
  §12.3.1 stop claiming that nothing in the family is implemented — a parent row that had been
  wrong since the forty-ninth session, the same shape ADR 0080 found one clause over.
- No gate moves: a thumbnail is not drawn on a page.
- `crates/pdf-model/tests/thumbnails.rs` writes side-by-side PNGs under
  `<target>/tmp/thumbnails/`, because a number saying two pictures differ never says which is
  wrong.
