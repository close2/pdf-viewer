# Signed field-lock control

These synthetic PDF 2.0 files differ by an actual signature value. Both contain two checkbox
fields (`name`, `other`) and a signature field whose indirect `/Lock` dictionary includes only
`name`. Each checkbox starts Off with explicit Off/On appearances; no font or default-appearance
assumptions are needed. The signature uses `ETSI.CAdES.detached`, SHA-256, a fresh test-only RSA
key, and the SigningCertificateV2 attribute produced by OpenSSL `cms -cades`.

`signature_field_lock_spec_audit.rs` checks unchanged cryptographic integrity, lock selection,
and the unsigned control. The viewer-core companion checks actual edit refusal under the
reader's On policy, an unlisted field, the unsigned form, and the explicit Off override. This
covers Include without `/P`.

`signed-lock-all.pdf`, `signed-lock-exclude.pdf` and `signed-lock-p.pdf` are the same document
signed with Table 236's `/Action /All`, `/Action /Exclude /Fields [(name)]`, and the Include
lock with `/P 1`; `signed_field_lock_shapes.rs` reads them. They were signed in one run with a
second test-only certificate, `test-only-cert-shapes.pem` (validity 2026-11-18 through
2036-11-15), so the first two files above keep their bytes.

The certificate is self-signed test material, not evidence of public trust. Its validity window
is 2026-11-16 through 2036-11-13. The generator deletes its temporary private key. Tests read the
committed files and do not invoke OpenSSL, qpdf, or a network service.

Regenerate into a scratch directory with Python 3, OpenSSL, and qpdf:

```sh
python3 crates/pdf-model/tests/fixtures/signed-field-lock/generate.py /tmp/pdf-lock-fixture
```

Fresh keys and signing time mean regeneration is semantically equivalent, not byte-identical.
The generator verifies detached CMS and checks PDF syntax. Parent review additionally extracted
`/Contents` and the declared byte ranges from the finished PDF using pikepdf, then verified that
extracted signature/content with OpenSSL. The committed signed file is 18,047 bytes with
`/ByteRange [0 462 16848 1199]`.

SHA-256:

- `signed-lock.pdf`: `c17424cc52db2a226c6397a6710223dacbe1f5059eaa6c180cd26dc2a7950f7c`
- `unsigned-lock.pdf`: `25a442e26c3442cf324de7d3b6874b73ac8bb139483683ae19c84a76f406044d`
- `signed-lock-all.pdf`: `10aff840664b244c7a02a0acf5c42671414db1a2c3452eab5fa84a77a0f72a50`
- `signed-lock-exclude.pdf`: `d8b25fc8c119cc22f0d676aa41201c92b3e42696647a0c4c2925ed467bbcfb7e`
- `signed-lock-p.pdf`: `b29204d1a03d22c86214bea017701dd768e2047121ea189c7d55246e2eba14db`
