# 2580 — A signature policy in TS 119 172-3's ASN.1 syntax is read by a second parser and enforced as the first is

Status: accepted. Session 1873, slot 3 of batch 128, a clause round on the owner's answer A713.
Context: ISO 32000-2 §12.8.3.4.4 ("During validation conforming signature handlers should not enforce seed values
constraints, if present, but shall enforce signature policy constraints."), §12.8.1; ADRs 1076, 1085, 2560; ETSI TS
119 172-3 clauses 4.1.3.2, 4.2 to 4.29 and Annex A, ETSI TS 119 172-2 clauses 4.14.1 to 4.29.2, ETSI EN 319 122-1
clause 5.2.9.1, ETSI EN 319 102-1 clause 3.1 — every ETSI text cited by clause and never quoted (ADR 1085).

## Decision

**A second parser, behind the first one's vocabulary.** `pdf_signature::policy_rules::ts_119_172_3` reads TS 119 172-3
clause 4.2's `SignaturePolicy` — its Annex A module, `EXPLICIT TAGS`, every `BasicRule` alternative `[0]` to `[12]`,
commitment rules with their matching indicator, each rule's recommended scope — and judges a signature against it with
the `Finding`, `Verdict` and `Constraint` types ADR 2560 built, so `verdict::Verdict::under_policy` and `viewer-core`'s
notes take its findings unchanged. `policy_rules::Syntax::of` tells the two syntaxes apart by the first member's first
value (an object identifier in TR 102 272, the `Digest` sequence in TS 119 172-3); `enforce` dispatches on it, and an
`Enforced` result names the syntax it was read in, which the note's sentence says. A document read under the other
syntax's type is refused by name as `OtherSyntax(..)`, never half-read.

**The basic encoding rules are this syntax's own, so there is no distinguished-encoding pre-pass.** Clause 4.1.3.2
admits BER for some types; the first syntax's DER pre-pass would refuse documents the standard writes. The budgets come
before the work in another form: the same `MAX_POLICY_OCTETS` before anything is read, the same `MAX_ENTRIES` charged per
list entry before it is pushed, nesting held at `der::MAX_DEPTH` by `der::Reader` as each value is opened, and the
members the parser does not read — trust trees, which carry whole certificates — stepped over without being opened.

**Binding, a choice where the texts are silent.** EN 319 122-1 clause 5.2.9.1 leaves what `sigPolicyHash` is computed
over to the policy's own specification, and TS 119 172-3 names no input. A document is enforced when its octets digest to
the signed value (`policy::Binding::Matches`), or when the signed value is the document's own `Digest` — which clause
4.3.2 computes over the whole encoding of `PolicyComponents`, every rule included, and which the parser re-computes
(a disagreement is `DigestDisagrees`, refused). Signing that digest commits to the same rules, as TR 102 272 Annex B.11's
own hash does for the first syntax. Unbound, or naming another policy, nothing is enforced (ADR 2560's rule).

**The readings each judgement rests on.** Media types (clause 4.16) and cardinality (4.18) are judged against the one
data object a PDF signature covers, a PDF file (§12.8.1's byte range). Relative position (4.19): a PDF signature is
enveloped in EN 319 102-1 clause 3.1's sense; a stated combination must hold whole (TS 119 172-2 clause 4.19.1's cases c
and d), and ASiC is not evaluated, since the file cannot say whether it arrived in a container. `signerOnly` signing
certificate information (4.24) is read as the signer's certificate being carried; `fullPath` is not evaluated. The timestamp delay (4.25) runs from
the claimed signing time to the signature timestamp, TS 119 172-2 clause 4.25.1's measure in validation. Cryptographic
suites (4.29) are an allow-list per usage named (TS 119 172-2 clause 4.29.1), enforced for the `Signature` and
`SigningCert` usages with their minimum key and hash lengths. **The `SigningCert` usage URI is accepted in both
spellings**: TS 119 172-2 clause 4.29.2's schema prints `Usage/#SigningCert`, with a `/` its five sibling URIs lack, and
a producer following the siblings writes `Usage#SigningCert`; refusing either would refuse a policy for a typo. A rule
scoped (TS 119 172-2 clause 4.14.1) to generation or augmentation alone is named `OutsideValidation`, not enforced.

**Named and not evaluated, never counted as met**: trust conditions (the host's anchors, ADR 1076), formats and levels
(4.20, 4.27's levels), augmentation, the levels of assurance of 4.25 and 4.28, signer attribute sets, usages other than
the two above, and every `Other` extension.

## Measured

`getrusage` on the ignored probe `hostile_policy_documents_are_refused_within_their_budgets` (a document as long as the byte budget
admits, of nested indefinite-length BER values, and one a single entry past the entry budget, each parsed and refused a
thousand times): maxrss 12 380 KiB in 0.11 s, against 12 180 KiB for the
first syntax's hostile probe and 12 204 KiB for the floor test in the same binary — 176 KiB over the floor.

## Consequences

The §12.8.3.4.4 row's residue is TS 119 172-2's XML syntax, human-readable policies (prose no validator enforces), and
the copy fetched at the URL qualifier, bound but not enforced. A third syntax is a third parser behind the same types.
