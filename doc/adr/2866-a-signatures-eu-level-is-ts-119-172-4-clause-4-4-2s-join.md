# 2866 — A signature's EU level is TS 119 172-4 clause 4.4.2's join

Session 2025, a signatures round (`doc/todo/77` item 3). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 2834, 2835, 2864, 2865; ETSI TS 119 172-4 V1.1.1 clauses 4.2, 4.4.2 and Annex A;
ETSI TS 119 615 V1.3.1 clauses 4.4 and 4.5. Cited by clause and paraphrased (ADR 1085).

## Context

The owner asked for the level a signature has (`doc/questions/A800` item 3). ADR 2835 held every
report back from the word "qualified" until the level was TS 119 172-4 clause 4.4.2's own output.
ADRs 2864 and 2865 built the two determinations it starts from. What was left is the join:
REQ-4.4.2-02 (the certificate), -05 (the creation device) and -06 (the two with clause 4.2's
validation being `TOTAL-PASSED`).

## Decision

1. **`verdict::level::join` is REQ-4.4.2-02 to -06.** A `Level` holds the answer (qualified
   signature or seal; indeterminate where -02 b) or -05 b) stopped the process; not qualified
   where -06's validation condition failed), the requirement that decided it, -02 a)'s certificate
   determination, and whether the validation passed.
2. **`PROCESS_PASSED` means that value.** -02 and -05 name it, and TS 119 615 sets
   `PROCESS_PASSED_WITH_WARNING` as a separate value of the same output. A warning stops the
   process. This is a reading, written down as one.
3. **REQ-4.4.2-03 makes no check.** It stops the process "if any of the checks specified in
   REQ-4.4.2-03 fails", which names itself and states no check. NOTE 3 points at Article 32(1)
   (c) to (e), which REQ-4.5-01 b) and c) make content of the report. This silence is recorded as
   a silence.
4. **Clause 4.2's validation is run again under REQ-4.2-03's constraints.** Its anchors are the
   listed `CA/QC` service's identities (a)), which a `Standing::Listed` path already ended at. Its
   revocation check is `eitherCheck` with no unknown status accepted (c) i)), so
   `Acceptance::RevocationMustBeGood` applies whatever the host's own acceptance is. Its freshness
   is zero against the best signature time (c) ii)), which is the new
   `trust::PathInputs::revocation_issuance` that the path walk passes to
   `revocation::status_within`. That run's `Valid` is the `TOTAL-PASSED` -06 c) asks for. A host
   that accepts an unknown status can make a signature *valid*, but it cannot make one qualified.
5. **A locally valid path has no level** (ADR 2834). Its anchor is not REQ-4.2-03 a)'s set.
6. **Annex A's types come from the outputs**, and nothing is guessed. A qualified answer is type
   i) or iv). Where the certificate was determined under -02 a) and the validation passed, the type
   is ii) or v). Where only the validation passed, the type is "advanced signature or seal",
   because NOTE 5 to -05 reads Article 32(1)(h), Article 26's requirements, as met by
   `TOTAL-PASSED`. Types iii) and vi) cannot be told apart there, so that type carries no single
   identifier.
7. **The report words the level only from `Level`.** `viewer_core::notes::level_sentence` says
   "qualified" about a signature only for REQ-4.4.2-06's first branch, and it names the deciding
   requirement and Annex A's identifier. ADR 2835 is lifted exactly there.

## Consequences

REQ-4.5-01's other report contents (the scope text, the signer's data, the timing list) are not
built. TS 119 102-1's validation for Long Term Availability (REQ-4.2-01) is this tree's verdict,
with its document timestamp chain. `doc/todo/77`'s order carries the rest.
