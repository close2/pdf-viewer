# 2715 — §12.8.3.4.4 stays `partial` on the path constraints this program has not built

Status: accepted. Session 1950. Context: ADRs 1035, 2624, 2711, 2714; ETSI TR 102 272 clauses 6.6 to 6.10; ETSI
TS 119 172-2 clauses 4.22 to 4.25.

## The reading

ADR 2711 named three things as the row's owed work: the trust conditions against the policy's own trust points,
clause 6.10's issuer lists and the full-path requirements. ADR 2714 builds the signing-certificate trust
condition, the end-entity and CA issuer lists and both full-path requirements. Read against the code's own reason
for each, what is still named and not evaluated is:

| constraint | why it is named | a limit, or work |
|---|---|---|
| acceptable policy set, policy constraints | RFC 5280 section 6.1's policy tree, which `trust` does not process | work |
| name constraints of a trust point | section 6.1's name processing, likewise | work |
| revocation checks a condition mandates | `revocation` answers the worst over the path, not which kind of check answered each certificate | work |
| time-stamp trust condition (6.8, 4.25) | the TSA's certificate needs its own path, with the same machinery | work |
| attribute trust condition (6.9), AA list | attribute certificates are not read | work |
| TSA issuer list | as the time-stamp condition | work |
| trusted-list anchors | a trusted list is fetched; a host could fetch it as it fetches a policy (ADR 1728) | work |
| human-readable policies | prose no validator enforces | limit |
| a C caller's fetched copy | the host lane's call (ADR 2624) | work, not this crate's |

## Decision

The residue still holds work, so under ADR 2624's rule the row stays `partial` (ADR 1035): a status is a claim
about its residue. The note names the evaluated constraints and the list above. The next round in this row takes
the policy tree and name constraints into `trust` (they also lift `trust`'s own refusal of certificates carrying
those critical extensions), or the time-stamp condition, which reuses `certification_path::search` with the TSA's
certificate as the target.
