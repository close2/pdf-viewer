# 3171 — §12.8.3.4.4 is `implemented`, and §12.8.3.4, §12.8.3 and §12.8 follow it

Session 2117. Status: accepted. Rows: §12.8.3.4.4 (`partial` to `implemented`); its aggregates
§12.8.3.4, §12.8.3 and §12.8 (`partial` to `implemented`, ADR 1599). Context: ADRs 1035, 1599,
2711, 2725, 2775, 2782, 3158, 3161, 3162, 3169, 3170.

## The residue, read from every table the note cites

| table | piece named work | now |
|---|---|---|
| ADR 2725 | attribute certificates, `aaCertAlgorithmConstraints` | built (ADRs 2797, 2890, 2891) |
| ADR 2725 | trusted-list anchors | built (ADRs 2882, 2886, 2932, 2936, 2937) |
| ADR 2725 | freshness and retention of revocation material | built (ADRs 2775, 2785) |
| ADR 2725 | the caution period and signature time-stamp delay | built (ADR 2775) |
| ADR 2725 | a mandated certification path | built (ADR 2781) |
| ADR 2725 | a distinguished-name value outside ASCII | built (ADR 3170); ADR 2775 had dropped it |
| ADR 2725, ADR 3162 | a C caller's fetched copy | built (ADR 3169) |
| ADR 2782 | retention, anchors, attribute certificates | as above |
| ADR 3162 | the six X.520 rules and `id-at-role` | built (ADR 3161) |

Everything else either table lists is a limit, the reader's choice or outside validation, each with
its reason in ADRs 2711, 2725, 3161 and 3162: human-readable policies and a policy in a syntax no
specification this program holds defines, a revocation requirement of `other`, a signed assertion EN
319 122-1 does not define, a TS 119 172 value constraint, a clear-text password against an encrypted
one, a value outside its type's syntax, and a trusted list the reader declines to fetch.

## Decision

Nothing is owed, so §12.8.3.4.4 is `implemented` (ADR 1035), and the three headings above it, whose
other leaves are all `implemented`, follow (ADR 1599). `doc/todo/65` takes the row out of bucket 2
and the three out of the aggregate list. No question is asked.
