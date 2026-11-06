# Q800 — Does the 2026-10-28 "ledger to final states" direction lift A66's trigger for the public-key handler?

Source: the orchestrating session, 2026-11-06, choosing the ledger lane's next rows.

## The conflict

- **A66** (2026-09-16): "Shared crate below both. Wait for a real trigger." §7.6.5's public-key
  security handler is to be built when a document exists whose recipient list could match a
  certificate the user holds, or when a host asks to supply a private key. A168 later named four
  pdfbox witnesses (`AESkeylength128/256.pdf`, `AES128/256ExposedMeta.pdf`) whose keystores are
  published beside them, as what the build is tested against "when A66's trigger fires, not that
  trigger".
- **The owner's direction of 2026-10-28** (memory `owner-words-2026-10-27`): bring the ledger to
  final states, and treat effort as no blocker; build every row whose only obstacle is effort.
- Four ledger rows sit at `reported` on A66 alone: 7.6.5, 7.6.5.1, 7.6.5.2 and 7.6.5.3. Row 7.6.6
  is `partial` on the same handler (Table 27).

## The options put

1. **Build it now** (recommended). Extract the `der`/`cms`/`x509`/`bigint` seam into a shared crate
   below `pdf-syntax` and `pdf-signature`, as A66 settled, and add `EnvelopedData` and RSADP there,
   with fuzz targets from the first commit. The private key stays a host input (ADRs 1134, 1039).
   Test end to end against A168's four witnesses. This moves five rows toward their final state.
2. Keep A66's trigger. The rows stay `reported`, with ADR 1134's calibrated refusal.

## What the tree does meanwhile

Keeps A66's trigger. No round builds the handler until this is answered.
