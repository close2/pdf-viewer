# 2883 — Annex A's time-stamp type is a silence of TS 119 172-4

Session 2033, a signatures round (`doc/todo/77` item 8). Status: accepted. Rows: §12.8.3.4.5.
Context: ADRs 2866, 2872, 2878; ETSI TS 119 172-4 V1.1.1 clauses 4, 4.4.1, 4.4.2, REQ-4.5-01 d)
iii) 3) and its NOTE 3, REQ-4.5-02 and Annex A; TS 119 615 V1.3.1 clauses 4.6 and 4.7, cited by
clause and paraphrased (ADR 1085).

## Context

Annex A item 2) lists seven digital signature types, the seventh "EU qualified electronic time
stamp" with `id-etsi-dst-euqtst` (`0.4.0.191724.1.2.7`). `verdict::level::Type` carries four of
the others, each from clause 4.4.2's join (ADR 2866). ADR 2878 reports a document timestamp by TS
119 615 clause 4.7's answer and gives it no type, until a reading finds a join over a token.

## The reading

The clauses' own titles were read before this was recorded. Clause 4 is "signature applicability
rules for the validation of EU qualified electronic signatures/seals"; clause 4.4.1 says its
process's main output is the suitability of a signature to implement an EU qualified electronic
signature or seal; REQ-4.4.2-02 to -06 are conditions on a signing certificate and its key's
device, and no requirement in clause 4.4.2 names a time-stamp token. REQ-4.5-02 says only that
Annex A's identifiers *may* be used to identify the type of a digital signature in the report.
The one sentence about a token's qualification is REQ-4.5-01 d) iii) 3) — the indication whether
a time-stamp is qualified — and its NOTE 3, which says TS 119 615 clause 4.6 can be used to
determine it. That indication is already reported from clause 4.7's two runs (ADR 2878).

So TS 119 172-4 states no process that assigns type vii): it defines the identifier and nothing
that decides when it applies. That is a silence, and this records it as one.

## Decision

A document timestamp is reported by clause 4.7's answer, in the clause's words, and given no
Annex A type. Attaching `id-etsi-dst-euqtst` wherever clause 4.7 answers "qualified" would be a
choice of this program's presented as the standard's mapping, which principle 5 forbids; and the
identifier would tell a reader nothing the clause 4.7 sentence does not.

## Reopened when

A version of TS 119 172-4, or a document it names normatively, states a process whose output is a
time-stamp token's type.
