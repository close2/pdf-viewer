# 1987 — The signing certificate issuer's retention is answered by an archive cutoff

Session 1987, a single ledger round (§12.8.3.4.4), the second ledger lane. ADRs 2785 and 2786; no question.

**The slice, and why.** The brief put retention first: one OCSP extension and the bound it gives. It was built. No
attribute piece followed it, because the worktree's warm-up build held the build lock for 93 minutes, with load
between 180 and 650 and IO pressure near 90 %. A reader with nothing judging its output would have been a
placeholder, so ADR 2786 records how the attribute residue divides instead: presence and kind need no RFC 5755
reader, and certified attributes do.

**Premise.** Both parsers named `SigCertIssuedByCAKeepsExpiredRevokedCertsInfo` and nothing evaluated it, and
`single_response` skipped `singleExtensions`, so the slice was unbuilt. RFC 6960 section 4.4.4 decides it, so effort
was the only obstacle and no Q823 is needed.

**Built.** `revocation::SingleResponse::archive_cutoff` reads the extension (`GeneralizedTime` only, at most
`MAX_EXTENSIONS` entries), and `revocation::stated_retention` returns what authorised responses state, or
`Undetermined::NoArchiveCutoff` and its siblings. `Requirement::retention` (a `Duration`) replaces `unprocessed`, and
both syntaxes parse into it. `revocation_checks::retention` returns met for a stated interval strictly exceeding the
bound, broken where every stated interval falls short (it names the longest), and not evaluated where nothing
authorised states one or no path validated. The result is `Constraint::Retention`, and on the time-stamp condition
the member is named and not evaluated. `Duration` records whether it rounded a fraction, so `exceeded_between`
decides *exceeding* exactly.

**Tests.** A P-256 CA, signer and five OCSP responses were minted with Python's `cryptography`, and openssl reads
their cutoffs. `retention_tests.rs` (four tests) covers exceeding and equal, the fractional bound, the longest
statement deciding, and the unstated cases. `duration/tests.rs` pins strictness. `ts_119_172_2/tests.rs` and
`ts_119_172_3/tests.rs` test each syntax end to end, and the XML one includes the time-stamp condition. **Plants**:
exceeded_between ignoring the rounding, together with authorisation skipped, fails three; a short retention met
fails five; reversed, all pass.

**Gates.** fmt `--check` and clippy `--all-targets -- -D warnings` of `pdf-signature`: exit 0. nextest: 468 of 468,
5 skipped (`CARGO_INCREMENTAL=0` after a rustc dep-graph ICE). `cargo test -p conformance`: exit 0. Fuzz
`cargo check --bins`: exit 101 from the three errors main's cd743e60 fixes, and exit 0 with that diff applied for the
check and then reverted. rustdoc `-D warnings` of `pdf-signature`: exit 0, after round 1944's `\[t]` trap. The
`state.sh docs`: exit 0, after the hand-off. Tier 2 (`corpus`, `raster_golden`), run at the merge: as at HEAD.

**Left of the row**: attribute certificates and their trust condition (presence and kind first, ADR 2786), trusted-list
anchors, and ADR 2725's limits.
