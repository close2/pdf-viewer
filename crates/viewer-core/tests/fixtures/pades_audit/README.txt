Test-only signed PDF fixtures, generated offline with OpenSSL cms -sign -cades.
The certificate identifies only a synthetic test signer. No private key is needed or included.
pades-tampered.pdf changes one MediaBox digit outside the signed Contents gap;
file length, xref, ByteRange and CMS are identical to pades-valid.pdf.
Tests use a fixed verification time and explicitly accept unknown revocation.
See doc/reviews/2038-specification-audit/shards/signatures_current/fixture-provenance.json
for hashes, ByteRange, certificate validity, independent verification and limits.
