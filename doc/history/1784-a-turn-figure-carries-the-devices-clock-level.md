# 1784 — A turn figure carries the device's clock level, and a band is a claim about one level

An "instruments" round of batch 118, slot 6, 2026-10-26; ADRs 2402 and 2403; Q650 unused. I worked
in `scratchpad/r1784/`. No ledger row moves; the change is in `render-raster`'s tests and example.
**The contract.** `turn_path`'s child samples `pp_dpm_sclk` through each timed frame and prints the
residency per level beside each figure; the check file states how a figure at the lowest level is
judged; then, inside the budget, a listing of the render node's holders this account may read.
**Built.** `tests/support/device_clock.rs`: a 2 ms sampler thread and a `Residency` per figure, kept
inside each timed frame's span by `frame_cost::round` and merged over rounds by `keep`. The child
prints it as fields, the parent beside every child and figure, `frame_budget` a line per row. A row
may state `<figure>_lowest_ms`: a figure read wholly at the lowest level is judged against it, one
whose clock moved between levels is not judged, every other one against `<figure>_ms` (ADR 2402).
`issue19802.pdf` gets `step_lowest_ms = 4.3 .. 7.3`, the file's rule over ADR 2388's 600 MHz arms.
Each child lists the render node's holders of this account and counts the processes it could not
read; no listing of another account's holders is open to it (ADR 2403).
**Measured.** One `--clock --nice 0` hold, HEAD's export and this one's interleaved three times, load
2.97–4.24, busy 5–14 %, no render-node holder of this account: raised arms against raised arms, the
33 figures' quickest differ by a median of +1.1 % (−5.3 to +4.4 %). A second hold of this export alone
(three arms) went looking for the lowest level for the bands and saw it nowhere.
**Premises.** ADR 2388's "raised only while another process keeps the device 30–49 % busy" did not
hold: the device sat raised (605–1 687 MHz) for whole arms at 4–14 % busy, and at 600 MHz for others.
The hypothesis held for the sampler (nothing measurable) and for the `issue19802.pdf` line; "not
judged" off the lowest level, built first, judged 0 of 33 in five of six arms and was rejected.
**Unfinished.** Lowest-level bands for EC3 p101's, `images.pdf`'s and `issue14415.pdf`'s steps, which
the lowest level moves out of their bands (one or two figures each, and the rule is over runs). The
gate's own run on the per-level judgement: both holds ran the build before it; a unit test holds it.
**Gates.** `render-raster`: `cargo fmt --check` exit 0; `cargo clippy --all-targets` exit 0, 0
warnings; `cargo nextest run` 112 passed, 3 skipped, exit 0. The turn gate under the hold: HEAD's arms
exit 101, 0, 0 (`images.pdf` and `issue19802.pdf` steps at the lowest level in the first), this
export's exit 0, 0, 101 (`issue14415.pdf`'s step at 600 MHz in the third); the second hold's three
arms exit 0. `a_planted_busy_device_is_seen_as_busy` exit 0, 31.6 % busy and 1 219–1 687 MHz planted.
`cargo test -p conformance --no-fail-fast` exit 0, run with this record in place.
