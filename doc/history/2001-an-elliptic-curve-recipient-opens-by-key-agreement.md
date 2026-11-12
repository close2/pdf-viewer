# 2001 — An elliptic-curve recipient opens by key agreement

Session 2001, a single ledger round (§7.6.5, §7.6.5.1, §7.6.5.3), the second ledger lane. ADR 2821;
no question.

**The slice, and why.** ADR 2801's item 4, RFC 5652's `KeyAgreeRecipientInfo`: the last library
item of the public-key handler. Item 1, the host input, is round 1997's crates. The slice is
`pdf-pki` and `pdf-syntax`'s `RecipientKey` alone, and every step has a published vector or can be
pinned by a generator that states its algorithm.

**The premise.** Held. `kari` was read as `RecipientInfo::Other(1)` and never matched. No `A*`
answer after A800 cites it. The curve packages were already in the lock. `aes-kw` was not, so the
unwrap is in-tree.

**Built.** `pdf_pki::key::PrivateKey` (RSA or EC, from all three encodings); `pdf_pki::ec`
(RFC 5915 key, SEC 1 3.3.1 on P-256/384/521 and brainpoolP256r1/384r1); `pdf_pki::key_wrap`
(RFC 3394 2.2.2); `enveloped::agreement` (`kari` read, the RFC 5753 3.1.3 receiving agent, the
ten `dhSinglePass` schemes, `ECC-CMS-SharedInfo`, the X9.63 KDF, the originator by key or by
`originatorInfo` certificate, `rid` by issuer and serial or `rKeyId`). `RecipientKey::new` takes
either family. MQV, the 3DES wrap, AES-wrap parameters, a foreign curve and brainpoolP512r1 are
refused by name.

**Tests.** RFC 5903 section 8 and RFC 7027 appendix A for the primitive; RFC 3394 section 4's six
vectors; `ECC-CMS-SharedInfo` encoded by hand; the KDF restated. `gen_kari.py` wrote eleven
envelopes and `key-agreement-s5.pdf`, and each opens to what it put in. Two `openssl cms` envelopes
open too. **Plant**: the KDF counter started at 0; four tests fail (three `pdf-pki`, the `pdf-syntax`
end-to-end); reversed, they pass.

**Gates.** fmt `--check -p pdf-pki -p pdf-syntax`: exit 0; clippy `-D warnings --all-targets` on
both: exit 0; nextest on both: 364 of 364, 1 skipped; `cd fuzz && cargo check --bins`: exit 0;
conformance after `git add -N`: exit 0; `state.sh --round 2001 docs`: exit 0. Tier 2 `corpus`: 101,
974 documents, 62 incomplete, 1 encrypted beyond us, HEAD's counts and breakdown. `raster_golden`:
101, held 958, moved 16, HEAD's 16 first pages exactly. `on_disk`: exit 0, 963 documents agree on 112562 objects, 11 refused both ways the same. `script_corpus`: not run (5271 s at round 1972; dropped from the queue to keep the wall budget; accepted at the merge: the change runs only when a recipient key is supplied, and no script walk supplies one).
`launch_path` not run (load above 250).

**Left of the rows.** §7.6.5.3 stays `partial`: RSAES-OAEP, ECMQV, the 3DES wrap and
brainpoolP512r1 are refused by name, effort only. §7.6.5.1 stays `partial` on the host input.
§7.6.5 and §7.6 follow their leaves.
