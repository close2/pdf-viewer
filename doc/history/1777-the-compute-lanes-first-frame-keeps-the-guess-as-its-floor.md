# 1777 — The compute lane's first frame keeps the guess as its floor

A "measure" round of batch one hundred and seventeen, 2026-10-26; ADR 2388; ADR 2389 unused; no Q.
I worked in `scratchpad/r1777/`. No ledger row moves; the code change is in `raster-gpu` alone.
**The contract.** `issue19802.pdf` page 1's zoom step read 3.17 ms at batch 116's merge against its
band of 1.1 .. 1.8 ms: find the stage that moved, then fix it with no pixel moved or re-take the band.
**Found.** Two causes. The tree's: ADR 2355 sized the compute lane's first frame by its own count,
so the gate's light warm-up left a capacity the page outgrew at 1× and the step outgrew again at
2×, and the step ran whole twice. Its phase list shows one attempt (trap 271); a scratch probe
that repeats the 2× frame on one device read the step at 9.0–10.2 ms against 4.7–5.0 for the
repeat on HEAD. Six trees built on their own, 0f9c12cf (2026-10-07) to ecb6bea0, read within
0.5–1.1 ms of it, so the doubling is 004f8e73's alone. The device's: `pp_dpm_sclk`, sampled every
2 ms, puts the 890M at 600 MHz through a whole child while nothing else uses it, and the step then
reads about 5 ms on every tree. It reads 1.7 ms only while another process (the owner's session,
which this account cannot list) keeps the device 30–49 % busy — and the gate does not judge that.
**Built.** `ComputePersist::first_size`: the first frame is sized by `max(count, floor)`, the floor
the old guess (32 edges a tile, at least 65 536) clamped to the frame budget. A light first frame
gets the pre-2355 capacity back; a dense one still runs once. One new unit test; ADR 2355's
device test now expects the floor. The cause is written above the band, which stays at 1.1 .. 1.8,
plus one `doc/performance.md` paragraph.
**Measured.** One `--clock --nice 0` hold, the gate's own child for the page, HEAD / this / ecb6bea0
six times interleaved, pinned, load 2.70–4.63 (the 60 s cap expired on five arms), no render-node
holder of this account. At ≥ 1 100 MHz the step reads HEAD 2.74–3.28 ms, this change 1.71–1.89,
ecb6bea0 1.70–1.76. Turn and seventh read 6.45–9.22 ms on every arm. An earlier hold of the whole
gate (two of three runs, stopped) was unjudged on the busy device and gave the same pair.
**Premises.** The brief's "HEAD's own step has doubled" did not hold for ecb6bea0: built on its
own it reads like the fix, so the merge's HEAD arm of 3.17 ms was probably not that commit. Both
of the brief's hypotheses held, one each.
**Unfinished.** Below 1 100 MHz the row is about 5 ms on every tree. The gate does not read the
clock, so a red on a quiet device is the machine's (ADR 2242) until it does.
**Gates.** `raster-gpu`: `cargo fmt --check` exit 0, `clippy -D warnings --all-targets` exit 0,
`nextest` 782 passed, 2 skipped, exit 0; `capacity_growth` under `sabotage-capacity` 1 passed. Six
arms on the export (cpu, gpu, compute at 1× and 4×), each exit 0, every page's digest equal against
HEAD's six arms: 968 of 968 pages at 1×, 965 of 965 at 4×. `tools/state.sh --round 1777 docs`:
exit 101, one red, not mine — rustdoc on `pdf-model`, which a sibling edits; `cargo doc -p
raster-gpu` under `-D warnings`: exit 0. `cargo test -p conformance --no-fail-fast`: exit 101, one
red, not mine — `records`, on 1775's Gates paragraph.
