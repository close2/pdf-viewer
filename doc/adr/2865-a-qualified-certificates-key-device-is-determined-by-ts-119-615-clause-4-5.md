# 2865 — A qualified certificate's key device is determined by TS 119 615 clause 4.5

Session 2024, a signatures round (`doc/todo/77` item 3, its first half). Status: implemented. Rows:
§12.8.3.4.5. Context: ADRs 2835, 2864; ETSI TS 119 612 V2.3.1 clause 5.5.9.2.3; ETSI TS 119 615
V1.3.1 clause 4.5; ETSI EN 319 412-5 V2.4.1 clause 4.2.2.

## Context

ETSI TS 119 172-4 REQ-4.4.2-04 asks TS 119 615 clause 4.5 whether the signing certificate's key
resided in a QSCD at the best signature time. The clause starts from clause 4.4's outputs: where the
certificate was confirmed `QC_For_eSig` or `QC_For_eSeal`, the qualifiers of the elements that
identified it (`CHECK_1_SET-OF_QE` or `CHECK_2_SET-OF_QE`) choose a column and the certificate's
`QcSSCD` statement a row of Table 7, or under the Directive (an instant strictly before
2016-06-30T22:00:00Z) of Table 6, where the QCP+ policy stands beside the statement. Unknown
qualifiers fail the process under a critical extension; five combinations are inconsistent.

## Decision

1. **Clause 4.4 keeps its three sets** (`Determination::sets_of_qe`), each qualifier with its
   extension's criticality, because PRO-4.5.4-04 b) turns on the extension a qualifier came from.
   The sets are the run at the asked instant's, which is the run PRO-4.5.4-01 names.
2. **`determination::qscd` is PRO-4.5.4-01 to -05.** Tables 6 and 7 print the same three columns'
   cells and are written as one rule with a function each, so a test reads each table's six cells.
   "Known" qualifiers are the thirteen TS 119 612 clause 5.5.9.2.3 defines for `CA/QC`, as the
   NOTE to PRO-4.5.4-04 b) points at. Two references are slips read by context and written in the
   module documentation: PRO-4.5.4-03 4)'s "(a).a." and "(a).c.", and PRO-4.5.4-04 e)'s row "by
   point c)", which is point d)'s `QcSSCD`.
3. **It rides beside clause 4.4's outputs** (`Qualification::Determined::qscd`), at the same
   instant, and `viewer_core::notes` says it in the clause's indications (`QSCD_YES` and its
   siblings) as being about the certificate's key.

## Consequences

Still no level: TS 119 172-4 REQ-4.4.2-06's join into qualified signature or seal, advanced on a
qualified certificate, advanced, or indeterminate is `doc/todo/77` item 3's remainder, and only then
does a report say "qualified" (ADR 2835).
