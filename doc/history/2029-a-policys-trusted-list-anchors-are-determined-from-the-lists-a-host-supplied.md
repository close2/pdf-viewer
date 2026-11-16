# 2029 — A policy's trusted-list anchors are determined from the lists a host supplied

Session 2029, a single ledger round, the signatures lane (`doc/questions/A800` item 3, `doc/todo/77`
item 5; §12.8.3.4.4). ADR 2874; Q856.

**The slice, and why.** TS 119 172-2 clause 4.22.2.1 b)'s `TAsInTrustedList`, whole: both policy
syntaxes, the host's lists and `viewer_core::notes`. Item 7 (clauses 4.6 and 4.7) was not started:
it would have moved the source under the tier-2 walks once they were queued.

**The premise.** Held: both parsers counted the reference, nothing searched it, no `A*` after A800
cites the item. Effort was the only obstacle.

**Built.** `trusted_list::anchors`: the clause's three steps over the host's files by ADR 2842's
names — a specific list's services' certificates, a list of trusted lists standing for the lists it
points at by territory, service types, statuses at the host's current time. `Reading` keeps each
list's file name and the authenticated LOTL. `certification_path::ListedTrustPoint` replaces the
count; each anchor is a trust point with the referencing one's members and the tuple's
`reliableUntil`, tried only where its subject names an issuer in the pool; unresolved references and
unsupplied pointed-to lists are counted unsearched; `Found::Reached::through_list` names the list.
`SignerFacts::with_lists`, `search_certificate`'s lists, `notes` passing the host's reading and its
clock where lists alone were given.

**Tests.** 6 in `trusted_list/anchors/tests.rs`, 3 in `certification_path/tests.rs`, 1 each in the
XML and ASN.1 parsers' tests, 1 in `notes` (an openssl root and signer under `r2029/gen/`, a fetched
policy copy, the list naming the root; its control without lists). **Plants**: step 3 skipped,
`ServiceTypes` dropped, `notes` without the lists: 5 tests failed; reversed, pass.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-signature`, `viewer-core`): 0. nextest of
the two: 1116 of 1116, 7 skipped. Fuzz `cargo check --bins`: 0. Rustdoc `-D warnings
--document-private-items`: 0. Conformance after `git add -N`: 472 passed. Tier 2 (load 40 at the
start): `corpus` 101, 974 documents, 61 incomplete, as at HEAD; `raster_golden` 101, held 951, moved
23, the HEAD set; `selection_census` 0 (974); `accessibility_census` 0 (1020); `confined` 0, 152
swept of 2677 classified (openpreserve, tika-issue-tracker absent), none killed; no script reached.

**Trap.** The fast disk's quota filled mid-round (349 of 350 GiB): "Disk quota exceeded" and a mold
"Bus error". Pruning this lane's own `debug/incremental` (26 GiB), `CARGO_INCREMENTAL=0`, cleared it.

**What is left.** §12.8.3.4.4 stays `partial` on its other residue (ADR 2782) and on lists nobody
supplied (item 4's fetch). `doc/todo/77`: a)'s text (Q855), h)'s TS 119 312 (Q856), the fetch, a C
host's lists, TSAs.
