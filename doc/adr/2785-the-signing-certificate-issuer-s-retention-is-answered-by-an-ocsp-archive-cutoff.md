# 2785 — The signing certificate issuer's retention is answered by an OCSP archive cutoff

Status: accepted. Session 1987. Context: ADRs 1067, 1085, 2720, 2725, 2775, 2782; ETSI TS 119 172-2 clause 4.23.3;
ETSI TS 119 172-3 clause 4.23.3; RFC 6960 sections 4.2.1, 4.2.2.1, 4.2.2.2 and 4.4.4; XML Schema Part 2 section
3.2.6 and Appendix E. The ETSI texts are cited and paraphrased (ADR 1085).

## The reading

TS 119 172-2 clause 4.23.3.1 item 2, for the revocation trust of the signing certificate only, lets a policy mandate
that the signing certificate's issuer keep revocation notices for revoked certificates after they expire, for a
period exceeding a lower bound the component states; clause 4.23.3.2 types it `xs:duration`, and TS 119 172-3's
`sigCertIssuedByCAKeepsExpiredRevokedCertsInfo [1] Duration` is the same lexical form in a `PrintableString`. Both
parsers named it and no round evaluated it. ADR 2782 found that the file can state it: RFC 6960 section 4.4.4 lets a
responder that retains revocation information past a certificate's expiry carry, in the certificate's
`singleExtensions`, an archive cutoff of syntax `GeneralizedTime`, which is `producedAt` less the retention interval.

| question | reading |
|---|---|
| whose statement counts | a response naming the signing certificate whose responder RFC 6960 section 4.2.2.2 authorises for it — the issuing CA or its delegate — so the statement is the issuer's; any other speaks for nobody |
| what "exceeding" means | strictly longer: the cutoff falls strictly before `producedAt` less the bound, by Appendix E's sum. A fractional bound is decided exactly, not by its rounding: `Duration` keeps whether it rounded, and `exceeded_between` takes *at or before* the rounded instant where it did and *strictly before* where it did not |
| several responses | one authorised statement exceeding the bound meets it; where all fall short the longest is the one named |
| a statement that falls short | **broken**: the issuer, through the responder that speaks for it, has stated a retention that does not exceed the bound. The other reading, that the CA might keep notices longer by a channel the file does not show, was weighed and set aside: the requirement is about the issuer's practice and the issuer's responder has stated it |
| no statement | **not evaluated**: a response without a cutoff, one whose cutoff is not a `GeneralizedTime` (section 4.2.2.1 gives a response's times RFC 5280's form), an unauthorised one, or none at all states nothing about retention, and absence is not evidence of a short one (ADR 1067) |
| the policy's freshness and caution period | they bound which status information answers a check at an instant; a cutoff is a statement of practice, so they do not bound which response states it |
| the member on another condition | clause 4.23.3.1 defines it for the signing certificate only, so on TS 119 172-2's `TstRevocationTrust` it stays named and not evaluated |

## Decision

`revocation::SingleResponse::archive_cutoff` reads the extension, at most `MAX_EXTENSIONS` entries deep, leaving an
unplaceable one absent rather than refusing the response; `revocation::stated_retention` returns every cutoff an
authorised response states for a certificate, or `Undetermined::NoArchiveCutoff` and its siblings; and
`revocation_checks::retention` judges `Requirement::retention` over the signer's validated path, reported as
`Constraint::Retention`. ADR 2782's row for retention is closed. What it still holds as work is attribute
certificates and their trust condition and trusted-list anchors, with ADR 2725's limits, so §12.8.3.4.4 stays
`partial` (ADR 1035) and no parent row moves (ADR 1599).
