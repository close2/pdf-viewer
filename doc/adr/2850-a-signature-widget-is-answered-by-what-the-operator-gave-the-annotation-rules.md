# 2850 — A signature widget is answered by what the operator gave the annotation rules

Session 2016, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/decision.rs`: `decide`,
`decide_as_underlying`; `archive/mod.rs`: the per-requirement loop passes `configured`).
Context: ADRs 1942, 2210; R16 of `doc/pdfa-blocking-issues.md`.
Clauses: ISO 19005-2 section 6.4.3, ISO 19005-4 section 6.5.1, both parts' section 6.3.2;
ISO 32000-2 §12.5.3 (Table 167).

## 1. What the census said

`signatures/signature-widgets-meet-the-annotation-rules` is a compound row: both parts ask the
annotation rules again of a signature field's widget, and `Answer::AsUnderlying` takes the most
constraining answer of the three rows it names. It read each row's *table* answer and never the
operator's. Both shipped profiles answer `annotations/flags-entry-present` with `preserve` (ADR
1942: the widget is kept and given the Print flag), so the flags row itself was answered, while the
compound row over the same widget reported `annotation-printing` unauthorised. `poppler-395-0-fuzzed`
(a signature widget stating no `/F`) and the two XFA forms `MOZILLA-1671648-0` and
`xfa_filled_imm1344e` were refused on it under `as-if-printed` and `as-shown` at both parts.

## 2. The decision

`decide` takes the operator's answer as a function of the row, and `decide_as_underlying` asks it
of each rule the compound names before the table's: the widget is one of the annotations that
rule's own rewrite reaches, so the conversion answers its places with that answer whatever row
they are reported under. A rule the operator did not answer keeps the table's answer, and the
ordering of `constraint` is unchanged.

- **Every profile, both parts**: no profile's choice moves. A profile that authorises nothing at
  the flags row keeps the unauthorised loss at the compound row too.
- **Nothing is lost** beyond what the rule's answer already loses and states.

## 3. What it freed

Nothing alone: `poppler-395-0-fuzzed` keeps a `FreeText` appearance this program can build only in
part, and the two XFA forms keep X1's refusal. The row now reports what the conversion does.

## What it costs

A third input to `decide`, carried as a pair beside the authorisations so the call stays one line.
