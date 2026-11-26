# 2254 — A redaction drops the private data its removal made stale

Lane `gwg`, the fix round of audit 2253 (§14.3–§14.5, §7.6.4.2). ADRs 3717 to 3720. The four
`*_spec_audit` tests of `metadata_writers_14_3_14_5_spec_audit.rs` pass unchanged; their entries
leave `expected-failures-current.json`, and 2253's six findings are `fixed`.

- **B03** (ADR 3717). `redact` writes a redacted page without `/PieceInfo`, `/LastModified` and
  `/Thumb`, an edited form without `/PieceInfo` and `/LastModified`, and — where any page was
  redacted — the catalog without `/PieceInfo` (replaced, so nothing only it reached is copied).
  §14.5's equality comparison would otherwise call the producer's private data current, and
  §12.5.6.23's "remove all traces" with A834 asks it anyway. No clock, so `/ModDate` crosses as
  stated. Each drop is a `Departure` naming the page (or none) and the entries.
- **B04** (ADR 3718). `write::written_metadata` finds the document-level packet in the catalog
  the update writes; `encrypt_for_update` exempts that one under `/EncryptMetadata false`. The
  reader still asks its own catalog, lazily (`Metadata::AsRead`).
- **B02, D01** (ADR 3719). `pdf_transform::information` reads `/Info` direct or indirect for
  `optimize`, `redact`, `split`, `archive/rewrite.rs`, `archive/recovered.rs` and `update`; a
  direct dictionary is written as a new object, its references carried by each writer's own
  copy. The converter's kept `/ModDate` beside a `/PieceInfo` is now written for a direct
  `/Info`. `SetInformation`'s warning quoting #106's struck words is gone.
- **B01, D02** (ADR 3720). `xmp::spelled_date` writes an absent zone as `Z` (#251); the viewer's
  `Display` still shows the date as written. `date.rs`'s and `xmp.rs`'s comments restated.
- New: `tests/stale_private_data.rs` (four tests: the drops and their report, an untouched page,
  `split` and `update` over a direct `/Info`). Ledger notes, tests and code lists of 14.3.2,
  14.3.3, 14.3.4, 14.5 and 7.6.4.2 restated; all stay `implemented`.
- Left, in order: other places removed text may survive a redaction (a structure element's
  `/ActualText` and `/Alt`, a page's `/Metadata` packet), unread here; a placement copy of a form
  is reported once per copy.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform -p pdf-archive -p viewer-qt`: fmt 0,
clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0, nextest `--lib --bins --tests`
(`xvfb-run -a`) 5337 of 5340, the three failures the manifest's §10.5 entries (`audit-diff.py`:
"failed 3, expected 3"); rustdoc of `pdf-syntax`, `pdf-model`, `pdf-transform` 0; no public type
changed, so no fuzz check; `cargo test -p conformance` 0. Census with the pack 0, every summary
line 2252's; veraPDF 1.30.3 over its 5523 conforming outputs 0: 5471 compliant, 40 not, 12
unfinished, as 2252's.

**Trap or habit.** A `default` added to a reader crate (`unwrap_or(0)` for #251's zone) is a site
`repair_census` counts: mark it `// not a repair:` with the clause's reason before the conformance
run, not after.
