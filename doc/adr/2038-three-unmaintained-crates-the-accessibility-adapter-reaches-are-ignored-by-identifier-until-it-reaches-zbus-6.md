# 2038 — Three unmaintained crates the accessibility adapter reaches are ignored by identifier until it reaches `zbus` 6

Session 1601, slot 3 of batch ninety-five, under the project owner's "switch to code improvements …
until I say otherwise" of 2026-10-14. Status: **accepted**. Context: `CLAUDE.md` principle 3 and its
scope's clause 14; ADR 0214. Code: `deny.toml` (`[advisories] ignore`), `doc/stack.md`'s
accessibility row. Cites no clause of ISO 32000-2 and moves no ledger row.

## 1. What failed

CI's `deny` job on main failed on 2026-10-19 with `advisories FAILED` and the other three sections
`ok`: RUSTSEC-2026-0370 (`zvariant` 5.13.1), 0371 (`zbus_names` 4.3.4) and 0372 (`zvariant_derive`
5.13.1), each `error[unmaintained]` with "No safe upgrade is available!". `zbus` 6.0.0 (2026-10-16)
took the three in as `zbus::wire`, `zbus::names` and its own derive macros, and its maintainers'
post-release issue (z-galaxy/zbus#2043) says each "receives no further releases". The local
`cargo deny check advisories` (0.20.2) reproduces it, exit 1, the same three and nothing else.

## 2. Why no upgrade answers it

Every route is `accesskit_unix` 0.22.1 → `zbus` 5 and `accesskit_atspi_common` → `atspi-common`.
Read on crates.io on 2026-10-19, nothing newer leaves `zbus` 5: `accesskit_unix` 0.24.0 (2026-09-25,
the newest) asks `zbus ^5.19` and `accesskit_atspi_common ^0.21`, which asks `zvariant ^5.15`;
`atspi` 0.30.0 and `atspi-common` 0.14.0 (the newest of each) ask `zbus ^5.5`, `zvariant ^5.2` and
`zbus_names ^4.1.1`. A bump would move `accesskit` to 0.25 and clear none of the three, so it is not
taken here; the brief's hypothesis held.

## 3. The decision and its cost

The three identifiers are ignored in `deny.toml`, each with its reason, as the two `iai-callgrind`
entries are — but these are in the **shipped** graph, and the comment says so. The cost: a fault
found later in the three is fixed in `zbus` 6 and not in the copies this tree links. The exposure:
they parse what the session bus delivers — a peer under the same user, never a document's bytes —
in the unconfined host, on the adapter's own thread, which `Bridge::new` starts after the first
frame (ADR 0214). An ignore names an advisory, not a crate, so a *vulnerability* advisory against
any of the three still fails the job. The alternative, no AT-SPI adapter, gives up the scope's
"tagged PDF as far as accessibility needs it" on Linux.

## 4. The revisit condition, and that it is mechanical

The first `accesskit_unix` release on `zbus` 6 is taken with the three lines removed. `cargo deny`
says when: an ignored advisory the graph no longer holds is `warning[advisory-not-detected]` —
checked by planting an ignore for RUSTSEC-2020-0071, which no package here matches, in a copy of
the file (`cargo deny --config <copy> check advisories`: exit 0, that warning, `advisories ok`).
