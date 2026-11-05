# 2721 — §12.8.3.4.4 stays `partial` on the policy tree, names, attribute certificates and trusted lists

Status: accepted. Session 1953. Context: ADRs 1035, 2624, 2715, 2720; ETSI TR 102 272 clauses 6.6 to 6.10; ETSI TS
119 172-2 clauses 4.22 to 4.25.

## The reading

ADR 2715 listed what was left as work. ADR 2720 builds two of them: the time-stamp trust condition with its
issuer list, and the revocation checks a condition mandates by kind. Read against the code's own reason for each
entry, what is still named and not evaluated is:

| constraint | why it is named | a limit, or work |
|---|---|---|
| acceptable policy set, policy constraints of a trust point | RFC 5280 section 6.1's policy tree, which `trust` does not process | work |
| name constraints of a trust point, and the authority's (6.8, 4.25.1 item 3 d)) | section 6.1's name processing, likewise | work |
| attribute trust condition (6.9), `aaCertAlgorithmConstraints` | attribute certificates are not read | work |
| trusted-list anchors | a trusted list is fetched; a host could fetch it as it fetches a policy (ADR 1728) | work |
| freshness and retention of revocation material (4.23.3) | the material's issue instant against a duration, not yet compared | work |
| clause 6.8's caution period and signature time-stamp delay | TS 119 172-3's delay is enforced; TR 102 272's spelling is not yet routed to it | work |
| a revocation requirement of `other` | a policy extension's mechanism, whose syntax its identifier names | limit |
| human-readable policies | prose no validator enforces | limit |
| a C caller's fetched copy | the host lane's call (ADR 2624) | work, not this crate's |

## Decision

The residue still holds work, so under ADR 2624's rule the row stays `partial` (ADR 1035). The next round in this
row takes the policy tree and name constraints into `trust`, which also lifts `trust`'s refusal of certificates
carrying those critical extensions and serves the trust point's and the authority's name constraints alike.
