# Q52 — A clause the committee is recorded as having set aside

Asked by session 940, which implemented it. **Provisional, not a blocker**: the rows are
implemented against the clause as published, and what is open is whether one of them should be.

## The question

ISO 19005-2 §6.6.6 and ISO 19005-4 §6.7.5 are the file-provenance subclauses. Both recommend that
each high-level action taken on a document be recorded in the `xmpMM:History` property of the
catalog's metadata stream, and both then state, as requirements, which fields each recorded action
carries:

| | required | recommended |
|---|---|---|
| ISO 19005-2 §6.6.6 | `action`, `parameters`, `when` | `softwareAgent`, `instanceID` |
| ISO 19005-4 §6.7.5 | `action`, `when` | `parameters`, `softwareAgent`, `instanceID` |

Both are implemented as of this session — `metadata/provenance-recorded-action-fields` and
`metadata/provenance-recorded-action-fields-four`. **Should part 2's `parameters` requirement be
among them?**

## Why it is a question at all

Three records, none of them the standard, and they do not agree.

- **veraPDF checks neither subclause.** Its `PDFA-2B.xml` and `PDFA-4.xml` profiles carry no rule
  numbered 6.6.6 or 6.7.5, which is why every witness in both corpus directories is named a pass —
  including three that omit one required field each.
- **The corpus says why.** `doc/veraPDF-corpus/TWG test files/TWG test suite A021-pdfa2-pass-*.pdf`
  each carry, in their outline, an ISO working group resolution recorded to the effect that
  requirements on the `xmpMM:History` property are application requirements and are therefore
  irrelevant to ISO 19005 validation. Four files, one per field, all named passes.
- **A 2016 record of the same agenda item says the opposite for part 2.** verapdf.org's
  *Update on the Resolution of Ambiguities* reports item A021 — which questioned the value and
  practicality of the `xmpMM:History` requirement in PDF/A-2 and PDF/A-3 — as accepted for the
  *next* part, while stating that the `parameters` field remains required for conformance with
  PDF/A-2 and PDF/A-3.

None is an approved erratum. `pdf-association/pdf-issues` publishes errata for ISO 19005-4:2020
alone (`crates/pdf-archive/src/errata.rs` says so and why), and no entry there touches either
subclause. So principle 5's answer is the one this session took, and it is the same answer the
`TN 0009` adjudication in `tests/corpus.rs` already took: contrary committee guidance that cannot
be read in an authoritative form does not displace a clause this project has read.

**ISO 19005-4's own text is the strongest evidence, and it is on the same side.** Part 4 was
published in 2020, after A021 was raised, and it keeps `action` and `when` as requirements while
demoting `parameters` to a recommendation. A committee that had concluded the whole property was
outside validation would not have restated two of its fields as requirements in the next part; a
committee that had accepted a narrower recommendation for the next part would have written exactly
this. That reading also makes the 2016 record and part 4's text consistent with each other, which
the blanket resolution does not.

## What it costs if the reading is wrong

Part 4's two fields cost nothing: a producer that writes a history entry at all writes `action` and
`when`, and the only witnesses that fail are one with no `action` and one that spells the time
field with a capital W.

**Part 2's third field is different.** Acrobat and several other producers write a `ResourceEvent`
as `action`, `instanceID`, `softwareAgent`, `when` and `changed` — with **no** `parameters` field.
The corpus's own PDF/A-1b witness `6.7 Metadata/6.7.2 Properties/veraPDF test suite
6-7-2-t03-fail-u.pdf` is exactly that shape. So enforcing §6.6.6 as published means reporting a
failure on a large class of PDF/A-2 files that every other validator passes, on a field the
committee is recorded as having been asked about twice.

That is the one place in this crate where following the clause has a visible cost to a real user
rather than to a fixture, which is why it is asked rather than decided.

## The three answers, and what each means

1. **Keep it.** The clause is the clause; the six corpus disagreements stay adjudicated as
   `SpecAgainstTheCorpus` and the reason is written where a reader of the verdict sees it. Nothing
   changes.
2. **Split part 2's row**, implementing `action` and `when` and leaving `parameters` its own
   `Check::Unchecked` row whose reason names the two contradictory records. Honest about the
   uncertainty, and it costs one row's worth of coverage on a requirement the standard states.
3. **Buy or obtain the record.** PDF Association `TechNote 0010` clarifies ISO 19005 parts 1 to 3
   and is the document that would settle this; it returns HTTP 403 at `pdfa.org` and has now done
   so for at least two sessions. If it says the resolution applies to the published part 2 rather
   than only to PDF/A-next, that is a reading of the standard this project could act on, and it
   would also settle the `TN 0009` adjudication that has been open since session 940.

The round that asked has no preference between 1 and 2 and would not take 3 without being told to.
