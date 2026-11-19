# Round 2114 — brainpoolP512r1 key agreement, and X.520's equality rules

A clause round in the `lane3` worktree on the ledger's two leaf `partial` rows.

## §7.6.5.3 — `partial` → `implemented` (ADR 3157)

- Owed: a recipient key on brainpoolP512r1 was refused by name, on ADR 2827's premise that
  `pdf-signature`'s own curve module was variable-time. It was not: `primeorder`'s `*` is the
  constant-time product; `VariableOnly` only means no base-point table.
- Built: the module moved to `pdf_pki::ec::brainpool_p512`; `pdf-signature` verifies on it from
  there, and `PrivateKey` runs Diffie–Hellman and 1-Pass ECMQV on it. No package added.
- Tests: RFC 7027 A.3 through `PrivateKey::agree`; the generated key's two encodings; a
  generator's DH envelope, a generated MQV envelope (`gen_mqv_bp512.py`, round 2044's script with
  RFC 5639 section 3.7's constants added), and an `openssl cms -encrypt` envelope. The session
  worked in `~/scratch/agent/r2114/`.

## §12.8.3.4.4 — stays `partial` (ADR 3158)

- Owed (by the note): a TR 102 272 value of a type whose X.520 equality rule was not held, and
  any value outside ASCII.
- Built: X.520 clause 7's whole preparation over `stringprep`'s table B.2, Form KC and Unicode
  general categories; a census of X.520's 86 `id-at` types into `values::RULES`; thirteen rules.
- Left, in order: `uriMatch`, `dnsNameMatch`, `intEmailMatch`, `presentationAddressMatch`,
  `userPwdMatch`, `jidMatch`, and `id-at-role` from ITU-T X.509. No question needed.

## Trap

A documentation sentence ("variable-time") carried a premise two ADRs then built a refusal on;
reading the dependency's code took one grep. Check a claim about a dependency's behaviour in the
dependency before refusing on it.

**Gates.** Over `-p pdf-pki -p pdf-signature -p pdf-syntax`: `cargo fmt --check` exit 0; clippy
`-D warnings --all-targets` exit 0; `cargo build -p pdf-sandbox --bins` exit 0; nextest `--lib --bins
--tests` under `xvfb-run -a` 1115 passed, 0 failed (exit 0); rustdoc `-D warnings
--document-private-items` exit 0; `cd fuzz && cargo check --bins` exit 0; `cargo test -p conformance
--no-fail-fast` exit 0 on the final tree.
