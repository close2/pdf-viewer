# 1594 — The interpreter and the fuzzer are two jobs, a start returns at once, and a greeting is held to its constant

Slot 3 of batch ninety-four, 2026-10-19, a docs round. ADRs 2024, 2025; no question.

**Premises.** (1) Held: GitHub's public jobs endpoint gives the `nightly` job's seven green runs of
2026-10-12 to 14 at 2 372–3 278 s of a 3 600 s ceiling, Miri 1 802–2 517 s of it, and round 1587's
local Miri line was 2 841 s. (2) Held as a symptom, not as a hang: round 1590's transcript shows both
calls moved to the background by the harness, one at the default 120 000 ms against `--start`'s own
240 s wall, one at 270 000 ms against `--start`'s 240 s plus the wait's 240 s. Reproduced at 20 s
walls under a 30 000 ms timeout. (3) Did not hold: the merge already made `doc/ui-boundary.md` say
`PDFVCF16`; slot 5 has since moved both to `PDFVCF17`, and the new test passes on that.
**Hypotheses.** Neither held. The fuzz build does not belong with `check`, which is stable-toolchain
fast feedback; the fuzz targets need nightly and a `cargo-fuzz` install. The detached run holds no
descriptor of the caller's (`/proc/<pid>/fd`: `/dev/null`, the log, the log), and under `| cat` the
pipe closed when the caller ended.
**Built.** (1) `nightly` is `miri`, a matrix of one crate a job with ceilings of 75 and 30 min and
the cache keyed by crate, and `fuzz` at 30 min; each ceiling about twice its longest healthy run,
the reason in the workflow's comment and the figures in ADR 2024. (2) `tools/wait.sh --start`
returns once the run has started and names the wait to make; `--wall`, `--tail` or `--until` beside
it is refused on one line with exit 64. Five self-test cases are new, among them a locked run queued
behind another on a scratch lock, started inside 2 s (ADR 2025). (3) `tools/conformance/tests/
greetings.rs`: every `const MAGIC: &[u8; 8]` under `crates/` and `raster/crates/`, and every version
of its family that `doc/ui-boundary.md` or the four navigational documents name, must agree.

**Files.** `.github/workflows/ci.yml`; `tools/wait.sh`; `tools/conformance/tests/{wait,greetings}.rs`;
ADRs 2024, 2025.

**Gates.** Workflow parsed with `yaml.safe_load`: jobs `miri` and `fuzz`, no `nightly`. `tools/wait.sh
--self-test`: 20 of 20; planted with the old waiting start, 6 fail. `greetings`: 2 passed; planted
with `PDFVCF14` in the document, both fail naming it. `rustfmt --check` on the two Rust files: 0.
`-D warnings` clippy `-p conformance --all-targets`: 0. `cargo test -p conformance --test bounded
--test wait --test greetings`: 27 passed. `cargo test -p conformance --no-fail-fast`: 57 binaries,
441 passed, 1 failed (`records`, on slot 7's unfinished `1598` record). `--bin pointers` and `--bin
quotations`: exit 0, no line naming these files. `tools/state.sh --round 1594 docs`: exit 101 twice,
on slot 6's `raster-gpu` mid-edit (`Limits::max_resident_bytes` unresolved); its rustdoc crates hold
no file of this round. About 3 800 s.

**Unfinished.** No green run proves the split, because a round does not push. CI's last run on
`main` also failed `deny`, which this slot does not own.
