# 1112 — The instant a token proves is what a verdict is asserted at

Session 1098. Status: **accepted**. Answers the residue §12.8.3.3.1's row has carried since ADR
1071 — the clause hands the treatment of a verified time-stamp token to the signature handler and
names nothing — by defining that treatment. Adds `verdict::BestSignatureTime` and `verdict::Proof`,
changes `Verdict::of` and `Verdict::reached` to take the first of them, and takes one comparison out
of `revocation::from_list` and `from_response`. Fetches ETSI EN 319 102-1 V1.4.1.

`§N` is ISO 32000-2 and nothing else. Nothing from ETSI's text is quoted anywhere in this tree
(ADR 1085): clause numbers are cited and the rules are paraphrased.

## 1. The clause hands the question over, and this tree is the party it is handed to

§12.8.3.3.1 reads the token's `genTime` and then says, of what to do with it, that "[t]he specific
treatment of this timestamp tokens and its processing is left to the particular signature handlers
to define." A row that records that and stops has recorded a *silence in the standard*, which
`CLAUDE.md` principle 5 says to answer by making a deliberate choice and documenting it as one.

The choice is not invented here. ETSI EN 319 102-1 clause 5.5.4 is a validation process in which the
instant is a named variable — best-signature-time, the earliest moment the existence of a signature
is proven — and the document is published without charge, which ADR 1085's rule makes a `curl`
rather than a question. Two of its clauses decide everything below: 5.5.4 for the instant and what
is compared against it, 5.2.5 for what makes a revocation list evidence.

## 2. The value, and the one rule that makes it

`verdict::BestSignatureTime` starts as the clock the host supplied and a token's established instant
lowers it, never raises it (clause 5.5.4 steps 1 and 3 b)). Three consequences, each a line of code
and each load-bearing:

- **A token stating a moment in the future proves nothing** and leaves the clock where it was. The
  old shape took a token's instant unconditionally, so a `genTime` ahead of the reader would have
  moved the question into a future nobody has evidence about.
- **The earliest of the established tokens wins**, rather than the first one asked. §12.8.3.4.5 (b)
  names three sources and states no order among them; ADR 1085 ordered the *asking*, and clause
  5.5.4 settles the *choosing* — what is wanted is the earliest proof, so a later one adds nothing.
- **`Proof` says which token supplied it**, and its first variant is the absence of the other two.
  The distinction is what the two carve-outs below turn on: the reader's clock is not a proof of
  anything, so nothing may be excused by it.

## 3. What is computed at the instant, and the comparison that was a defect

- **RFC 5280 section 6.1.3 (a)(2)'s validity period.** Already an input to `trust::validate_for`, so
  this needed no code — only a value that is sometimes not now. A certificate that has since expired
  validates a signature made while it was current, which is what §12.8.4.2 says a document security
  store exists for.
- **A revocation's own date** (clause 5.5.4 step 4) a)). A revocation that took effect after a
  *proven* instant does not reach the signature; one before it, or one where nothing proved an
  earlier moment, still does. **RFC 5280 states no such rule and could not**: section 6.3.3's steps
  (i) to (k) set `cert_status` from a CRL entry without reading its `revocationDate` at all. This is
  the one rule in this ADR that holding ETSI's document was strictly necessary for.

  **Which date, though, is RFC 5280's question, and the answer was a field nothing read.** Section
  5.3.2's `invalidityDate` is when the certificate is known or suspected to have become invalid and
  "may be earlier than the revocation date in the CRL entry", which is only "the date at which the
  CA processed the revocation". A carve-out measured from the processing date would excuse a
  signature made with a key the issuer says was already compromised, so `Revocation::Revoked` now
  carries both instants and the comparison takes the earlier. The two are kept as separate fields
  rather than one, because a report asking when the CA acted is asking a different question.
- **Freshness** (clause 5.2.5), and here the existing code was wrong in a way nothing could have
  caught while the instant was always now. It refused a list whose `thisUpdate` was *after* the
  instant as stale. Clause 5.2.5 makes the window one comparison rather than two — against
  `nextUpdate` — and a list issued after the instant is fresher evidence about it rather than none.
  Left as it was, moving the instant into the past would have made every long-term-validation
  document answer `Unknown`: a store is assembled after signing, so every list in one postdates the
  signature it is about. **One case keeps the old comparison and gains a reason**: an OCSP response
  stating no `nextUpdate` is one whose responder has said newer information is available all the
  time (RFC 6960 section 2.4), so the moment it was produced is the only end this reader can state,
  and a question put before that moment is not one the response answers.

## 4. What is not relaxed

`Valid` still has no public constructor and still takes an `Anchored` (ADR 1076 section 2). An
instant is established only through a path to an anchor a host supplied, so on every document in
this tree the instant is the reader's clock and both carve-outs are unreachable — which is the
honest state of a program whose default is that nobody has named an authority. `viewer_core::notes`
now says the instant and what fixed it beside *every* verdict, including that one, because "valid"
with no instant reads as *valid now* and that is the claim this program never makes.

## 5. What this does not close

The two curves. §12.8, §12.8.1, §12.8.3, §12.8.3.1, §12.8.3.3 and §12.8.3.3.1 stay `partial` for
ISO/TS 32002's brainpoolP512r1 and Ed448, whose blocker is a package's release state and is
re-measurable rather than permanent. Nothing about the instant touches them.
