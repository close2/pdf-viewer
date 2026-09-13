# 1021 — What the `/ByteRange` leaves out

Date: 2026-09-13. Branch `batch-1020-1025`, worktree shared with five sibling rounds. No ADR:
nothing here overturns a decision, and the argument is in the code.

## The finding

`Signature::coverage` is arithmetic over `/ByteRange`'s pairs — and **no arithmetic over the pairs
can see what sits in the region between two of them**. That region is the one place in a signed
document where bytes lie that no digest was taken over, and §12.8.1 names one thing that may be
there: "the signature value itself (the Contents entry)", which Table 255's `/ByteRange` row repeats
as a `shall`. Nothing here read it: a file whose hole swallows the bytes after its signature value
answered `Coverage::WholeFile`, recomputed its digest to what the signature recorded, and verified
under the signer's own key — every answer green, with unsigned syntax inside the file.

## Built, in `crates/pdf-signature/src/signature.rs`, reported by `viewer-core`'s `notes`

- **`Signature::excluded`** reads the region off the file a window at a time (`HexScan`, which
  decodes §7.3.4.3's hexadecimal string without holding it) and compares its octets with
  `/Contents`: `TheSignatureValue`, `TheDigitsOfTheSignatureValue` (§12.8.3.3.1's "shall fit
  precisely" unmet, nothing hidden — `signed_verified.pdf`'s real shape), `NotTheSignatureValue`,
  `MoreThanOneRegion`, `Nothing`, two range refusals.
- **`Signature::signed_end`** is the other end of §12.8.1's sentence, "to the end of the \"%%EOF\"
  comment, possibly followed by an optional EOL marker" — which gives `Coverage::Unsigned` a
  meaning: a tail is NOTE 1's ordinary incremental update only if what precedes it is a *whole*
  revision.
- **`Signature::indirect_values`** is §12.8.1's "all values in the signature dictionary shall be
  direct objects", read from the dictionary rather than through `Document::get_key`, which resolves
  a reference without saying so.

Table 255 states the first of these twice, and the second statement is a document timestamp's: its
range "shall specify the complete PDF file contents (excepting the Contents value)".
`must_cover_whole_file` was the *complete* half and always had been; `excluded` is the *excepting*
half, and §12.8.5's fixture now asserts both.

## Calibration, the corpus, and what stays owed

The planted witness builds a **self-consistent** signed file whose hole swallows sixteen bytes past
the value, and asserts `WholeFile` and `Unchanged` *before* the refusal — otherwise the test proves
only that two readings of the same arithmetic agree. Over the corpus the two new questions have the
same four answers, neither list written by hand: the four signatures whose region is not their value
are the four that stop away from an `%%EOF`, and the four
`every_corpus_signature_is_asked_whether_its_document_changed` finds `Changed` — whose prose claimed
of three that their "`/ByteRange` no longer even brackets their own `/Contents`", unchecked.
§12.8.2.2.2's step two is still not done and the row still says so: the signed revision is not
reconstructed, and what is narrower is that its *boundaries* are now established rather than
assumed. No row moved to `implemented` — all four stay `partial` for trust, four curves, that step.
