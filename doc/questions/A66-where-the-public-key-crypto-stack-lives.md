Status: complete
Given: 2026-09-14, in conversation — transcribed by the round
Owes: none

> Shared crate below both. Wait for a real trigger.

Reading: the shape is settled and the build waits. **The shape**: the `der`/`cms`/`x509`/`bigint`
seam is extracted into a crate below both `pdf-syntax` and `pdf-signature`, and `EnvelopedData`
and RSADP are added there — one implementation, one fuzz surface, one audit point, preserving
`doc/stack.md`'s "where they live: `crates/pdf-signature`, and nowhere else" against the one
thing that rule forbids, a second ASN.1/RSA surface on the crate that touches every byte of
every untrusted document. This is the second extraction of the same seam (ADR 1020 moved the
signature stack out of `pdf-model`), and the round that takes it carries two riders: the shared
crate takes the fuzz targets with it from the first commit — the `EnvelopedData` and the RSA
inputs are attacker-controlled even in a decrypting document — and the private key stays a host
input, per ADR 1134's shape and ADR 1039's trust-store rule.

**The trigger**: §7.6.5 is built when a document exists whose recipient list could match a
certificate the user holds, or a host asks to supply a private key — not before, and not for the
clause's own sake. Five documents in ~89 000 carry the handler and not one could decrypt under a
finished implementation, so the build buys coverage, not robustness; the calibrated refusal
(ADR 1134) is the honest state meanwhile.