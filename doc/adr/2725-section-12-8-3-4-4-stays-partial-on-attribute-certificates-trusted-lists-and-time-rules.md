# 2725 — §12.8.3.4.4 stays `partial` on attribute certificates, trusted lists and the time rules

Status: accepted. Session 1955. Context: ADRs 1035, 2624, 2721, 2724; ETSI TR 102 272 clauses 6.6 to 6.10; ETSI TS
119 172-2 clauses 4.22 to 4.25; RFC 5280 sections 4.2.1.10 and 7.1. The ETSI texts are cited and paraphrased (ADR
1085).

## The reading

ADR 2724 builds the first two lines of ADR 2721's table: a trust point's acceptable policy set, name constraints
and policy constraints, and the time-stamping authority's name constraints, are evaluated. Read against the code's
own reason for each entry that is still named and not evaluated (round 1948's habit), the residue is:

| constraint | why it is named | a limit, or work |
|---|---|---|
| attribute trust condition (6.9), `aaCertAlgorithmConstraints` | attribute certificates are not read | work |
| trusted-list anchors | a trusted list is fetched; a host could fetch it as it fetches a policy (ADR 1728) | work |
| freshness and retention of revocation material (4.23.3) | the material's issue instant against a duration, not yet compared | work |
| clause 6.8's caution period and signature time-stamp delay | TS 119 172-3's delay is enforced; TR 102 272's spelling is not yet routed to it | work |
| a mandated certification path other than the signature's (`UseCertPath`, `userCertPath`) | a path the policy supplies is not yet validated in place of the one built | work |
| a distinguished-name value outside ASCII that differs from its constraint | RFC 4518's mapping and normalisation tables are not in this tree, so section 7.1's comparison is undecided | work |
| a name form section 4.2.1.10 constrains and this program does not compare (`otherName`, `x400Address`, `ediPartyName`, `registeredID`) | refused only where a constraint names that form, as RFC 5280 section 4.2.1.10 allows | limit |
| a subtree stating a minimum or maximum distance | RFC 5280 section 4.2.1.10 says the fields are not used | limit |
| an XML acceptable policy that is a URI and no object identifier | nothing a certificate's `certificatePolicies` can assert | limit |
| a revocation requirement of `other` | a policy extension's mechanism, whose syntax its identifier names | limit |
| human-readable policies | prose no validator enforces | limit |
| a C caller's fetched copy | the host lane's call (ADR 2624) | work, not this crate's |

## Decision

The residue still holds work, so under ADR 2624's rule the row stays `partial` (ADR 1035), and no parent row
moves (ADR 1599). The policy-tree output a trust point's acceptable set yields with no explicit-policy requirement
is RFC 5280's and is said in the sentence; whether a stated set should bind on its own is Q796, which changes no
status either way.
