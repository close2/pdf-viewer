# 1098 — The instant a token proves, and two rows read again

Batch 1098–1103. Contract: §12.8's two remaining residues — the selection sentence, and
§12.8.3.3.1's instant. ADR 1112 is the argument.

**1. Row by row, and both sentences were stale.** §12.8's — Table 256's `/Data` and Table 259's
`/Fields` "select no objects" — described a debt paid twice over (ADR 1096, ADR 1104), all seven §12.8.2
rows being `implemented`; §12.8.1's was "nothing here validates a certification path, checks revocation
or holds a trust store", falsified in order by ADR 1039, ADR 1067 and ADR 1076. Deleted, with §12.8's
neighbour saying four curves are refused where ADR 1063 left two. Both keep `partial` on "shall be
determined and verified", unmet on a brainpoolP512r1 or Ed448 key; §12.8 also on §12.8.3.4.4's policy.

**2. The instant.** §12.8.3.3.1 leaves a verified token's treatment to the handler; this tree is it,
and ETSI EN 319 102-1 clause 5.5.4's best-signature-time is the treatment. `BestSignatureTime` starts
at the host's clock and a token's established instant lowers it, never raises it — a future `genTime`
proves nothing, the earliest token wins rather than the first asked — and `notes` says it and its
source beside **every** verdict, the clock included, since "valid" alone reads as *valid now*.

**3. What is computed at it, and two comparisons that were wrong.** The validity period (RFC 5280
section 6.1.3 (a)(2)) needed no code, only a value that is sometimes not now. The carve-out for a
revocation dated after the instant is clause 5.5.4 step 4) a) and needed ETSI's document, RFC 5280
section 6.3.3's steps (i) to (k) never reading `revocationDate` — but *which* date is RFC 5280's own
question, section 5.3.2's `invalidityDate` "may be earlier than the revocation date", so `Revoked`
carries both and the comparison takes the earlier. And `revocation.rs` called a list issued *after*
the instant stale where clause 5.2.5 makes the window one comparison, against `nextUpdate`.

**4. The census, 90 763 paths, 90 340 opened, 58 s — and the calibration** (trap 13). 1658 documents,
2075 signature dictionaries; **462** values carry §12.8.3.3.1's timestamp attribute, all 462 commit to
the signature they sit on, **426** of those signers' certificates are past `notAfter` at the census
instant, and **426 of the 426** state a `genTime` inside the period — no negative witness in the world,
so the fixture calibrates it alone (trap 8). That signer is current 2026–2027: at a 2027 clock its path
is `NotCurrent`, at a 2026 instant a token proved it is `Anchored`, a later token moves nothing. Five
arrangements for the carve-out and one says the word — after a proven instant valid; at it, before it,
unproven, and under an earlier `invalidityDate`, not — a moved page byte still stopping it.

**5. Gates.** Tier 1's `fmt --all --check`, both `fuzz/` lines and `--doc` at 0; `clippy --workspace`,
`nextest` and `conformance` red on siblings alone — two lints in `pdf-model/src/variable_text.rs`, another
round's over-budget record — with `clippy -p pdf-signature` 0 and its 185 tests green. Tier 2 at 0:
`pdf-model --test corpus`, `dates`, `xmp`, `jpeg2000`, `gate`, `on_disk` 963 documents 112 562 objects,
`launch_path` 26 banded 0 outside, `raster_golden` 974 held 0 moved, `awkward_classes` 225 swept 0 killed.
