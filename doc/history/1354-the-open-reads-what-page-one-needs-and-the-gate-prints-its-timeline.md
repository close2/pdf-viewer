# 1354 — The open reads what page one needs, and the launch gate prints its timeline

The PERFORMANCE slot of batch fifty-five: the launch path's eager outline. ADRs 1543 and 1544. No
ledger row moved; no question written. Cut by a network outage and resumed; final gates after it.

**What the open read (ADR 1543 section 1).** Callgrind by function on ISO 32000-2's counted open,
185.1 M: `Outline::read` 35.5 M, and `announce_page`'s `Pages::indices` — the whole page tree, for
the caption's section — 65.1 M, plus 15.7 M dropping both. Page labels (1.6 M), `/OpenAction`,
`ViewState`, the layout and §12.6.3's page events are page one's and stay.

**The change.** `Open::outline` is a `OnceCell`; the opening `PageChanged` names a section only if
both are read; `Viewer::preparation` / `Preparation::run` / `Viewer::prepared` let a host read them
on any thread (the document is an `Arc`, `Sync` since ADR 0260; the core spawns nothing); an
answer for a reopened file is dropped by `Arc::ptr_eq`. `quorra` runs it at the join and takes the
panel's outline when it lands; the gate mirrors that and its turn phase prepares before turning.

**A/B, both arms copied aside and md5'd, three gate runs each, alternated.** Instructions: BPC
4963.8 → 4029.3 k, WTPDF 26692.3 → 13183.7, ISO 185137.9 → 70348.7, bug1815476 3419.2 → 3401.6,
xfa 1821.4 → 1160.3. ISO's cold open 22.0–26.8 → 5.4–6.1 ms, warm 12.5–15.9 → 4.4–5.4, reads 4320
→ 102 KiB in 1075 → 27 calls, allocated 11.6 → 3.9 MiB. First page unchanged (the device is the
longer thread on four rows); every frame hash identical. Bands moved down by the file's rule with
the reason beside each: cold/warm open, `read_kib`, `read_calls`, `open_kinstructions` on four rows,
ISO's `open_peak_mib`.

**`bug1815476.pdf` (item 2).** Its document thread ends page one 2.2–3.0 ms after the device. On
that thread alone: open 2.3 M, page one 318 M, of which `image::unpack` 246.5 M (one-bit CCITT
samples to RGBA one `sample_rgba` a pixel), font catalogue 28.2 M, a JPEG 20.7 M — all page one's
ink, nothing deferrable; the unpack is the lever and is named in `doc/todo/42` for `pdf-model`.
`peak_anon_mib` read 44.5–46.2 MiB in six runs; 1346's 50.73 predates ADR 1532's 4.5 MiB of GL, so
39.0 .. 50.0 stays.

**The timeline (ADR 1544).** Eight milestones a line from the process's spawn, both threads.

**Gates.** rustfmt --check on my files 0; clippy -D warnings viewer-core, viewer-ui --all-targets
0; nextest of both 457 passed; `cargo test -p conformance` 0. Behind the lock: `launch_path` no
clocks 0 (26 banded, 0 outside), with clocks 0 (42 banded, 0 outside, ISO first page 27.9 ms);
drive-windows 0 (111 works, 3 not offered, 0 wrong; `quorra` titled "— Chapter one" at open).

**Left.** `quorra-gtk`, `-qt`, `-confined` and the C ABI take no preparation yet: their opening
caption has no section and their first turn or `Query::Outline` reads both. The one-bit unpack.
