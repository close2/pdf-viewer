# 2835 — A listed service is not a qualification level

Session 2008, a signatures round (`doc/questions/A800` item 3). Status: accepted. Rows: §12.8.1,
§12.8.3.4.5. Context: ADR 2834; ETSI TS 119 172-4 V1.1.1 clause 4.4.2; ETSI TS 119 615 V1.3.1
clauses 4.4 and 4.5; ETSI EN 319 412-5. Cited by clause and paraphrased (ADR 1085).

## Context

The owner asked for the level a signature has — qualified (QES), advanced on a qualified
certificate (AdES/QC), advanced (AdES). TS 119 172-4 clause 4.4.2 defines the determination:
REQ-4.4.2-01 runs TS 119 615 clause 4.4 (is the signing certificate an EU qualified certificate,
for signatures or for seals, at the best signature time), REQ-4.4.2-04 runs clause 4.5 (was it made
by a qualified creation device), and REQ-4.4.2-06 joins them with the validation's `TOTAL-PASSED`.
Clause 4.4's processing is fourteen pages — the certificate's QC statements (EN 319 412-5), the
service's qualifications extension (TS 119 612 clause 5.5.9.2) with its criteria lists and
qualifiers, the status at the instant, and the transitions between the 1999 Directive and the
2014 Regulation — and clause 4.5 three more.

## Decision

A `verdict::Listing` records what clause 4.3 returned — the list, the provider, the service, its
type, its status when the certificate was issued — and is never worded as a level. The sentence a
report prints for a listed path says that the level is not determined by this build. Neither
"qualified" nor "QES" appears in any report until clauses 4.4 and 4.5 are executed, because a
reader told *listed by Austria as a qualified certificate service* would otherwise hear *qualified
signature*, which needs the certificate's own statements and the device as well.

## Consequences

`doc/todo/77` carries the plan: clause 4.1 and 4.2 first (a level resting on an unauthenticated list
would be worth less than its words), then EN 319 412-5's QC statements, then clause 4.4, clause 4.5,
and TS 119 172-4 REQ-4.4.2-06's join into a `Level` beside `Standing`.
