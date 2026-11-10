# 2775 — A signature policy's time rules bound which revocation material answers

Status: accepted. Session 1982. Context: ADRs 1067, 1085, 2720, 2725; ETSI TR 102 272 clause 6.8 and Annex B.5.3
to B.5.4; ETSI TS 119 172-2 clause 4.23.3; ETSI TS 119 172-3 clause 4.23.3.2; ETSI EN 319 102-1 clauses 5.2.5
and 5.5.4; W3C XML Schema Part 2: Datatypes Second Edition, section 3.2.6 and Appendix E. The ETSI texts are cited
and paraphrased (ADR 1085).

## The reading

ADR 2725's table named two rows of work that are about time: TS 119 172-2 clause 4.23.3's freshness (with its
retention), and TR 102 272 clause 6.8's caution period and signature time-stamp delay. Three of the four are rules
about **when the revocation material that answers a check was issued**, and one is the delay TS 119 172-3 already
enforces:

| member | what it says | how it is read |
|---|---|---|
| `Freshness` / `MaxDifferenceRevocationAndValidation` | issuance within a duration of the validation time | EN 319 102-1 clause 5.2.5.4: fresh where issued *after* validation time minus the maximum; step 1 takes it in place of the `thisUpdate`-to-`nextUpdate` interval |
| `Freshness` / `TimeAfterSignature` | issued a duration after the best-signature time | issued at or after best-signature time plus the duration, beside the material's own window |
| `cautionPeriod` (TR 102 272 6.8) | revocation status information not collected or used until after the period following the signing time | issued at or after best-signature time plus the period; and Annex B.5.3's wait, said as its own finding |
| `signatureTimestampDelay` (TR 102 272 6.8) | timestamp later than the signing-time attribute by more than the delay: invalid | the same `DeltaTime` and the same finding TS 119 172-3 clause 4.25 already produces |

The *issuance* of revocation status information is a CRL's `thisUpdate` and an OCSP `SingleResponse`'s
`thisUpdate`; EN 319 102-1 clause 5.2.5.4's NOTE 3 is the clause that separates the latter from `producedAt`. The
*best-signature time* is EN 319 102-1 clause 5.5.4's: a signature timestamp's generation time where one covers the
signature, otherwise the time validated at.

Material outside a bound does not answer, and a check only it would have answered is **not evaluated**, never
broken — EN 319 102-1's "try later". A bound applies to a *good* answer only: a revocation is a statement its
issuer made whenever it made it, which is ADR 1067 section 3's argument for the list's own window too.

The caution period is a member of the time-stamp trust condition but speaks of the signer's key and of the
revocation status information that completes the signature's validation data. That information covers the paths
of both conditions, so the bound is applied to the revocation checks of both. This is a reading of a clause that
does not name the certificates, and it is the stricter of the two available.

## The duration arithmetic

TS 119 172-2 types freshness `xs:duration` and TS 119 172-3 writes the same lexical form in a `PrintableString`,
citing the Second Edition of XML Schema Part 2 as reference [8]. A duration is six-dimensional, so a month is not a
number of seconds: `policy_rules::duration` implements Appendix E's sum (months first, the day pinned into the
resulting month, then the time components) and its own worked examples are the tests. The seconds component is
rounded up as a signed value to a whole second, and because every instant compared is a whole second (RFC 5280
section 4.1.2.5.2 forbids fractional seconds) both comparisons decide exactly as they would under the stated
fraction; the module comment gives the argument.

## Decision

Built as above: `revocation::Issuance` carries the two bounds into `status_issued`, `revocation_checks::Timing`
places them on the time line, `Requirement::freshness` is read in both TS syntaxes, and TR 102 272 clause 6.8's
members `[3]` and `[4]` are read instead of being named. What ADR 2725's table still holds as work is the
retention of expired revocation notices (TS 119 172-2 clause 4.23.3's second component), attribute certificates,
trusted-list anchors and a policy-supplied certification path, so §12.8.3.4.4 stays `partial` (ADR 1035) and no
parent row moves (ADR 1599).
