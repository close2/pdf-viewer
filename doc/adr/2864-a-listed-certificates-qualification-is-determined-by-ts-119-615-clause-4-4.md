# 2864 — A listed certificate's qualification is determined by TS 119 615 clause 4.4

Session 2024, a signatures round (`doc/todo/77` item 2). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 2834, 2835, 2842, 2853; ETSI TS 119 612 V2.3.1 clauses 5.4.1, 5.5.9.2 and 5.5.9.4;
ETSI TS 119 615 V1.3.1 clause 4.4; ETSI TS 119 172-4 V1.1.1 REQ-4.4.2-01; RFC 5280 section 7.1.

## Context

A service of type `CA/QC` publishes in its list entry what its certificates cannot say of
themselves: TS 119 612 clause 5.5.9.2's `Qualifications` extension, qualification elements each
pairing a list of assertions about a certificate (key usage bits, policies, extended key usages,
subject attributes, nested lists under `all`, `atLeastOne` or `none`) with qualifier URIs. TS 119
615 clause 4.4 joins those qualifiers with the certificate's EN 319 412-5 statements (ADR 2853) and
the service's status at an instant into three checks — for signatures, seals and web sites — by
Tables 1 to 3, refuses two purposes at once by Table 4, takes Directive 1999/93/EC's path and
Table 5 before 2016-06-30T22:00:00Z, and repeats itself at the certificate's `notBefore`. TS 119
172-4 REQ-4.4.2-01 runs it at the best signature time.

## Decision

1. **`trusted_list::qualifications` reads clause 5.5.9.2** into types that keep everything it cannot
   evaluate by name — an indicator the clause does not give, a key usage bit X.509 does not name, a
   criterion clause 5.5.9.2.2.3 leaves to the scheme operator, an identifier that is not an OID —
   and `CriteriaList::identifies` answers three ways. Reading never refuses a list for them; the
   list's existing bounds hold the recursion.
2. **Clause 5.5.9.2.0's critical flag decides what an undecided answer costs.** Under a critical
   extension the certificate is discarded, which is reported as INDETERMINATE with this reader's
   own named sub-status; under a non-critical one the element is set aside with a warning. The
   clause states the first and is silent on the second; setting aside is the reading of a
   non-critical extension RFC 5280 section 4.2 gives every other one, and the warning keeps it loud.
3. **`trusted_list::determination` is PRO-4.4.4-01 to -36**, Tables 1 to 3 transcribed cell by
   cell as printed (not derived from a pattern, so a cell reads against the page), Table 4 and
   Table 5 as the rules their cells state. Three sentences of the clause cannot be taken literally
   and are read by their siblings, each written in the module documentation: PRO-4.4.4-18A c) sets
   `CHECK_1` in the seal branch, PRO-4.4.4-28 b) sets `CHECK_3` to a sub-status, and PRO-4.4.4-33
   f) 3) stops with no results set.
4. **PRO-4.4.4-06 b)'s strategy is EXAMPLE 1 ii), then i)**: the issuer's name equal to the subject
   of a certificate of the service's identity, else its `commonName` one of the provider's names;
   names compare by RFC 5280 section 7.1, and text this tree cannot prepare is said as undecided
   rather than as a mismatch. `Provider::names` holds every language's name and trade name.
5. **No list of the issuer's territory is its own answer**, never "Not_Qualified": clause 4.3's
   `CC` names the Member State's list, and a host that did not supply it has not supplied the
   procedure's input. The latest issue of a territory's list is the one read.
6. **The determination rides on the listing** (`verdict::Listing::qualification`), computed once
   per signature at the best signature time, and `viewer_core::notes` says it in the clause's own
   indications, with the list, as being about the certificate and not the signature's level. No
   report words QES or AdES/QC: that needs clause 4.5 and REQ-4.4.2-06 (ADR 2835).

## Consequences

The level is still not said; clause 4.5 (QSCD) is ADR 2865, and the join is `doc/todo/77` item 3.
Clause 4.3's duplication warnings are not copied into `QC-Sub-Status`, because clause 4.4 reads
only its errors. `fuzz/`'s `trusted_list` target runs clause 4.4 on every identity of a fuzzed list,
twice.
