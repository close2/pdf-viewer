# 1955 — RFC 5280's policy tree and name constraints are evaluated in the path validation

Session 1955, a single ledger round (§12.8.3.4.4), the ledger lane. ADRs 2724 and 2725; Q796.

**The premise held.** `trust` fixed RFC 5280 section 6.1.1's policy inputs to their defaults and refused every
certificate carrying `certificatePolicies`, `policyMappings`, `policyConstraints`, `inhibitAnyPolicy`,
`nameConstraints` or a critical `subjectAltName` as an unrecognised critical extension; all three policy syntaxes
kept a trust point's acceptable set, name and policy constraints, and the authority's name constraints, as names
not evaluated. No corpus signature states a machine-readable policy (ADR 1709): policy tests construct theirs.

**Built (ADR 2724).** `x509` carries the six extensions; `trust::policies` is the `valid_policy_tree`, reported
in the anchor's domain, `trust::names` the name constraints for directory names, `rfc822Name`, `dNSName`, URIs and
IP addresses, `trust::constrained` the state in section 6.1's lettered order. `PathInputs` is section 6.1.1 (c),
(e) to (i); a trust point's members are the inputs (RFC 3125 and TS 119 172-3 in `EXPLICIT TAGS`, TS 119 172-2's
base-64 `GeneralName`s); the authority's name constraints are tested on its own certificate. Section 7.1's
comparison is exact on ASCII, undecided beyond. Bounds: 1024 nodes, 32 policies, 64 subtrees, 32 names.

**Tests.** NIST PKITS 4.8 to 4.13: 101 cases, 124 runs, one test per case over PKITS's own certificates, with the
authorities- and user-constrained sets where PKITS states them. Eleven unit tests of the forms, the comparison,
the intersection and the bounds; one test per syntax for a trust point's constraints, met and broken; the
authority's names met, broken and without a path. **Plant**: eleven defects (excluded never fires; permitted never
fires; no anyPolicy fallback; inhibited mapping kept; absent policies keep the tree; intersection skipped;
inhibitAnyPolicy ignored; requireExplicitPolicy ignored; trust point names read implicitly; DNS suffix without a
label boundary; authority names unchecked) fail 14, 14, 29, 7, 7, 8, 6, 34, 1, 2 and 1 tests.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-signature`): exit 0. nextest: `pdf-signature` 445
of 445, `viewer-core` 443 of 443, `viewer-host --test signature_policy` 11 of 11. rustdoc `-D warnings`
(`pdf-signature`): exit 0. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh --round 1955 docs`: exit 0.
Tier 2: `ahead`, `ahead-strips`, `dates`, `xmp`, `on-disk` exit 0; `corpus` and `golden` exit 101 at HEAD and
here, the same 62 incomplete documents and the same 16 moved pages; `scripts` 101 (the engine column's known
ratchet); `jpeg2000` 1 (`opj_decompress` not installed, the test itself passes); `transform` 101 (20.2 pages/s
against 40, a clock under the other lane's builds); `launch` 101 and `turn`, `toolkit-launch` not run: a GPU hang
marker from 08:51, before this round, refuses every hardware section until a person reads `journalctl -k`. The
signature census over the pdf.js corpus and `doc/corpora` (13 `SignedData`) printed the same report at HEAD.

**Left of the row** (ADR 2725): attribute certificates, trusted lists, 4.23.3's freshness and retention, 6.8's
caution period and delay, a mandated path other than the signature's, and non-ASCII name comparison (work); name
forms not compared, stated distances, non-OID policy URIs, `other` revocation, human-readable policies (limits).
