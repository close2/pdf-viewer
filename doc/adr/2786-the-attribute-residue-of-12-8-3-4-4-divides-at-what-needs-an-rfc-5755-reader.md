# 2786 — The attribute residue of §12.8.3.4.4 divides at what needs an RFC 5755 reader

Status: accepted. Session 1987. Context: ADRs 1085, 2714, 2720, 2725, 2782, 2785; ETSI TR 102 272 clause 6.9; ETSI
TS 119 172-2 clause 4.26; ETSI TS 119 172-3 clause 4.26; RFC 5755 sections 4 and 5. The ETSI texts are cited and
paraphrased (ADR 1085).

## Why this round built retention and no attribute piece

Round 1987 was given retention first and then the largest coherent part of attribute certificates that fit. Retention
was built (ADR 2785). With the machine's load between 270 and 440, the first `cargo clippy` of `pdf-signature` waited
behind the worktree's warm-up build for over half an hour, so a second construction with its own compile cycles did
not fit the three hours, and none was started: a reader with nothing judged over it would be a placeholder.

## How the residue divides, read for the next round

ADR 2782 states the attribute residue as one construction headed by an RFC 5755 reader. Read against the clauses, it
is two, and the first needs no attribute certificate at all:

| piece | what decides it | needs |
|---|---|---|
| presence and kind | TR 102 272 clause 6.9's `attributeMandated` and `howCertAttribute` (claimed, certified or either); TS 119 172-2 and -3 clause 4.26's level of assurance (claimed, certified, signed assertion, any) and the attribute identifiers that must be present | the signer-attributes attribute's claimed and certified members read out of the signed attributes (the v2 form `cms.rs` already checks structurally, and the v1 form); the attribute type constraints are decidable on claimed attributes, which are plain `Attribute`s |
| certified attributes | RFC 5755 section 5's seven conditions: the holder bound to the signer's certificate, the issuer's signature and its path (to the condition's trust trees, or the signer's where none are stated), the issuer's profile, validity at the instant, targeting, unsupported critical extensions; then `attrRevReq` or `AttributeRevocationTrust`, and clause 6.10's `aaCertAlgorithmConstraints` | an RFC 5755 reader, `certification_path`'s search run from the attribute authority, and `revocation_checks` over that path, each already shaped for a second condition (ADR 2720) |

Where a policy mandates a certified attribute and the signature carries none, the first piece already answers
**broken**; where it carries one, the second is what answers. So the next round takes the first piece in all three
syntaxes, and the round after it the RFC 5755 reader with the authority's path. Nothing here is owner-blocked, so no
question is asked; §12.8.3.4.4 stays `partial` (ADR 1035) and no parent row moves (ADR 1599).
