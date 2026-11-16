# 2874 — A policy's trusted-list anchors are determined from the lists a host supplied

Session 2029, a signatures round (`doc/todo/77` item 5). Status: implemented. Rows: §12.8.3.4.4.
Context: ADRs 1076, 1085, 1728, 2714, 2725, 2782, 2834, 2842; ETSI TS 119 172-2 V1.1.1 clause 4.22.2,
TS 119 172-3 V1.1.1 clause 4.22.2, TS 119 612 V2.3.1 clauses 5.3.13 and 5.5. Cited by clause and
paraphrased (ADR 1085).

## Context

A signature policy states its signing-certificate, time-stamp and attribute trust conditions as
trust points (ADR 2714). TS 119 172-2 clause 4.22.2.1 lets a trust point give its anchors two ways:
a) a certificate with an optional reliable-until instant, which was built; and b) a reference to a
TS 119 612 list — a specific list, or a list of trusted lists — with optional service types,
service statuses and a reliable-until instant, which was counted and never searched. TS 119 172-3
clause 4.22.2.1 adopts the same semantics for its ASN.1 `TAsInTrustedList` (`uri`, `serviceTypes
[0]`, `serviceStatuses [1]`, no instant). ADR 2782 named b) as work; the list reader and the
host's `--trusted-lists` (ADRs 2834, 2842) are what it needed.

The clause's procedure is three steps: the initial set is every certificate of the referenced
specific list, or of every specific list a referenced list of trusted lists points at; step 2 keeps
only those of a service whose type is listed; step 3 keeps only those of a service whose status *at
current time* is listed. Its Example 3 says the initial set is the certificates *corresponding to
the trusted services listed*.

## Decision

1. **The anchors are the services' digital identities** (TS 119 612 clause 5.5.3), current and
   historical, each once — Example 3's reading of "the certificates present in the TL". A list's
   signing certificate and its pointers' certificates are about the list, not about a service.
2. **A list is a list of trusted lists where it lists no service.** That is the clause's own
   definition (it "does not list trusted services but pointers"), and it decides the case
   without reading `TSLType`, whose values are scheme-specific.
3. **A reference is resolved among the host's files by ADR 2842's names**: the file named the
   reference's last path segment, else — where a list of trusted lists points at that location —
   the list of the pointer's territory, as TS 119 615 clause 4.2 places a Member State's list. A
   list of trusted lists' own `EU` pointer is itself, and a pointer stating no territory names a
   list this program cannot place. `trusted_list::Reading` now keeps each list's file name and the
   list of trusted lists clause 4.1 authenticated, which clause 4.1 had used and dropped.
4. **"Current time" is the host's instant the lists were read at** (`anchors::Supplied::now`),
   since this program reads no clock of its own; the path is still validated at the policy's
   instant (ADR 2714). With lists and no anchor directory the host's clock comes with the lists
   (ADR 2842), so `viewer_core::notes` now gives the policy's search that instant too.
5. **Each anchor is a trust point with the referencing one's other members** — `pathLenConstraint`,
   the RFC 5280 section 6.1.1 inputs, a mandated path — and the tuple's reliable-until instant, so
   a listed anchor is met, broken, past reliable or refused exactly as a stated one is, and the
   sentence names the list that determined it.
6. **What is not examined is counted, never dropped**: a reference with no lists supplied, to a
   file not supplied, to one `Lists::read` refused (not reliable: TS 119 615 clauses 4.1 and 4.2),
   and each specific list a referenced list of trusted lists points at that was not supplied. A
   condition with one is never called broken (EN 319 102-1 clause 5.2.6.4's indeterminate).
7. **Only anchors that could end a path are tried.** A Member State's list holds hundreds of
   services and the search tries at most `MAX_TRUST_POINTS`; RFC 5280 section 6.1 closes a path on
   an anchor whose name is the topmost issuer, and `trust` compares those names as octets, so an
   anchor whose subject no certificate in the pool names as its issuer can end no path the search
   builds. Dropping it changes no outcome and keeps the bound for the anchors that matter.

## Consequences

`TrustTrees::trusted_lists: usize` became `TrustTrees::listed`, `SignerFacts::lists` and
`SignerFacts::with_lists` carry the host's reading, and `search_certificate` takes it for an
attribute authority's path. `Found::Reached` says which list determined its anchor. Both policy
parsers read the tuple whole; the existing `policy_document` fuzz target reaches both. A list
nobody supplied is still not fetched: that is `doc/todo/77` item 4, a host's transmission at A98's
`Submissions` level through ADR 1728's port.
