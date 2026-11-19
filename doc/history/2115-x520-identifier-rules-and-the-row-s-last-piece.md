# Round 2115 — X.520's identifier rules, and what §12.8.3.4.4 still owes

A clause round in the `lane3` worktree on §12.8.3.4.4, after round 2114's thirteen X.520 rules.

## Built (ADR 3161)

- `uriMatch` over RFC 3986 section 6.2.2 (`values/uri.rs`); `dnsNameMatch` and `intEmailMatch` over
  RFC 5890's label forms, RFC 3492's Punycode, RFC 5892's derived property and Appendix A's
  contextual rules, RFC 5893's Bidi Rule (`domain.rs`, `punycode.rs`, `code_points.rs`); `jidMatch`
  over RFC 7622 and RFC 8265's profiles (`jid.rs`, `precis.rs`); `presentationAddressMatch` and
  `userPwdMatch` (`structured.rs`). `id-at-role` has no equality rule: ITU-T X.509 clause 16.5.1
  defines it with none. A clear-text password against an encrypted one is a limit.
- Unicode properties from `icu_properties`, already locked under `boa_parser`; `doc/stack.md` has
  the argument and the `unsafe` count; `cargo deny check` passes.
- Tests: RFC 3492 section 7.1's samples, RFC 3986's examples, X.520 clause 8.9.2's note, RFC 7622's
  tables of legal and illegal JIDs (`values/identifier_tests.rs`, fourteen tests). Two planted
  defects (no `Unstable`, no Bidi Rule) each failed a test and were reversed.

## The row stays `partial` (ADR 3162)

The trusted-list piece was built by ADRs 2932 to 2937, and what else is left is limits or the
reader's choice — but ADR 2725's "C caller's fetched copy" is still work: `viewer-ffi` has no entry
point handing a bound copy on as `Command::PolicyCopy`. The note now names it as the one piece owed.

## Premises

The brief's list held, with one more piece than it named. X.520's `pwdEncAlg` points to an Annex L
the 10/2019 edition does not contain.

## Trap

A row's residue sentence can drop a piece an earlier ADR's table still holds; read the table, not
only the note, before moving a status.

**Gates.** Over `-p pdf-signature`: `cargo fmt --check` exit 0; clippy `-D warnings --all-targets`
exit 0; `cargo build -p pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a`
646 passed, 0 failed (exit 0), against an empty audit manifest; rustdoc `-D warnings
--document-private-items` exit 0; `cd fuzz && cargo check --bins` exit 0; `cargo deny check` exit 0;
`cargo test -p conformance --no-fail-fast` exit 0 on the final tree.
