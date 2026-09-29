Status: complete
Given: 2026-09-16, in conversation — transcribed by the round
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

**2026-09-28 — a correction appended by the acting round, because `doc/questions/A168`'s `Owes:`
line asks for it; the owner's words above are unchanged and this file is amended only by this
paragraph.** The reading's *not one could decrypt under a finished implementation* is false as stated for four
of the five: `PDFBOX-4421-0.pdf` to `-3.pdf` are encrypted to `CN=testnutzer`, serial `5F609C62`,
and the PDFBox corpus publishes that certificate's keystore, `PDFBOX-4421-keystore.pfx`, in
`doc/corpora/pdfbox` beside `AESkeylength128.pdf` and `AESkeylength256.pdf` (byte-identical to
`-0` and `-1`); `AES128ExposedMeta.pdf` and `AES256ExposedMeta.pdf` there are encrypted to
`CN=test`, serial `60FFD550`, of `PDFBOX-5249.p12`. Those four `doc/corpora/pdfbox` documents are
decryptable with the published test keystores shipped beside them, and they are the end-to-end
witness set the build is tested against when a real trigger fires
(`crates/pdf-syntax/tests/public_key_witnesses.rs`). They are not the trigger: no document is known
to be encrypted to a key a real reader holds, both arms above stand as written, and the build stays
untriggered (A168).
