# 992 — The configuration format, and the first departure

Date: 2026-09-12. ADRs 1012 (the format and the caller-reads seam) and 1018 (departures).

RFC 0007's configuration, built now that the owner answered its questions (`A54`–`A60`). A refusal
is a question the converter asks; this round lets the answer be given in advance, in a TOML file the
*caller* reads and hands to `apply` as data — so RFC 0002 section 5's purity holds. Two answers reach
a real conversion: a `discard` at one of section 3's seven loss sites becomes the `Authorisations`
the flags build, and a departure (section 4.7). Everything else — `preserve`, `derive`, `supply`,
`discard` at an unbuilt site — is recognised, validated and enumerated but not applied, so the site
stays refused with the sentence it already carries. Installing a configuration changes no pipeline
unless it authorises a built loss or names a departure (ADR 0954).

The first departure is the owner's XML case (`A60`): a PDF/A-2 conversion that accepts XML and only
XML attachments — *only*, because the predicate is asked of every embedded file. By `A59` the output
omits the PDF/A identification and does not claim to be PDF/A-2; `--claim-conformance` is the second
switch that keeps the claim a validator fails anyway. Stage three's net is narrowed, never switched
off. Demonstrated on a fixture built from the clauses and revalidated three ways, and run end to end
through `quorra-transform` with `doc/profiles/factur-x.toml`.

`--remedy-sites --to <target>` enumerates every refusal site from the same table the converter
decides from (78 for 2b, 68 for 4f). The four shipped profiles are re-expressed in the real format
and held to it by a test that loads each for all six targets, so a profile cannot drift from the
format again.

Left for the next converter round: `derive` and the external-tool executor (`A54`, `A56`), `supply`,
`preserve` by attachment and by appended pages (`A58`, session 994's amendment), the shape-aware
remedies the shape qualifier is built for, and a departure catalogue beyond the embedded-file case.

## Files touched

- `crates/pdf-transform/src/archive/toml.rs` (new) — the TOML subset reader.
- `crates/pdf-transform/src/archive/config.rs` (new) — `Configuration`, `Kind`, `Departure`,
  `Coverage`, `Site`, `Unbuilt`, `ConfigError`, `sites`.
- `crates/pdf-transform/src/archive/mod.rs` — module decls and re-exports; `ArchivePlan` gains
  `departures` and `claim_conformance`; `departures_over`, `stands_as_departed`, and the departure
  threading through `run`/`decide_every_failure`/`apply_the_decisions`/`rewrites_wanted`.
- `crates/pdf-transform/src/archive/prepare.rs` — `omit_identification` and `departure_history`
  threaded into `Prepared::of`/`Recording`/`the_packet`/`prepare_metadata`; `DEPARTED_ACTION`.
- `crates/pdf-transform/src/archive/decision.rs` — `IDENTIFICATION_CLAIM`.
- `crates/pdf-transform/src/archive/report.rs` — `Departed`, `DepartureOutcome`, `departure_history`,
  the `departures` field on `Conversion`, and its rendering and JSON.
- `crates/pdf-transform/src/bin/quorra-transform.rs` — `--config`, `--remedy-sites`,
  `--claim-conformance`, `--depart-from-the-standard`; `read_config`, `print_remedy_sites`.
- `crates/pdf-transform/tests/archive.rs` — the departure demonstration; new ArchivePlan fields.
- `crates/pdf-transform/tests/archive_corpus.rs` — new ArchivePlan fields.
- `crates/pdf-transform/tests/profiles.rs` (new) — the profile-load test.
- `doc/profiles/keep-everything.toml` — the one `on-failure` chain became a single alternative.
- `doc/profiles/factur-x.toml` (new) — the shipped departure example.
- `doc/pdf-a-mitigations.md` — sections 14 and 15's findings marked built where they are.
- `doc/pdf-a-conversion-limits.md` — section 3.1's departure marked built.
- `doc/adr/1012`, `doc/adr/1018`, and this file.

## What the RFC should now say (for session 994, which owns `doc/rfc/0007`)

- Section 3: the format is built. Its reader is `crates/pdf-transform/src/archive/{toml,config}.rs`;
  it is a documented TOML subset, not full TOML. `default` (only `stop`), the target qualifier and
  the shape qualifier are all in the grammar. `[tool.…]` is carried past the reader unread.
- Section 4.6: the target qualifier is applied. The shape qualifier (section 5b.2 / catalogue 14.1)
  is parsed and validated but inert — no shape-aware remedy reads it yet.
- Section 4.7: the departure is built for `embedded-files/embedded-file-is-itself-pdfa` (and its
  plain-profile sibling), narrowed by media type and relationship, with the two switches `A59`
  requires and the `--depart-from-the-standard` call-site gate. `--claim-conformance` is the CLI's
  spelling of the second switch.
- Section 5: `on-failure` is a single alternative (`A57`), enforced as a named error on a list.
- Section 7's five questions are answered (`A54`–`A60`); the RFC can move from *proposed* toward
  ratified, with `derive`, the executor, `supply` and appended-page `preserve` marked as the next
  converter round's rather than open.
