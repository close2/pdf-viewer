# 1270 — nine published passwords open nine pages, and the CMYK group re-read

Batch forty-one, the corpus slot. ADRs 1377, 1378.

## Built
- `crates/pdf-model/tests/support/corpus_passwords.rs`: the one table of the pdf.js corpus's
  published passwords (seven from `test_manifest.json`, two from pdf.js's PR/browser test), read
  by `pdf-model`'s `corpus`, `oracle`, `raster_golden` and `render-raster`'s `corpus`.
- `pdfref`: `render_with_password` on `Reference` and `Cache`; the password is in the command line
  and so the cache key; `None` is the old invocation. `saslprep-r6.pdf`'s references get `SaSLprep`.

## What moved
- `LOCKED` 10 → 1, `NO_RENDER_NEEDS_A_PASSWORD` 10 → 1, `NOT_COMPARABLE` 16 → 7: the remaining
  password page is `encrypted-attachment.pdf` (none published); `PDFBOX-4352-0.pdf` stays.
- The nine: complete in the corpus gate; agree raster vs CPU; eight agree with the oracle's
  consensus; `issue3371.pdf` (a 4.17:1 reduced JBIG2 scan, text in the image) joins
  `AMBIGUOUS_IMAGE_REDUCTION` with an ink ladder. `raster_golden`: `issue21579.pdf` locked → drawn.
- The five pageless documents: all read again against the bytes; four are the file's own damage
  (§7.5.5 no catalogue ×2, §7.3.10 null child, §7.7.3.2 null `/Kids`), `Brotli-Prototype-FileA.pdf`
  a filter ISO 32000-2 does not define. No `pdf-syntax` debt.
- `CONTRADICTED_DEVICE_CMYK_CONVERSION`: not §10.4.2.5 against anyone — two §10.3.2 source
  assumptions; still every member's reason; figures re-taken, `transparent.pdf`'s corrected.

## Gates
Tier 1 scoped; `pdf-model` corpus, raster_golden, oracle, text_extraction; `render-raster` corpus;
`viewer-core` selection_census — all exit 0 behind the lock.

## Left
- Nine other private copies of the password table (five `pdf-transform` corpus tests, two `pdf-vfs`,
  `save_round_trip.rs`, `pdf-syntax/tests/encryption.rs`) could read the one table; not this
  round's files.
