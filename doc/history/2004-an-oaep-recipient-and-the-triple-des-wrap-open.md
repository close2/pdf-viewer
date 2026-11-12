# 2004 — An RSAES-OAEP recipient and the Triple-DES key wrap open

Session 2004, a single ledger round (§7.6.5, §7.6.5.3), the second ledger lane. ADRs 2826 and 2827;
no question.

**The slice, and why.** Two of §7.6.5.3's four effort-only refusals, the two a tool that states its
algorithm writes (`openssl cms`). 1-Pass ECMQV needs this tree's own generator from SEC 1 section
3.4, which with the walks owed did not fit three hours; brainpoolP512r1 has no reviewed package.

**The premise.** Held. Both were refused by identifier; no `A*` answer after A800 cites either.
`sha1`, `sha2` and `des` were already `pdf-pki`'s. No `bp512` is published, and `pdf-signature`'s
curve is a variable-time verifier.

**Built.** `pdf_pki::rsa::oaep`: RFC 8017 section 7.1.2 and MGF1, the decoding folded into one flag,
RFC 3560 section 3's parameters with their defaults, appendix A.2.1's seven hashes, anything else
named; `RecipientInfo::KeyTransport` carries its parameters. `key_wrap::unwrap_triple_des`: RFC 3217
section 3.2; a 24-octet KEK, NULL in `keyInfo`, and a wrapped key that is not 40 octets named
(OpenSSL's `-aes128 -wrap des3-wrap`, outside RFC 3217). `key_unwrap` fuzzes it.

**Tests.** Section 7.1.1 encoded by the tests under every hash pair, with and without a label; each
step 3.g condition planted; parameter defaults and refusals; RFC 3217 section 3.4 both ways with
checksum, parity and length plants; five OpenSSL OAEP envelopes and two 3DES-wrap ones open,
parameters asserted first. **Plant**: PS's stray check dropped, MGF1's default SHA-256, step 8
skipped, `keyInfo`'s NULL dropped; four tests fail; reversed, they pass.

**Gates.** fmt of `pdf-pki` and `fuzz/`: exit 0; clippy `-D warnings --all-targets` on `pdf-pki`:
exit 0; nextest `-p pdf-pki -p pdf-syntax`: 373 of 373, 1 skipped; doctests: exit 0; `cd fuzz &&
cargo check --bins`: exit 0; fuzz clippy (`key_unwrap`, `enveloped`): exit 0 once HEAD's unchecked
`key.len() + 8` became `checked_add`; conformance after `git add -N`: this record's placeholder
only, then `records` and the three history gates exit 0; rustdoc `-D warnings` with private items on
`pdf-pki`: exit 0 after one private link was unlinked; `state.sh --round 2004 docs`: 0, after
the wall budget. Tier 2 `corpus`: 101, 974 documents, 62 incomplete, 1 encrypted
beyond us, HEAD's breakdown. `raster_golden`: 101, held 958, moved 16, HEAD's 16 by name. `on_disk`:
exit 0, 963 documents agree on 112562 objects. `script_corpus` not run (5271 s; no script walk
supplies a recipient key); `launch_path` not run (`io.pressure` above 94 %).

**Left of the rows.** §7.6.5.3 stays `partial`: 1-Pass ECMQV and brainpoolP512r1, refused by name,
effort only (ADR 2827). §7.6.5 and §7.6 follow their leaves.
