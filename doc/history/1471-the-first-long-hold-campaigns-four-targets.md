# 1471 — The first long hold campaigns four targets, and the script target reaches the tail of Tier 1

Slot 4 of batch seventy-four, 2026-10-08, a fuzz round. ADR 1776; no ADR 1777, no row moved (a
fuzz round's contract names none), no question.

**For slot 5 (the lock log).** Three `kind=long` holds on lane 2, each one wrapper: the census,
asked 22:57:57, granted 23:00:59, held 1 170.8 s; **the campaign, asked 23:20:55, granted 23:21:23,
held 3 340.0 s to 00:17:04**; the pass and census after, 00:17:22 to 00:29:26, 724 s.

**Premise.** Held: `grep -c kind=long /home/AI/heavy-walk.log` printed 0 before this round, and
`wire::VERSION` is 13. It did not hold that seeds alone could reach round 1463's members: the
target told the realm every key up, no rich value and no page's words, so five members and the
word pair's answer were unreachable whatever a seed spelled. No hypothesis was stated.

**Built.** `fuzz/fuzz_targets/script.rs`: an optional fourth input part (keys byte, then a rich
text field's `/RV`) and words on the realm's pages; an input without it runs as before.
`fuzz/seed_script.py`: five scripts written with four extras each, eighteen for the members the
file answers, `event.keyDown` out of the refused block; every earlier seed byte for byte. The
part's own edges, by taking it away (trap 13): 240 seeds 5 775 with it, 4 991 without.

**Census before → disk plus finds after**: `script` 14 240 stale (fresh 15 461), re-seeded to
8 752 seeds and 15 459, → 21 934; `script_wire` 893 → 2 964; `page` 33 106 → 34 085;
`jpeg_bands` 1 770 → 2 491. Fuzzed (ran less the fork parent's load): `script` 2 339 s,
`script_wire` 2 414 s, `page` 2 403 s, `jpeg_bands` 2 223 s — its load was 1 116 s. Peak 2.15 GiB
with `script`, `script_wire`, `jpeg_bands` at 1 024 MB and `page` at 3 072 MB.

**Stops.** No crash, no timeout. `script`'s one OOM is `s += s` doubled twice over one string,
ADR 1590's operator growth, held by `growth_past_the_ceiling_ends_the_worker_and_is_named`.
`page`'s two slow units are one disk seed and its one-byte mutation: 7.7 s of processor time alone
in the campaign build, 2.1 s in release, 99.8% inside nested Type 3 glyph runs — a cost lead, not a
finding. No test owed. `doc/verify.md`: two sentences (ADR 1776). **Unfinished**: `cargo fuzz
cmin jpeg_bands` in the main checkout (the owner's corpus), so its load stays a third of a campaign;
finds stay in `scratchpad/r1471/run/*/corpus`.

**Gates.** `cargo check --manifest-path fuzz/Cargo.toml` exit 0; `RUSTFLAGS="-D warnings" cargo
clippy --manifest-path fuzz/Cargo.toml --all-targets` exit 0; `cargo fmt --manifest-path fuzz/Cargo.toml`
`--check` exit 0; `cargo test -p conformance` (with `--test fuzz_workspace`'s 4): 427 passed, exit 0.
`fuzz/seeds.sh check`: before, `script` stale and three current; after, `script` current. Four
campaigns, libFuzzer exit 0 each; `tools/bounded.sh` exit 0 on all five locked runs. No crate
touched, so no crate's nextest. Duration 6 704 s.
