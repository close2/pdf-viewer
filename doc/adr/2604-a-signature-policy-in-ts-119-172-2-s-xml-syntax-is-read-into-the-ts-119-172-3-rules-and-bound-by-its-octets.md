# 2604 — A signature policy in TS 119 172-2's XML syntax is read into the TS 119 172-3 rules and bound by its octets

Status: accepted. Session 1885, slot 5 of batch 129, a clause round on the owner's answer A713; ADR 2605 is unused.
Context: ISO 32000-2 §12.8.3.4.4 ("During validation conforming signature handlers should not enforce seed values
constraints, if present, but shall enforce signature policy constraints."); ADRs 0186, 1020, 1076, 1085, 2560, 2580;
ETSI TS 119 172-2 clauses 4.1.2, 4.1.3, 4.2 to 4.6, 4.13 to 4.21, 4.24 to 4.29 and Annex B; ETSI TS 119 172-3 clause
4.29.2; RFC 3061; XML Signature 1.1 sections 6.2 and 6.4; RFC 9231 section 2 — every ETSI text cited by clause and never
quoted (ADR 1085).

## Decision

**A third parser, into the second one's types.** TS 119 172-2 clause 4 defines the building blocks TS 119 172-3 writes
again in ASN.1, so `pdf_signature::policy_rules::ts_119_172_2` reads clause 4.2's `SignaturePolicy` element into the
same `BasicRule`, `CommitmentRule` and `ScopedRule` values, and `ts_119_172_3::judge_rules` judges both. `Syntax::of`
says `Ts1191722` where the document opens with markup, which no DER encoding of either ASN.1 syntax can, and both ASN.1
parsers refuse such a document by name as `OtherSyntax`, as the XML parser refuses DER and a foreign root.

**The tokenizer is reused, the tree is this crate's.** `xmlparser` is the tokenizer ADR 0186 took for XMP and that XFDF
and XFA read with; `pdf-signature` cannot reach `pdf-model`'s XFA tree (ADR 1020 keeps the signature stack off the page
model), so the module folds the tokens into its own tree, resolving namespaces (clause 4.1.2's URI with and without its
final `#`, since the schema and the clause's list print both), expanding only XML's predefined and character
references, and refusing a document type declaration by name, so no declared entity is ever expanded.

**A stack decision with no new crate.** `pdf-signature` gains one dependency, `xmlparser`, by the workspace's own
`xmlparser.workspace = true` — the version `pdf-model` and `pdf-formcalc` already take, so `Cargo.lock` and
`fuzz/Cargo.lock` each gain one edge and no package, and `cargo deny check` passes on advisories, bans, licences and
sources. A tokenizer is what a reader of hostile XML should not write again (ADR 0186 chose this one), and a tree crate
over it (`roxmltree`, not in the tree) would allocate every element before this module could charge its budget for it,
which is the order the budgets exist to reverse.

**The same budgets, before the work**: `MAX_POLICY_OCTETS` before a byte is tokenized, `der::MAX_DEPTH` on element
nesting — the deepest fixed path of clause 4, a time-stamp trust anchor's service type URI inside a commitment rule, nests fifteen elements under the bound's sixteen, so only `AnyType` content meets it — and
`MAX_ENTRIES` charged for every element, every namespace declaration and every `xs:list` token before it is kept.

**Identifiers, a reading where clause 4 says only `xs:anyURI`.** The policy identifier is compared as RFC 3061's
`urn:oid:` URN (clause 4.6 says an object identifier is written so). A commitment identifier that is such a URN is that
object identifier; another URI never equals a CAdES commitment type, which is an object identifier. An algorithm is
placed by the URN or by the fifteen XML Signature 1.1 / RFC 9231 URIs of the digest, RSA and ECDSA algorithms this
crate verifies, each beside `const_oid`'s identifier and checked against RFCs 3279, 4055, 5754 and 5758; an entry left
a URI may name the used algorithm, so a used algorithm no placed entry lists is `NotEvaluated` where one exists, and the
URI is named. Clause 4.29.1 makes an absent `Usages` every usage, written as the empty list (TS 119 172-3's list is never
empty), and names an `ExpirationDate` without the instant it is compared with, so each is named and not evaluated. A
`RecommendedScope` list including `Validation` is used; one naming other scopes alone is `OutsideValidation`.

**Binding by the octets alone.** Clause 4.3's `Digest` is over the canonicalised `PolicyComponents` under the
canonicalisation method it names. This program does not canonicalise XML, so the document is enforced only where its
octets digest to the signed value (`Binding::Matches`, decisive in that direction); a signer who signed the stated
`DigestValue` instead gets `NotEnforced::CanonicalDigestNotComputed`, said in the note, never an enforcement resting on a
value nobody checked. An `xs:dateTime` without a time zone is refused: it names no instant.

## Measured

`getrusage` on the ignored probe `ts_119_172_2::tests::hostile_policy_documents_are_refused_within_their_budgets` (the
byte budget filled with nesting, with sibling elements and with namespace declarations, each refused a thousand times):
maxrss 6 572 KiB in 2.50 s, against 5 652 KiB for TS 119 172-3's hostile probe and 5 328 KiB for a pure unit test in the
same binary.

## Consequences

The §12.8.3.4.4 row's residue is human-readable policies, the copy fetched at the URL qualifier, and an XML policy bound
only through its own `Digest`, which waits on an XML canonicalisation this tree does not have.

The fuzz target `policy_document` reaches all three readers from one input: the reader that accepts a document is the
one `Syntax::of` names, and the findings judged from it are a function of the bytes (`doc/verify.md`).
