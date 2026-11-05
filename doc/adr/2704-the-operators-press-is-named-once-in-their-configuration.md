# 2704 — The operator's press is named once, in their configuration

Session 1945, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/config.rs`: `[profile] press`;
`archive/press.rs`: `configured_press`; `bin/quorra-transform.rs`: `configured_press`, `press_path`;
`bin/quorra-transform/route.rs`: `configured_press_file`) and in `viewer-host`
(`archiving.rs`: `configured_press`).
Context: the owner's answer A784, ADR 2130, ADR 2705, RFC 0007 sections 4.6 and 4.7.3, RFC 0014
section 6, RFC 0002 section 5.

## Decision

**"Once" is a key of the operator's configuration file**: `[profile] press = "<file.icc>"`, a
relative path taken from the configuration file's own directory. With `[profile] base =
"as-if-printed"` beside it, a company's whole setting is three lines, and every conversion that
reads the file — `quorra-transform archive --config <file>` and a window offering the operator's
profiles from a host-named directory — uses the press. The ranking is the flag
(`--output-intent-profile`), then the configured press, then the shipped GRACoL.

The face that converts reads the file (RFC 0002 section 5: `apply` opens no path) and hands the
bytes to the library's `configured_press`, so the command line and a window hold it to the same
checks; the confined route adds the resolved file to the worker's fences, as it does for every path
the command line names.

## Why not a per-user file in the XDG configuration directory

- **It is invisible at the call site.** RFC 0007 section 4.7.3 puts an operator's intent where the
  command is written, not only in a file that is inherited; a press read from `$HOME` changes what a
  run writes without anything on its command line saying so.
- **It makes a run depend on who runs it.** A batch job under a service account and the same job
  under a person's account would state different presses, and a census or a test run on a
  developer's machine would read the developer's press — the censuses stay unconfigured only
  because nothing ambient is read.
- **It adds nothing the key cannot do.** `[profile] base` already lets an operator's file be a
  shipped profile plus their answers (RFC 0014 section 6), and a window already offers operator
  files; the press travels with the profile that assumes it.

A per-user default stays possible later, ranked below `--config`, if the owner asks for one.

## What the key is held to

A configuration naming a press that authorises `press-assumed` at no site is refused by its line,
because the key would do nothing (RFC 0007 section 4.6, `ConfigError::PressNeverAssumed`). A
shipped profile names none (`tests/archive/configured_press.rs`). The file itself is refused, with
the configuration and the resolved path named, where it cannot be read, is not an ICC profile, is
not an output (`prtr`) profile over CMYK, or states a version the target part's edition list does
not admit (`archive/press.rs`'s module comment has the two lists).
