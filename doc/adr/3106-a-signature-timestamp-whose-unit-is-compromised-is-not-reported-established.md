# 3106 — A signature timestamp whose unit's path is compromised is not reported established

Session 2101. Status: **implemented.** Context: ADR 2968 (the best signature time declines a compromised or
earlier-revoked unit), ADR 1071 (a token's four steps), ADR 3056 (a token's own path in TS 119 312 clause 9.5).
Clauses: ISO 32000-2 §12.8.3.4.8, §12.8.3.3.1; ETSI TS 119 172-4 REQ-4.5-01 d) iii).

## The defect

`verdict::applicability::Report` states, for each time that bears on a signature, its "evidential relevance and level of
assurance" (REQ-4.5-01 d) iii)), and for a token that is `Assurance::Token`'s `established`: whether ADR 1071's four
steps held. Steps 1 to 4 hold for a token whose time-stamping unit's path is revoked — revocation is what the path
carries (`Time::Established::revocation`), not a step — so a signature timestamp whose unit was revoked for
`keyCompromise` was reported "established by the token's validation" (`viewer_core::notes`'s words), while
§12.8.3.4.8 says of exactly that case: "the UTC time included in the timestamp token cannot be considered as reliable
and the time indicated in the timestamp token cannot be used". The best signature time already honoured the clause
(ADR 2968), so the verdict was right and the report's line beside it was not.

No report-level test built a signature whose `signature-time-stamp` token carries its unit's certificate chain, which
is why it went unseen: the defect showed the first time one did.

## The decision

For this signature's own timestamp, `established` is the four steps **and** §12.8.3.4.8's three cases on the unit's
path, asked at the instant the path was validated (the clause's "current UTC time"): nothing revoked, usable; revoked
for `keyCompromise`, `cACompromise` or no stated reason, not; revoked for another reason, usable only where the
revocation is after `genTime`. The rule is the one `viewer_core::notes::signature_timestamps_instant` applies, asked in
`applicability::reliable`.

The field keeps its type, so no host changes: a host's sentence for an unusable token reads "not established", which
is §12.8.3.4.8's own conclusion. Two things are deliberately unchanged:

- **Document timestamps.** §12.8.3.4.8's paragraph is about "a timestamp token is already present in the CAdES
  signature as a signature timestamp attribute"; a document timestamp's chain is §12.8.5.3's, which the chain in
  `timestamp` already asks.
- **Clause 9.5's keys.** The unit's path is still asked of TS 119 312, because a key the report names as weak is weak
  whether or not the time it signed is usable.

## Tests

`verdict::applicability::tests::a_signature_timestamps_unit_and_its_authority_are_reported_validated_trusted_and_listed`
signs `policy_rules::revocation_checks`'s generated token (unit and intermediate carried, root supplied) into a document
and reads the report: established under the root with current CRLs; not established with no anchor; not established
with the intermediate's CRL listing the unit `keyCompromise` (the plant: with the rule removed, this assertion fails);
established through a list's `TSA/QTST` service, which d) i) names and clause 4.7 answers `INDETERMINATE` for a unit
stating no country; and clause 9.5's unit and authority at position 1 in each validated case.
