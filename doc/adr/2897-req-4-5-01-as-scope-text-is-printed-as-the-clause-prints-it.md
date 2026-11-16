# 2897 — REQ-4.5-01 a)'s scope text is printed as the clause prints it

Session 2040, a signatures round (`doc/todo/77` item 3). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 1085, 2872; `doc/questions/Q855`, `doc/questions/A855`; ETSI TS 119 172-4 V1.1.1
clause 4.5.

## Context

REQ-4.5-01 a) requires the report to include a scope text the clause prints: a heading and one
sentence. ADR 1085 keeps ETSI's sentences out of this tree, because ETSI's notice permits no
reproduction without its written permission, so ADR 2872 said the scope in a paraphrase that said it
was one. Q855 asked whether to seek ETSI's permission, carry the text, or keep the paraphrase; the
owner chose to carry it (A855, option 2), reading the requirement to print the text as the
permission to print it.

## Decision

`Scope::heading` and `Scope::text` return the heading and the sentence as printed, the sentence
without the semicolon that closes item a) in the clause's list. They are the only ETSI words this tree
carries as written, and they are carried as strings in one place: no comment quotes them, so ADR
1085's rule for comments and the ledger is unchanged, and the conformance checker, which verifies
quotation against ISO 32000-2, is not asked to. `viewer_core::notes` prints both in quotation marks,
attributed to REQ-4.5-01 a), in place of the paraphrase. A test pins each string against the clause.

## Consequences

A reader of a listed signature sees the check named in the clause's words, which is what a) asks.
`doc/third-party-data.md` records the exception beside the ETSI entry, with the owner's answer.
