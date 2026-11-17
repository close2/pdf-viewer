# Public current-time CRL report fixtures

These offline test PDFs carry real detached CAdES signatures and issuer-signed DSS CRLs. Both have exact xrefs, the same signer/root, no trusted timestamp, and fixed host time **1794848986** (2026-11-16 17:09:46 UTC). The leaf is valid from 2026-11-16 16:39:46 UTC through 2036-11-13 16:39:46 UTC.

`current.pdf` has CRL thisUpdate 17:08:46 UTC; `future.pdf` has thisUpdate 17:10:46 UTC on the same date. Both nextUpdate values are 2027-01-15 17:09:46 UTC. The only relevant semantic difference is whether the CRL window includes the supplied clock; signatures, CRL numbers and digest bytes necessarily differ too.

ISO 32000-2 §12.8.3.4.7 requires a current-time CRL window to include the current UTC instant. Historical validation under §12.8.3.4.8 has a different rule. The public test uses `Command::Report`, a supplied root and fixed clock, and `RevocationMustBeGood`. It checks actual PDF ByteRange integrity, CMS verification and signing-certificate binding before the revocation verdict. This is not a claim that a generic mode-less status API should infer current-versus-historical context.

The parent independently extracted both signed ByteRanges and verified detached CMS signatures with OpenSSL, verified the signer path at the fixed host time and the CRL signatures, and checked PDF syntax with qpdf. qpdf is only a syntax check.

To regenerate semantically equivalent inputs, run `sh generate.sh NEW_OUTPUT_DIRECTORY`. The directory must not exist. The script creates ephemeral keys in a temporary directory and removes them on success or failure. It checks its outputs before publishing public PDFs/certificates/CRLs/time files. Regeneration changes keys, signatures, clock and hashes; the committed pair remains fixed. No private keys or CSRs are retained.

SHA-256 of the committed fixtures:

```text
13dfeb717fbe3d4b27b0159d22907aeaf606d29794fde6128a8e3a6df0652b27  root.der
7f4f578390fdde0105e001f3610a64b4cce0288a2a08a19ffad63c85dc129fe5  current-crl.der
ee017bf79e2cd9a1ddb7088a88fbfe1dd7e37048ee352ae1acc1fdd759bc0474  future-crl.der
4a737484c36baff3c1794442c9aea44cd217de2bf4136dea8b5cefbd83e57dbd  current.pdf
2e6a484326f759a51b0d9c5b4615233d0338ac0297b1eaece250835480a50c86  future.pdf
```
