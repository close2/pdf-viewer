# 1753 — A C caller binds a signature policy's copy it fetched itself

Status: accepted and **built**. Session 1458. Takes back ADR 1738 section 1's "the binding
comparison is not offered to a C caller". Context: ISO 32000-2 §12.8.3.4.4; ETSI EN 319 122-1
clauses 5.2.9.1 and 5.2.10, cited and never quoted (ADR 1085).
Code: `crates/viewer-host/src/policy.rs` (`signature_policy_handed`, `bound`),
`crates/viewer-ffi/src/events.rs` (`policies`, `policy_bound`), `crates/viewer-ffi/src/abi.rs` and
`include/quorra.h` (`quorra_event_policy_count`, `quorra_event_policy_url`,
`quorra_event_policy_bind`).
Tests: `events::tests::a_callers_copy_of_a_policy_is_bound_against_the_signed_digest`; the ABI's
counts move to 232 entry points, 216 of them `unsafe`.

## 1. Decision

**The caller fetches; the library binds and says.** A C caller reads the
`SIGNATURE_POLICIES_PUBLISHED` event's list (`quorra_event_policy_count`, `_url`), decides at its
own reader's network level whether to fetch — the decision ADR 1738 keeps a host's, because a URL
is a server — and hands the octets to `quorra_event_policy_bind`, which compares them with the
digest the signer signed and answers `opens` (the signer's copy and a PDF, to open beside with
`quorra_open`) and the sentence, in the two-call idiom.

- **The sentence is the windows' own**, from `viewer_host::policy::bound`, which
  `signature_policy_fetched` now ends in too; only the opening differs — "the copy … that this
  program was handed" — because this library saw no server's answer and must not say one came.
- **No level is read here.** The library makes no request, so there is no act for
  `may_fetch_signature_policy` to rule on; the caller that made one read its own level.
- §12.8.3.4.4's enforcement stays `partial`: every outcome still ends saying the policy's
  constraints were not enforced, and why.
