# Historical OCSP `producedAt` fixture

This offline fixture targets ISO 32000-2:2020 + EC3 §12.8.3.4.8, the defining paragraph at physical p.599 / printed p.584. EC3 does not change the condition that an OCSP response's signed `producedAt` must follow the trusted past UTC validation time. The separate requirement that the response was captured before signer-certificate expiry is not inferred from `producedAt`.

`early.pdf` and `late.pdf` contain the same signed PDF revision and real CAdES signature-time-stamp token. Each has an exact incremental xref update carrying one OCSP response and the same TSA good CRL. The document signer and TSA use separate supplied trust anchors so the TSA CRL cannot supply fallback GOOD status for the document signer.

The fixed trusted `genTime` is **2026-11-16 17:09:09 UTC**. Both OCSP answers are issuer-signed `GOOD`, with `thisUpdate` before and `nextUpdate` after that instant. The early window is 16:59:06–18:09:06 UTC; the late window is 16:59:11–18:09:11 UTC. They differ by five seconds, but both bracket T. `early` has `producedAt` **17:09:06 UTC**; `late` has `producedAt` **17:09:11 UTC**. The pair swaps the OCSP payload while holding the signature, timestamp, signer/TSA chains, host time and response status fixed; each OCSP window independently brackets T. Certificate validity is 2026-11-16 17:09:06 UTC through 2036-11-13 17:09:06 UTC; fixture-generation records place the DSS creation before signer expiry, but the appended DSS revision has no trusted timestamp proving capture time for arbitrary documents.

Independent generation checks succeeded: OpenSSL verified the RFC 3161 token, CMS signature bytes, both certificate paths at the trusted time, each OCSP signature/CertID/status, and the TSA CRL signature. qpdf reported no syntax or stream encoding errors. These checks do not certify every aspect of PDF semantic conformance. The hashes below identify the committed PDF and DER files.

Regenerate into a fresh, nonexistent output directory:

```sh
sh generate.sh /tmp/ocsp-producedat-fixture
```

The script refuses an existing output path, traps and removes its temporary directory (including ephemeral private keys), and exports only public test artifacts. CMS `signingTime` and response/certificate serials vary on regeneration, so regenerated outputs are semantically equivalent, not byte-identical.

The public integration test is `../../historical_ocsp_report_spec_audit.rs`. Parent execution: late control passes; early response incorrectly supports local validity (B47). Parent independently reverified the actual PDF ByteRanges, CMS, token imprint/signature, OCSP signatures and CRL with OpenSSL. qpdf syntax checks pass. These are bounded offline controls, not full signature conformance certificates.

SHA-256:

```text
3fb7df2a9ac22a8eaa1aef791349716cbf8e85c18f1cc23d20a6265d627ab232  doc-ca.der
af3d721023f6b393db32bbb756fbd67bc130e246f1f40d30fd2ae25ad3228058  early.ocsp.der
12936c2ab07c9631cafd10a4a32917a91272f62c877b19fdb9aca36ff3f829ef  early.pdf
deebef68126e314fcaab669c9b62fce7d995e7c070f2491c3f26180bfb4f662c  late.ocsp.der
71f75ba863722542ac0bcd6cf11f313763ae0c4c9caf84055e9cfc72293de251  late.pdf
ead72c0b4eff77a107dec315fc745be4893f0afc05912520df97fee032ac08b2  root.der
e4aadbc8a327a14178b36f232ccfdbb861bdd01547d6bb0d1e5e382e1fe164d7  tsa-good-crl.der
```
