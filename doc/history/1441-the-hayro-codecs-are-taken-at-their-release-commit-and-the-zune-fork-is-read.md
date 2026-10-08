# 1441 — The `hayro` codecs are taken at their release commit, and the `zune-jpeg` fork is read

Slot 4 of batch sixty-nine, 2026-10-08, a dependency round. ADR 1714; ADR 1715 and Q337 not used.
No ledger row changed status; §7.4.7's note now names the new pin. **Premises.** The stale
"[n]o release carries it" held: 0.3.1, 0.4.1 and 0.4.0 of 2026-10-04 carry all three pins' reasons.
`doc/history/1436` has no numbered findings; its §7.4.7 paragraph is the one that was meant. The
owner's `doc/zune-image/` holds no uncommitted file, because both commits were pushed at 08:27.

**The pins (ADR 1714).** `64efcaca` is on no ref of `close2/hayro`. Compared with the releases,
JBIG2 differs by one `#[inline(never)]`, CCITT by its version, and JPEG 2000 by six upstream commits.
#1352 among them is the fork's own commit. Taken from crates.io, `hayro-jbig2` is one package with
the reference renderer's copy, and `hayro-interpret`'s unconditional `unsafe` feature compiled the
worker's decoder with `fearless_simd`. `cargo +nightly build --release --bin pdf-sandbox-worker
--unit-graph` printed `fearless_simd, simd, std`. So all three are taken from
`LaurenzV/hayro` at `ea9c81dc`, the tagged release commit, and the same command prints `std`. The
lock moved three packages, and `fuzz/Cargo.lock` the same three, **at 09:54:33**. `deny.toml` swaps
`close2/hayro` for upstream. The three patches are re-based (each applies, and the H.2 test
passes there). `tools/main-checkout.py` now names the fork in the owner's step.

**`zune-jpeg`, read only.** `close2/zune-image` branch `pdf-viewer/0.5.15-with-fixes` is at
`1c7d01b815932b3722766ee88f59b4d61f91771d`, pushed (ls-remote). It is `31d81fed` plus two commits
equal to the two patches applied there: a tree diff is empty. `zune-core` there is byte for byte
crates.io's 0.5.1. The switch is the stanza `Cargo.toml` writes out with that `rev`. After it, the
fork joins `deny.toml`'s `allow-git`, both locks get `cargo update -p zune-jpeg`, and
`banded_decodes.rs` loses its two guards. The owner commits A227 first.

**Unfinished.** The `zune-jpeg` switch waits on the owner's commit. Two stale texts are other
slots': `crates/pdf-sandbox/src/decode.rs:830–831` names `1dc833f7` as the pin, and
`doc/todo/65`:73 says the manifest pins `close2/hayro`. No check holds the feature isolation.

**Gates.** `cargo deny check`: advisories, bans, licences and sources ok. `t88_conformance`: exit 0,
the ADR 1459 table unchanged, 2 of 10 streams. `RUSTFLAGS="-D warnings" cargo clippy` on
`pdf-sandbox`, on `pdf-model` with `hayro-compare`, and on fuzz (`--all-targets`): exit 0 each.
`cargo nextest run -p pdf-sandbox`: 48 passed. `-p pdf-model` without `corpus`: 1997 passed and 18
skipped, the JBIG2 family test among them. Behind the lock as `--tree 6`: `jpeg2000` exit 0, 14 /
13 (worst 1) / 3 as held, 33 s, 3.57 GiB; `corpus` exit 0, 17 s, 1.49 GiB; `raster_golden` exit 101,
971 held, 3 moved (`bug1721218_reduced`, `bug1782186`, `issue2761`), 41 s — each holds `/Lab` and
no JBIG2, JPX or CCITT stream, so slot 3's. `cargo test -p conformance --no-fail-fast`: exit 101
at first on three tests in siblings' files mid-edit, then exit 0, 415 passed, after the record.
