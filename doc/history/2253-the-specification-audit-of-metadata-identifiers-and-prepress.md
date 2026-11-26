# Round 2253 — the twentieth specification audit: §14.1 to §14.5 and §14.10 to §14.13

An audit round in the `gwg` worktree, in the shape of review 2038 and audits 2128 to 2248: every
`shall` and `should` of §14.1 to §14.5 and §14.10 to §14.13 (procedure sets, metadata, file
identifiers, page-piece dictionaries, Web Capture, prepress support, document parts, associated
files) against `pdf-model`, `pdf-syntax`'s writers, the transform verbs and the archive converter.
The clause changes no pixel, so the lens was every writer of `/Info`, XMP, `/ID` and `/PieceInfo`.
It fixes nothing.

## What it found

- `doc/reviews/2253-specification-audit-14-1-14-5-14-10-14-13.md` and its directory: 4 B, 0 T, 2 D.
- B01 a §7.9.4 date with no zone goes into the packet with no zone, against #251's +0'00 (update,
  merge, converter). B02 `optimize`, `redact`, `split` and the converter drop a direct `/Info`
  (#106), so a catalog `/PieceInfo` crosses without its required `/ModDate`. B03 redaction carries a
  page's `/PieceInfo` and `/LastModified`, so §14.5's comparison calls the private data current.
  B04 an update's new packet is encrypted under `/EncryptMetadata false`. D01 a warning quotes
  Table 15's struck "shall be an indirect reference"; D02 the 14.3.4 note and two comments on the zone.
- Four failing tests with controls: `pdf-transform/tests/metadata_writers_14_3_14_5_spec_audit.rs`;
  listed in the audit's `expected-failures.json` and appended to `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged (§14.1, §14.3.x, §14.4, §14.5, §14.11.x but four, §14.12.x but
14.12.4.2, §14.13.x `implemented`; §14.2, §14.10.x, 14.11.4, 14.11.6.1, 14.11.6.3, 14.11.7,
14.12.4.2 `inapplicable`). The findings contradict 14.3.2, 14.3.3, 14.3.4, 14.5 and 7.6.4.2.

**Gates.** Over `-p pdf-syntax -p pdf-transform -p pdf-archive -p viewer-qt`: fmt 0; clippy
`-D warnings --all-targets` 101 once on five lints in the new test, fixed, then 0. `pdf-sandbox` 0.
nextest `--lib --bins --tests` (`xvfb-run -a`): 2288 run, 4 failed — exactly the manifest's
(`audit-diff.py`: "failed 4, expected 4"). Rustdoc of `pdf-transform` 0. `cargo test -p
conformance --no-fail-fast` 0. Fuzz not owed (no public type changed).

## Habit

Clause 14 changes no pixel, so its readers were already audited and its writers were not: the
defects were all in what a verb carries without reading (a page's private data, a direct `/Info`)
and in an exemption decided against the file being read rather than the file being written.
