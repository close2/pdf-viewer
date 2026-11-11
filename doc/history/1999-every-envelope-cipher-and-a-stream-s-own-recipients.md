# 1999 — Every envelope cipher, and a stream's own recipients

Session 1999, a single ledger round (§7.6.5, §7.6.5.3, §7.6.6), the second ledger lane. ADRs 2809
and 2810; no question.

**The slice, and why.** ADR 2801's items 2 and 3: §7.6.5.3's four deprecated envelope ciphers and
Table 27's per-stream recipient list. Both are `pdf-pki` and `pdf-syntax`'s `crypt` alone, both
could be pinned by a fixture a tool that states its algorithm writes, and together they close
§7.6.6. Item 4 (RFC 5652 key agreement) needs an elliptic-curve private key in `RecipientKey`, ECDH,
X9.63's KDF and RFC 3394's key unwrap below `pdf-syntax`; it did not fit a three-hour wall under a
load of sixty, and item 1 is round 1997's crates.

**The premise.** Held. Both were refused by name in the code (`EnvelopeError::Cipher`, the
"carries recipients of its own" refusal), effort only, no owner decision behind either; no `A*`
answer cites them after A800.

**Built.** `enveloped.rs` reads `desCBC`, `des-ede3-cbc` (24- or 16-octet keys, RFC 3370 section
5.1), `rc2-cbc` with RFC 2268 section 6's version read back through `Table[]`, and `rc4`, on
RustCrypto's `des` and `rc2` and the workspace's `rc4`; the clause's key bounds are enforced.
`Encryption` keeps a key per filter with a string `/Recipients` (seed alone, digest without step
c)), `StreamFilter::Named` routes a `/Crypt` stream to it for reading and for §7.5.6's writer, and
Table 25's `/AuthEvent` is asked of each key separately.

**Tests.** Known answers from FIPS 81 Table C1, RFC 2268 section 5 and RFC 6229; six envelopes
`openssl cms -encrypt` wrote (`gen_envelopes.py`); a two-key Triple DES envelope; RC2's table read
back; three PDFs `gen_stream_recipients.py` wrote, opened, refused one stream at `EFOpen`, refused
at `DocOpen`. **Plant**: RC2's version taken as the bit count, and step c) applied to a stream's
key — three tests fail; reversed, they pass.

**Gates.** fmt `--check` (`rustfmt` on every file, `cargo fmt -p pdf-pki -p pdf-syntax`): exit 0;
clippy `-D warnings --all-targets -p pdf-pki -p pdf-syntax`: exit 0; nextest `-p pdf-pki -p
pdf-syntax`: 349 of 349, 1 skipped; `cd fuzz && cargo check --bins`: exit 0; conformance after `git
add -N`: exit 0; `state.sh --round 1999 docs`: exit 0 (6487 s held). Tier 2 `corpus`: 101, 974
documents, 62 incomplete, 1 encrypted beyond us, HEAD's counts and breakdown. `raster_golden`: 101,
held 958, moved 16, HEAD's 16 first pages exactly (run at the merge). `launch_path` not run.

**Left of the rows.** §7.6.6 is `implemented`. §7.6.5.3 stays `partial` on key agreement, effort
only. §7.6.5.1 stays `partial` on the host input. §7.6.5 and §7.6 follow their leaves.
