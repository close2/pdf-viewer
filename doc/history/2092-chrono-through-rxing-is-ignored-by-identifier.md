# 2092 — `chrono`'s unmaintained advisory, reached through `rxing` alone, is ignored by identifier

Session 2092, a dependency round in the lane3 worktree. ADR 3071. No ledger row (a supply-chain decision).

**The premise.** Held. CI's `deny` job (run 44319529598) failed on RUSTSEC-2026-0543, "chrono is unmaintained";
locally `cargo deny check` (cargo-deny 0.20.2) failed the same way, `advisories FAILED, bans ok, licenses ok,
sources ok`. `cargo tree -i chrono --target all -e normal,build,dev` finds one route: `rxing` 0.9.4, the
dev-dependency of `xfa-barcode`.

**Route 1, a feature, fails.** `rxing` 0.9.4 requires `chrono` (it timestamps a decode result); the crates.io
index shows every release from 0.8.3 to the newest, 0.9.4, the same.

**Route 2, another decoder, fails.** `datamatrix` 0.3.3 and `zedbar` 0.7.0 cover Data Matrix and Code 39, but no
pure-Rust crate besides `rxing` decodes PDF417, so that oracle would be lost or `rxing` kept anyway.

**Route 3, taken.** `deny.toml` ignores RUSTSEC-2026-0543 in the form of its other unmaintained ignores: what
reaches it (one crate's tests, never shipped), the cost, and the trigger (an `rxing` without `chrono`, reported
as `advisory-not-detected`). `doc/stack.md`'s barcode row says so. After: `cargo deny check` exits 0,
`advisories ok, bans ok, licenses ok, sources ok`; the 33 `duplicate` warnings are unchanged and not errors.

**Tests.** Every `xfa-barcode` test kept as it was; no source, manifest or `Cargo.lock` line changed.

**Gates.** `cargo deny check` exit 1 before, exit 0 after. No crate's source changed, so fmt, clippy and rustdoc
have nothing to read; nextest `-p xfa-barcode --lib --bins --tests` 32 of 32 passed, exit 0, and the audit
manifest names no `xfa-barcode` test. Conformance after `git add -N`: exit 101 on this record's missing gates
paragraph, fixed, then `--test records` exit 0.

**No trap, no habit.** No question for the owner.
