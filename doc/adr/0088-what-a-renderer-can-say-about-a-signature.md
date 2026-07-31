# ADR 0088 — What a renderer can say about a signature

Status: accepted, 2026-08-01.

## Context

§12.8 was forty `silent` rows — the largest block left in the ledger — behind one true sentence:
this program cannot verify a digital signature. Verification needs a certificate, a chain and a
revocation check, which is the public-key infrastructure §7.6.5's security handlers are already
refused for (ADR 0031).

That sentence justified reading none of the clause, and it should not have. §12.8.1 divides the
work itself, and only two of its three parts need cryptography.

## Decision

**Read every signature; verify none; and make the one check that needs no cryptography.**

`pdf-model/src/signature.rs` reads Table 255 whole, §12.8.6's permissions dictionary, and
§12.8.2.2's `/P` level — an author's statement of "what changes shall be permitted to be made to
the document and what changes invalidate the author's signature".

The check is `Signature::coverage`. §12.8.1 says what a byte range digest covers:

> This range should be the entire PDF file, including the signature dictionary but excluding the
> signature value itself (the Contents entry).

So a `/ByteRange` that stops short of the end of the file names bytes **nobody signed**, and
finding that out costs one comparison against the file's length. It is not a validity verdict and
the module says so twice: a range covering everything may still be forged, and one that stops
short may be a perfectly honest later revision.

**The digest is deliberately not computed.** It would be cheap and it would be misleading:
without the certificate half, a hash that matches proves the bytes are the bytes the *stored*
hash was made from and nothing about who made it — and a caller shown "digest matches" will hear
"signature valid". The refusal is a design decision about what this program is willing to imply.

## What the corpus says, and one file that is the clause's own example

- 6 documents carry a signature in a signature field; 4 of their ranges run to the end of the
  file and 2 do not.
- **`xfa_filled_imm1344e.pdf` is §12.8.2.2 demonstrated.** It holds the corpus's only
  certification signature, its `/Perms /DocMDP` states `/P 2` — which "permit[s] modifications
  that are appropriate for form field or comment workflows" — and **2 542 822 bytes** were
  appended after the signed range: a filled-in form, saved by incremental update exactly as
  §12.8.1's NOTE 1 describes. Whether those bytes contain *only* permitted changes is
  §12.8.2.2.2's question, needs the digest, and is not answered here. That they are there is.
- 4 documents state a `/Perms`, all four with a `/UR3`.

## Saying it out loud is what makes fifteen rows `reported`

`viewer-ui` now prints, once per document: who signed it, why, whether bytes were appended after
the signature — and, as its own line, that this program verifies nothing and has no certificate
store. That last line is the point. A viewer that draws a signed document in silence lets a
person assume the signature was checked; §12.8.3's fifteen rows are `reported` rather than
`silent` because the omission is now stated to the person who would otherwise assume it.

This is the same move as the previous session's nine refused actions, one clause over, and it is
becoming the project's standard answer to "we cannot do this": **not silence, and not a refusal
to open the file — a sentence.**

## Consequences

- `silent` falls 49 → **27**, and `reported` rises 36 → 51. Twenty-two rows move.
- The whole ledger's remaining silence is 27 rows of clause 12: FDF, the DSS and document
  timestamps, collections' user interface, and the actions nobody has built.
- No gate moves: a signature is not ink.
