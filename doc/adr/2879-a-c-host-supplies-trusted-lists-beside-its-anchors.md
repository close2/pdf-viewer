# 2879 — A C host supplies trusted lists beside its anchors

Session 2031, a signatures round (`doc/todo/77` item 6). Status: implemented. Rows: §12.8.3.4.5.
Context: ADRs 1039, 1144, 2834, 2842; ETSI TS 119 612 V2.3.1, TS 119 615 V1.3.1 clauses 4.1 and 4.2.

## Context

`viewer-host` reads `--trusted-lists` into `TrustPolicy::lists` (ADR 2834); the C ABI had only
`quorra_trust_anchors`, which sent `Lists::none()`, so a C host — the Windows host among them —
could not ask any of TS 119 615's procedures.

## Decision

1. **One entry point, `quorra_trusted_lists`, beside the anchors'**: the files' octets, their
   names, a source sentence and the caller's clock. **Names are required**, unlike the anchors':
   ADR 2842 matches a list to the location naming it by that location's last path segment, and
   clause 4.1's files by `OJEU-Loc`, `OJEU-LOTL-Loc` and `OJEU-LOTL-Certs-Set*`, so a nameless
   list could be authenticated by nothing.
2. **The session holds the policy it sent** (`Session::trust`), ADR 1144's shape for the
   restriction policy: `viewer_core::Command::Trust` carries anchors and lists together, so each
   entry point replaces its own half and keeps the other.
3. **The clock comes with the lists where no anchor was supplied**, `viewer-host`'s rule (ADR
   2842): clause 4.1's "has passed" and a listed path are asked at the anchors' instant, and with
   no anchor there would be none.

## Consequences

The ABI has one more pointer-taking entry point; `QUORRA_ABI_VERSION` does not move, since no
struct crosses by value. The header, the C# declarations and `unsafe_position.rs`'s counts move
with it. Fetching the lists stays the caller's transmission (`doc/todo/77` item 4).
