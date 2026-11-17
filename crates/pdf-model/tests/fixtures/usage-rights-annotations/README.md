# Usage-rights annotation-save controls

Two exact-xref PDF 1.7 documents carry genuine detached CMS signatures, identified
by the catalog's `/Perms /UR3`. Their direct reference dictionaries use the standard
`/TransformMethod /UR`, `/V /2.2`, and `/P true`. Both grant `/Document /FullSave`
and `/Form /FillIn`; only `annotation-granted.pdf` grants `/Annots /Create`.

The test checks the original byte-range digest and cryptographic authenticity, adds
and saves the same Highlight annotation, and verifies that it is attached to the
saved page. The granted control retains the usage-rights reference. The withheld
case should remove that reference under the project's adopted withdrawal policy.
ISO 32000-2 §12.8.2.3 uses **should**, not shall, for withdrawal. This is a policy
regression witness, not a claim of mandatory conformance failure or public trust.
The certificate is self-signed test material; no feature entitlement is certified.

The generator uses a fresh test key, deletes it with its temporary directory, checks
PDF syntax with qpdf, and verifies CMS with OpenSSL. Regeneration is semantically
equivalent rather than byte-identical because keys and signing times change:

```sh
python3 crates/pdf-model/tests/fixtures/usage-rights-annotations/generate.py /tmp/ur-audit-fixtures
```

Parent verification independently extracted the final PDFs' `/Contents` and declared
byte ranges with pikepdf. OpenSSL verified both; changing signed content made both
fail verification. Tests use the committed bytes and require no external programs
or network. Original byte ranges are `[0, 579, 8773, 251]` for granted and
`[0, 572, 8766, 251]` for withheld.

SHA-256:

- `annotation-granted.pdf`: `4bac7c3ca37ae30f87f21f59d8299f13c6d46f436fe325a96c8ba27c655a17f3`
- `annotation-withheld.pdf`: `9e82caff8bf11f5035f98527a2d2af6cca6c13b3ba0e97dca1b0cd7c02d8cca9`
