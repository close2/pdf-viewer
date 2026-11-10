# 2782 — What is left of §12.8.3.4.4 is work, and each piece names what would answer it

Status: accepted. Session 1985. Context: ADRs 1035, 1076, 1085, 1728, 2624, 2725, 2775, 2781; ETSI TS 119 172-2
clauses 4.23.3 and 4.26; ETSI TR 102 272 clauses 6.9 and 6.10; RFC 6960 section 4.4.4; RFC 5755. The ETSI texts are
cited and paraphrased (ADR 1085).

## Why this round built one piece and read the other three

Round 1985 was given the residue ADR 2775 left — attribute certificates, a policy-supplied certification path,
the retention of expired revocation notices, trusted-list anchors — and asked to build the first two where they fit.
The mandated path was one construction over the existing search (ADR 2781). Attribute certificates are not one
construction: TR 102 272 clause 6.9's attribute trust condition and TS 119 172-2 clause 4.26's list both need an
RFC 5755 attribute-certificate reader, the attribute authority's signature verified and its own path built to the
condition's trust trees (or the signer's, where none are stated), the holder bound to the signer's certificate,
the certificate's validity at the instant, its revocation under `attrRevReq` or `AttributeRevocationTrust`, and the
type and value constraints — in three syntaxes, beside clause 6.10's `aaCertAlgorithmConstraints`. With the
machine's load past three hundred, a build took a quarter of an hour, and that is a round of its own.

## The premise checks

| piece | the premise given | what the texts say | so |
|---|---|---|---|
| retention of expired revocation notices (TS 119 172-2 clause 4.23.3.1 item 2) | a fact about the CA's practice that nothing in the file states | RFC 6960 section 4.4.4 lets an OCSP response carry an archive cutoff, whose distance from `producedAt` is the responder's retention interval, so an answering response can state it | **not** undeterminable from the file: it is decidable where an answering OCSP response for the signing certificate carries the extension, and not evaluated where none does. Work: this crate reads no OCSP single extension today |
| trusted-list anchors (TS 119 172-2 clause 4.22.2.1 b)) | a host's to fetch, through ADR 1076's port or the network rule's | a trusted list is a document published at a URL, as a policy is; ADR 1076's anchors are the host's own store and answer whom the reader believes, not which hierarchies the policy admits (ADR 2714) | the port is ADR 1728's: `viewer_host::policy::may_fetch_signature_policy`'s act class at A98's `Submissions` level, `ask` by default and `refuse` fetching nothing; then a TS 119 612 reader. Work, and nothing owner-blocked |
| attribute certificates (TR 102 272 clause 6.9, TS 119 172-2 clause 4.26) | effort | as above | work, taken by no round yet |

## Decision

None of the three is a limit and none needs the owner's word, so no Q821 is asked. The row's note names each with
what would answer it, and §12.8.3.4.4 stays `partial` (ADR 1035); no parent row moves (ADR 1599).
