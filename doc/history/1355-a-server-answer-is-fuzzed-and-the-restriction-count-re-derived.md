# 1355 — A server's answer is fuzzed where a host hands it over, and the restriction count is re-derived

Robustness slot of batch fifty-five. No ADR (no defect of this tree found, no no-route document
moved), no row, no question. Cut by a network outage and resumed; every run reached its 1200 s.

**Targets.** What batches fifty-two to fifty-four changed, and what reaches it: `MAX_LIST_BYTES`
(ADR 1507) and the page-tree scan said out loud (ADR 1533), `page` (`interpret`, `Pages::new`);
the restart plan without `EOI` (ADR 1513), `jpeg_bands` (`banded_decodes`); the kept meets and the
black frame's reuse (ADRs 1517, 1529), `meet` for the area only: `KeptMeets` lives in the device's
encoder, which no target can reach without a GPU, and is held by `kept.rs`'s tests and the raster
gate; the published password (ADR 1534) is test data, `crypt` reaching §7.6's algorithms. Nothing
drove `interact::import` with arbitrary bytes: `forms_data` imports into a document with no form
and `xfdf` into one field. **`fetched_import` is new**: `Command::Respond` against a form with a
field tree, a check box, a choice and a template, with Annex O's `fdf` fragment open, in front or
behind, FDF or XFDF, submission or import; it asserts that every answer is said and said only
about the document it names (ADR 1527 section 3). Seeded by `fuzz/seed_fetched_import.py`
(136 seeds), with its `seeds.sh` arm and `doc/verify.md` line.

**Campaign** (`-s none`, behind the lock, INITED → DONE cov): `fetched_import` 6672 → 9789, 1207 s;
`xfdf` 1895 → 3610, 1244 s; `meet` 591 → 775, 1226 s; `page` 31898 → 32855, 1239 s; `crypt`
837 → 872, 3873 s with the lock wait. Two disk corpora were stale and their first runs bought
nothing: `forms_data` INITED 270 against 1241 from fresh seeds, `display_list` 105. Re-run from
fresh seeds: 1343 → 2698, 1216 s, and 956 → 1674, 1895 s. **Triage**: one crash, `jpeg_bands`
after 241 s, `zune-jpeg`'s `bitstream.rs:400` multiply with overflow checks on, which is
`doc/questions/Q227`'s open patch (no tree fix exists without the owner's fork); re-run with
`-fork=1 -ignore_crashes=1`, 1206 s, 0 crashes. One slow unit, 12 s in `page`: the unmutated seed
`ContentStreamCycleType3insideType3.pdf`, ADR 1507's witness, refused by `ListBytes`.
`doc/verify.md` says how a `jpeg_bands` campaign runs past Q227's crash.

**Counts.** `restrictions.rs` holds its census as an ignored test (`// not a gate:`): 974, 973
open; 25 encrypted, 24 open, 4 as owner; 10 withhold, by name. `restriction.rs`: 963 of 973.

**No route.** `issue17333.pdf`: `/Flags 32`, `/MacRomanEncoding`, code 0, a (1, 0) format-6
subtable holding 165 alone, so §9.6.5.4's last sentence. The three cycles re-read against §8.10.1
and errata #111's paragraph; no ADR after 1411 built a route.

**Gates.** rustfmt on my four Rust files 0; clippy `-D warnings` on `pdf-model` and the fuzz
workspace 0; `cargo nextest run -p pdf-model` 1777 passed; `cargo test -p conformance` 0.
`tools/state.sh fuzz` before and after: crash, timeout and oom counts unchanged in
`fuzz/artifacts`; seeds added for `fetched_import`, `xfdf`, `meet` and `jpeg_bands`.
