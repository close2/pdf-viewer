# 496 — The arithmetic moves out and the construction stays

**Finding.** An owner decision executed: signature verification's big-integer arithmetic left the
in-tree `crates/pdf-model/src/bigint.rs` for RustCrypto's `crypto-bigint` 0.7.5 — three packages
this tree did not already compile, `Apache-2.0 OR MIT`, no default features — with the three verifiers (`pkcs1.rs`,
`pss.rs`, `dsa.rs`), their budgets and their refusal vocabulary untouched above the seam.
`signature_algorithm_census` over 67 460 documents (811 signature dictionaries in 681) is
**byte-identical before and after**, which is the port's whole observable-behaviour statement.
The whole-scheme `rsa`/`dsa` crates were re-measured and stay declined: `rsa` stable is still on
the `digest` 0.10 second hash stack, and both re-parse keys with a strict DER stack that loses the
four corpus signatures beginning `30 80`. The ECDSA/EdDSA refusal was then re-examined because the
owner's decision kills its heaviest premise — reviewed curve constants now stand on the same
footing as reviewed arithmetic — and the refusal survives on new ground: `p256`/`p384`/`p521` 0.14
turn out to be stable on this tree's line now, but the three real EC witnesses are one DER P-256,
one *plain* brainpoolP256r1 and one *plain* P-256, where "plain" is BSI TR-03111's `r ‖ s`
encoding in a document this tree does not hold and the Brainpool crates are rc-only — so the
stable packages would close one signature of 811.

**Date.** 2026-08-14.
**ADR.** [0331](../adr/0331-the-arithmetic-moves-out-and-the-construction-stays.md), which also
amends 0314 and 0229 in place (superseded-in-part status lines, same commit).
**Touched.** `Cargo.toml` (crypto-bigint, with reasoning), `Cargo.lock` (the fuzz workspace's own
lockfile moved too and is gitignored), `crates/pdf-model/Cargo.toml`, `crates/pdf-model/src/bigint.rs` (rewritten as the seam),
`crates/pdf-model/src/dsa.rs` (digest truncation via the seam; hand-rolled shift deleted),
`crates/pdf-model/src/pkcs1.rs` and `pss.rs` (module docs re-grounded), `doc/stack.md` (public-key
section retitled and re-measured), `doc/conformance/ledger.toml` (§12.8.3.1's refusal ground),
`doc/todo/51` (ECDSA re-grounded on live blockers), `doc/HANDOVER.md` (one sentence),
`doc/adr/0331-*` (new), `doc/adr/0314-*` and `doc/adr/0229-*` (status), this file.

## Gates, as run in this worktree

- `git rebase e937840` clean, `rev-list --count` 0.
- `cargo fmt --all --check` clean.
- `cargo clippy --workspace --all-targets` silent of lints (the only output is `viewer-qt`'s
  documented cold-build gcc `cargo:warning=` lines, `doc/todo/02` §2's note).
- `cargo nextest run --workspace`: **1801 passed, 11 skipped** — one more than the 1800 base,
  which is the seam's new shift test; the two Montgomery-internals tests went with the machinery
  they addressed and the wide-modulus consistency test was rewritten against the seam's API.
- `cargo test --workspace --doc`: 0 failures.
- `cargo test -p conformance`: 5 passed.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok.
- Fuzz, seeded (`fuzz/seed_x509.py` wrote 28 corpus certificates): `cms` 50 000 runs clean,
  `x509` 200 000 runs clean, no artefacts.
- Census: `signature_algorithm_census` over the 67 460-document population, gates profile, before
  and after — `diff` empty.
- **No raster gate was run and none can see this change**: no gate in this tree looks at a
  signature (ADRs 0215, 0229, 0314, 0322 each recorded the same), and nothing outside §12.8's
  machinery was touched.
- `doc/todo/02` §5 was not run: release binaries are built from `main`; whoever merges owns it.

## What the next round should know

- **`bigint.rs` still exists and must stay arithmetic-free.** It is deliberately a seam — the
  file-to-integer conversions, `MAX_BITS`, and one `crypto-bigint` call per operation. If an
  operation is missing, take it from the dependency, do not write it.
- **`Modulus::invert` changed its failure mode in the closed direction**: Fermat's
  garbage-for-non-prime-q became `None`-for-non-invertible, and `dsa::verify` already read `None`
  as "does not verify". Same verdict on every input a real key produces.
- **The ECDSA refusal's old first premise is dead — do not resurrect it.** `doc/todo/51` now
  names the live blockers: rc-only Brainpool on the current line, and BSI TR-03111 (the plain
  `r ‖ s` encoding two of the three witnesses use) being a document `doc/` does not hold. Getting
  that Technical Guideline into `doc/` is the single cheapest thing that would move the family.
- The `dep-probe` scratch crate's measurements (package lists for `crypto-bigint`, `rsa` 0.9.10,
  `dsa` 0.7.0, `p256` 0.14.0) are in ADR 0331; re-run `cargo add`/`cargo tree` rather than
  quoting them once the ecosystem moves.
