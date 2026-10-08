# 1446 — A signature policy's published copy is fetched at the reader's level, bound and shown

Slot 3 of batch seventy, 2026-10-08, a clause round and the batch's `partial`-row work. ADR 1728;
ADR 1729 and Q342 not used. No row changed status: §12.8.3.4.4 stays `partial` on enforcement and
its note says the human-readable half is built; §12.8.3.4, §12.8.3 and §12.8 are aggregates
(ADR 1599) and were read, unchanged.

**Premises.** ADR 1709's four OIDs, the Spanish policy and its SHA-1 held; `policy.rs`'s URL
qualifier and `submit::fetch` were where the brief said. **One did not hold as worded**: of the
thirteen Spanish signatures only three carry the URL qualifier, so "the thirteen real ones" through
the fetch is three; ten carry none and have nothing to fetch.

**Built (ADR 1728).** `SignaturePolicy::published` hands out each clause 5.2.9.2 URL with clause
5.2.9.1's digest as an owned `PublishedPolicy`; `PublishedPolicy::binding` is the stored copy's
comparison over fetched octets. `viewer_host::policy::may_fetch_signature_policy` asks the
`Submissions` level (ADR 1527's act class), after the scheme and `check_url` refusals;
`signature_policy_fetched` answers `submit::Reply` — a bound PDF is `Reply::Document`, opened
beside; a mismatch, an all-zero digest, a non-2xx or a failure is said — and
`signature_policy_declined` is "not fetched at this level". Each ends by saying the constraints were
not enforced and why. The fixture is `cms::fixtures::pades_under_a_published_policy`, a URL and no
store, the corpus's shape; the host's tests serve the policy from a loopback listener.

**The three real ones**, through the code by a scratch test since removed: the URL they name,
`sede.060.gob.es`, has no address; the archive's 2016 capture is a 301 to `sede.administracion.gob.es`,
said as a redirect not followed; that address serves 208 306 bytes, 26 pages, and the fetch over it
and over its 2022 capture binds under SHA-1 and answers `Reply::Document`.

**Unfinished.** No window calls the fetch: `viewer_core` hands no `PublishedPolicy` to a host (a
`viewer-core` event, slot 2's crate, named in ADR 1728). `restriction::SUBMITTING`'s words and
`viewer-host/Cargo.toml`'s "named for `Supply` and `Acceptance`" comment are slot 2's to widen.

**Gates.** `rustfmt --check --edition 2024` on the four touched sources: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy -p pdf-signature --all-targets`: exit 0; `-p viewer-host --all-targets`:
exit 101 on `viewer-core/src/viewer.rs:1987` (slot 2's, mid-edit), so re-run with `--no-deps`:
exit 0. `cargo nextest run -p pdf-signature` (the signature corpus gate) behind the lock as
`--tree 6`: 236 passed, 1 skipped, exit 0, 11 s, 1.70 GiB. `cargo nextest run -p viewer-host`: 233
passed, exit 0. Plants: an always-true digest comparison fails both stored and fetched controls
and the host's one-octet control; reverted. `cargo test -p conformance --no-fail-fast`: 417 passed,
exit 0, `the_frontier_map_places_every_open_row_once_and_nothing_else` among them.
