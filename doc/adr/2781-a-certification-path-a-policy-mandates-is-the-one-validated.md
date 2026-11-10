# 2781 — A certification path a policy mandates is the one validated

Status: accepted. Session 1985. Context: ADRs 1085, 2714, 2724, 2725, 2775; ETSI TS 119 172-2 clause 4.22.5;
ETSI TS 119 172-3 clause 4.22.5; RFC 5280 section 6.1; ETSI EN 319 102-1 clause 5.2.6.4. The ETSI texts are cited
and paraphrased (ADR 1085).

## The reading

TS 119 172-2 clause 4.22.5.1 lists five components of a trust point. The fifth names the certification path to be
used, either explicitly or by pointing at the path the signature provides, and where its content contradicts one of
the four before it, the fifth takes precedence. TS 119 172-3 adopts the semantics; its syntax is `userCertPath`, a
choice of `asInSignature` and `path`, and TS 119 172-2's is `UseCertPath`, a choice of `AsInSignature` and a list of
`X509Certificate`. TR 102 272 (RFC 3125) has no such member. ADR 2725's table held the explicit form as work: both
parsers kept it by name and a condition stating it was never met.

Four things the clause leaves to a reading, and how each is read:

| question | reading |
|---|---|
| which certificates the path is built over | the listed ones, *in place of* those the signature carries, so the signature's own cannot stand in for the policy's |
| the order of the list | none is stated, so the order is the one issuer and subject names give a path, as RFC 5280 section 6.1 defines one |
| whether the list holds the signer's and the trust point's certificates | it may; either is on every path to that trust point, so neither is counted as left off |
| what the precedence acts on | `pathLenConstraint`, the acceptable policy set, the name constraints and the policy constraints: a mandated path longer than the constraint is not refused for its length, and one section 6.1 refuses under the trust point's inputs and validates under section 6.1.1's defaults is met |

A path that validates but leaves a listed certificate off is **not evaluated**, never broken: the search returns the
first path that validates and does not ask whether a longer one through that certificate would have. A listed
certificate this program does not read, or a list longer than any path the walk builds plus the trust point, makes
the trust point unexamined, as an unreadable anchor already is. `asInSignature` false names no path at all and stays
named and not evaluated.

## Decision

`TrustPoint::mandated_path` carries the list from both parsers; `certification_path::search_from` builds over it
(`pool`), checks every listed certificate is on the path (`left_off`), and lets the four components yield, each named
in the sentence through `Found::Reached`'s `mandated` member. The path handed to the full-path reference and to the
revocation checks is the mandated one. ADR 2725's row for a mandated path is closed; what it still holds as work is
attribute certificates, trusted-list anchors and the retention of expired revocation notices (ADR 2782), so
§12.8.3.4.4 stays `partial` (ADR 1035) and no parent row moves (ADR 1599).
