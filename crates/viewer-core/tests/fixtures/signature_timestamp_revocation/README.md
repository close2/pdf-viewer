# Offline TSA revocation regression fixture

These test artifacts use ephemeral root, document-signer, and TSA RSA keys created by OpenSSL. The private keys were removed after generation; none are real credentials. `./generate.sh NEW_OUTPUT_DIRECTORY` recreates semantically equivalent material with new keys and timestamps, not byte-identical files. After regeneration, update the token time and verification instant in `src/notes/timestamp_revocation_spec_audit.rs` (relative to viewer-core), and refresh these hashes.

`cms-with-signature-timestamp.der` is an OpenSSL-signed detached **CAdES** CMS value over `signed-content.bin` (`openssl cms -sign ... -cades`). It has a genuine RFC 3161 signature-time-stamp unsigned attribute. The timestamp token's message imprint covers the actual outer CMS `SignerInfo.signature` octets. Appending this unsigned attribute leaves the original signed attributes and signature unchanged. OpenSSL independently verifies both signatures.

The TSA chain terminates at `root.der`, supplied to the test as a host trust anchor. `tsa-good-crl.der` is the correctly signed, unrevoked positive control, generated at `2026-11-16T16:12:23Z`. `tsa-revoked-crl.der` is also root-signed and lists TSA serial `1000` for `keyCompromise`, with revocation time `2026-11-16T16:12:23Z`. For both inputs the test evaluates TSA status at `2026-11-16T17:12:23Z` (Unix `1794849143`). Both CRL windows are `2026-11-16T16:12:23Z` through `2027-01-15T16:12:23Z`; thus `thisUpdate ≤ genTime ≤ verifyNow ≤ nextUpdate`, and the revoked entry is effective before the check. The token `genTime` is `2026-11-16T16:12:23Z` (Unix `1794845543`). The TSA and document-signer certificates remain valid through 2036.

The fixture is a valid signed CMS carrier with a valid signed timestamp token; it is not a PDF and does not test a PAdES byte-range digest. The regression seam is the actual `viewer-core::notes::signature_timestamps_instant` consumer. Its source-derived expected result is `None` when the TSA path is known revoked for `keyCompromise`, so its caller can retain the reader-clock fallback. The regression asserts the selector result; it does not separately execute the full fallback/report path. It makes no claim about required UI wording or the full PDF/report integration.

Independent checks performed on these exact files:

```sh
openssl ts -verify -in token.tsr -CAfile root.pem -untrusted tsa.pem -queryfile request.tsq
openssl verify -CAfile root.pem tsa.pem
openssl verify -CAfile root.pem doc.pem
openssl cms -verify -binary -inform DER -in cms-with-signature-timestamp.der -content signed-content.bin -CAfile root.pem -purpose any -out /dev/null
openssl crl -in tsa-good-crl.pem -noout -verify -CAfile root.pem -lastupdate -nextupdate
openssl crl -in tsa-revoked-crl.pem -noout -verify -CAfile root.pem -lastupdate -nextupdate
openssl crl -in tsa-revoked-crl.pem -noout -text
openssl cms -inform DER -in cms-with-signature-timestamp.der -cmsout -print
```

All commands succeeded. OpenSSL reported timestamp verification successful, both certificate chains `OK`, CMS verification successful, and both CRL signatures `verify OK`. The revoked CRL reports serial `1000` and reason `Key Compromise`. The outer CMS uses CAdES `signingCertificateV2`; the token's ESS certificate binding is present. The independent generator checks those profiles before deleting keys.

SHA-256 hashes for the exact generated files:

```text
88f8a035d99cda131e713110919aa19a8a5b2a07314c0f4af37eb05a59fbd360  root.der
7c95bead67936e41a6247a2451acbb4a16862c57016963a487a2694417b52291  tsa.der
2b82e1f1271a037fb7f2862bce36e902611b3667ce2b2df8f6cd2988222417a8  doc.der
ce641eecec0f80f980ba0dd5b8de513d916a9869c346062e6bada9c0598df0c1  tsa-good-crl.der
7eb7a9f76c96e79a9f61d1a548c50edd0a7f46b421f8f10029550a5026930a08  tsa-revoked-crl.der
91fdb60f7bfcb84be494f7bbc342ceaca5724833f721899402df7e356544f5e8  token.der
1b7e42208d354e2a7c4696b961894e0dc834374f722304b12d32039de3389599  cms-with-signature-timestamp.der
```

Parent verification repeated CMS, timestamp-imprint, chain, and both CRL signature checks on the committed DER artifacts at the fixed verification instant. The relocated generator also completed in a separate scratch directory, and its cleanup removed the ephemeral private keys. Regeneration output is not the committed fixture.
