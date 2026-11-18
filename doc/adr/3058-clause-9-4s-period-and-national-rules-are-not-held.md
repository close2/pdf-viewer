# 3058 — Clause 9.4's period and national rules are not held, and what would bring them

Session 2089, a signatures round (`doc/todo/77` section 2 item 3). Status: accepted (nothing built).
Rows: §12.8.3.4.5. Context: ADRs 2896, 2927, 2928 (its item 5, restated here with its reason);
ETSI TS 119 312 V2.1.1 clauses 9.4 and 9.5; ETSI TS 119 172-1 V1.1.1 clause A.3.2.5; TS 119 172-2
V1.1.1 clause 4.13 (`SigningPeriod`); TS 119 172-4 REQ-4.5-01 h). Cited by clause and paraphrased.

## Context

Clause 9.5's last sentence applies clause 9.4's rule to the other keys: a trust anchor shall stay
secure for the whole period during which the signature needs to be verified. Record 2057 left it
and REQ-4.5-01 h)'s national rules unasked; this round read whether any data names either.

## Decision

1. **The period is a business policy's statement, and no input carries it.** TS 119 172-1 clause
   A.3.2.5, BSP (j), has a signature policy's author state the signature's expected longevity — the
   period up to which it shall be verifiable — as prose in the business policy. Neither machine
   form states it: TS 119 172-2's `SigningPeriod` bounds when the policy may be used to *sign*, and
   TR 102 272 has no such field; the PDF, the CMS object, the DSS and the trusted lists state none.
   So no instant is invented — a default such as "now" or a certificate's `notAfter` would be a
   policy this program chose — and clause 9.4 and 9.5's last sentence stay unasked.
2. **What would bring it**: a host stating the period (a validation input like `Acceptance`), or a
   machine-readable policy element carrying it. Then each anchor's and other key's rows would be
   asked to outlast that instant as clause 9.3 is asked (ADR 2927 items 1 and 2).
3. **No national rules are held, and none is needed.** REQ-4.5-01 h) asks for the issues against
   "either" national rules or TS 119 312, naming which was used; the report uses TS 119 312 and
   says so (ADR 2896). A national catalogue would be a host's choice of jurisdiction and document,
   which no input names; supplying one is a host setting, not a reading of the clause.
