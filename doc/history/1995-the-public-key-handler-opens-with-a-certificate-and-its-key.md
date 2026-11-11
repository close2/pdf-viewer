# 1995 — The public-key handler opens with a certificate and its key

Session 1995, a single ledger round (§7.6.5 to §7.6.5.3, §7.6.6), the second ledger lane. ADRs 2801, 2802.

**The slice, and why.** The largest slice of A800's item 1 that ends in something that runs. It
extracts the shared crate, adds `EnvelopedData` and RSADP with their fuzz targets, and builds the
handler behind `Document::open_with_recipient`, tested end to end. The host input is the next slice.

**The premise.** Held. A800 lifts A66's trigger, and nothing named `EnvelopedData` or RSA
decryption. One refinement: `cms` stays in `pdf-signature`, because it calls the four signature
families. Moving it would move the verification stack below the object reader (ADR 2801).

**Built.** `crates/pdf-pki`:
- `der`, `bigint` and `x509` moved out of `pdf-signature`, which re-exports them. `x509/keys.rs`
  holds the key types.
- New: `enveloped` (RFC 5652 §6, AES-CBC) and `rsa` (PKCS#8 or PKCS#1 key, RSADP, RFC 8017 §7.2.2,
  constant-time exponent).

`pdf-syntax`:
- `crypt/public_key.rs`: s3, s4 and s5, the recipient scan, Table 24, and §7.6.5.3's SHA-1 or
  SHA-256 key with step c).
- `RecipientKey`, and two new errors: `RecipientRequired` and `RecipientKey`.

Fuzz targets `enveloped` and `rsa_private_key`, with their `verify.md` lines and `seeds.sh` arms.
**Tests.** `public_key_witnesses.rs` opens A168's four `PDFBox` witnesses and checks the page text
(read through `ToUnicode`) that `PDFBox`'s test states. It also opens script-written s3 and s4
files (`gen_s34.py`), refuses a mismatched key, and asks for a certificate without one. `pdf-pki` has
unit tests built from RFC 8017 and RFC 5652. **Plant**: dropping step c)'s four `0xFF` fails both
`ExposedMeta` witnesses; reversed, all pass.

**Gates.** fmt `--check`, clippy `-D warnings --all-targets`: exit 0. nextest (pdf-pki, pdf-signature, pdf-syntax, pdf-model): 3211 of 3211, 28 skipped. fuzz `cargo check --bins`: exit 0. conformance after `git add -N`: exit 0. `state.sh docs`: exit 0 (5607 s). `viewer-ui --test notices`: 7 of 7. Tier 2: `corpus` 101, 974 documents and 62 incomplete (HEAD's); `raster_golden` 101, 16 moved (HEAD's font documents). `launch_path` was not run. The key fixtures are recorded in `third-party-data.md`, and `NOTICE` carries PDFBox's.

**Left of the rows.**
- §7.6.5.1: `partial`, waiting on the host input.
- §7.6.5.2: `implemented`.
- §7.6.5.3: `partial`, because the four deprecated envelope ciphers are refused by name.
- §7.6.6: `partial`, because Table 27's per-stream recipient list is refused by name.
- §7.6.5 and §7.6 follow their rows.

All of these are effort only, in ADR 2801's order.
