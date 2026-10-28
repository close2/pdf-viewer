# 2538 — An operator's configuration is read at all six targets before any document, and a base is a shipped profile one level deep

Status: implemented. Session 1852. Q718 is not asked: `doc/questions/A706` item 5 adopts RFC 0014 section 6 as written.
Context: RFC 0014 sections 6 and 8; RFC 0007 sections 3.1 and 4.6; `doc/questions/A706` items 5 and its note on tool names;
ADRs 0954, 1942, 1954.
Code: `crates/pdf-transform/src/profiles.rs` (`read_at_every_target`), `src/archive/base.rs` (`inherit`, `ReadFrom`),
`src/archive/config.rs` (`ConfigError::UnknownBase`, `ConfigError::BaseOfABase`, `Configuration::base`,
`Configuration::answered`), `src/bin/quorra-transform.rs` (`every_target`, `print_remedy_sites`,
`print_remedy_sites_at`, `print_unbuilt_answers`), `tests/profiles.rs`; the directory: `src/profiles.rs`
(`operators_profiles`, `OperatorProfile`, `NotOffered`), `crates/viewer-host/src/archiving.rs` (`ARCHIVE_PROFILES`,
`offer_profiles_from`, `Offered`, `offered`, `planned`), `crates/viewer-host/src/reader.rs`.

## Decision

1. **A configuration is read at every target in `Target::ALL`'s order when the run plans, before `open_inputs`**, and
   the first target it does not read at refuses the run as a usage error naming that target. `--profile` is read the
   same way as `--config`: a shipped profile passes by `every_shipped_profile_loads_for_every_target`, so the check costs
   it nothing and the rule has no exception to remember. A site a target does not bind stays no error at that target
   (`Configuration::read`'s own rule), so a file answering only part 4's sites reads at part 2.
2. **`archive --remedy-sites` with a configuration and no `--to` prints the six listings**, each headed `== <target> ==`,
   and the first target the file does not read at ends the run non-zero after its own listing. With `--to` the listing
   is one target's, as before.
3. **`[profile] base = "<shipped profile>"` is a merge of the two files' tables before validation**, so the merged
   configuration is held to the target by the one reader a single file is.
   - A site the operator's file names in *any* table is the file's at every target, and none of the base's tables for
     it is kept. A merge row by row would let the base's target-qualified row beat the operator's unqualified one at
     that target, and the answer would then be in neither file alone.
   - `[depart."<id>"]` and `[tool.<name>]` are taken over by identifier the same way.
   - `[profile]`'s keys are the file's where it states them and the base's where it does not, except `name`, `sentence`
     and `base`, which say which file this is and are never inherited.
   - **One level**: a base whose own `[profile]` names a base is `BaseOfABase`. The base is a profile this program
     ships, compiled in, so naming one opens no path and a configuration still reads no file (RFC 0002 section 5).
4. **The listing says where each answer came from**: `--remedy-sites --config <file>` prints every answered site as
   *this file* or *from `<base>`* after the unbuilt answers, for every configuration, with or without a base.
5. **The example tool is `to-pdf`** in `derive-attachments.toml`, `keep-everything.toml`, RFC 0014, `tests/profiles.rs`,
   `config.rs`'s test text, `doc/pdfa-remedies.md` and `doc/pdf-a-mitigations.md` (A706's note): the key is the
   operator's label, and the converter passes the attachment's bytes in and reads a PDF back whatever it is called.

6. **A window offers the operator's profiles from a directory its host names** (RFC 0014 section 6's third bullet):
   `--archive-profiles <dir>` on the reader's command line, never a path a document states. Every `.toml` file there is
   read in file-name order, at most `MOST_OPERATOR_PROFILES` (64) files and each at most `MOST_OPERATOR_PROFILE_BYTES`
   (256 KiB) by its metadata before it is read, and is offered only where it reads at all six targets by item 1's check,
   states a `[profile] name`, and takes no shipped profile's name or an earlier file's; every other file is listed as
   not offered with its reason. The directory is read when the chooser opens and again when the conversion is planned,
   not at launch (`CLAUDE.md` principle 2), so a file edited between the two is held to the check as it then is.

## What it costs

An operator whose file answers one target with what another refuses must now qualify the row by target
(`[site."<id>".target."<t>"]`); the check refuses the file until they do, which is the point (RFC 0014 section 6). An
error at a row the base contributed names the base file's line, which the operator's file does not have; the shipped
bases read at every target, so only a combination can raise one.

## Not decided here

RFC 0014 section 5's stages 2 and 3 of `derive` (ADR 2539). The C library behind the WinUI and macOS windows
(`viewer-ffi`) offers the shipped profiles only, and its directory is a later batch's.
