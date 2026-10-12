# 1518 — The read is divided where `Q` restores the state, and the next build reads one page on many threads

Slot 4 of batch eighty-two, 2026-10-12, a measure round, the step back of the owner's performance
target. ADR 1873; no ADR 1874, no question. No ledger row moved: the partial-row word yields.

**Premise.** Half held. The first look is round 1509's 0.54 s; the read is no longer 76 % of the first
frame but 96–97 % (ADR 1855 draws while it reads, the frame lands 0.08–0.11 s after the read ends);
the read is 1.74–1.93 s headless, not 2.14. The zoom notch did not hold on the owner's display:
0.62–1.14 s, the device's frame from the seventh notch on, where round 1511 read 0.14–0.16 s.
**Hypothesis.** Did not hold as stated: lexing alone is 335–342 ms, 18–19 % of the read on the clock,
so N lexers and one dispatcher give at most 1.24×. It holds stronger: §8.4.2's `Q` restores the
entire state, so the cut divides the operators too — 3.7× on four fast cores, 6.8–8.0× on 12–16
threads, the joined list equal to the page's.

**Where the seconds go** (HEAD `c6fb18fb`, `quorra` `1baece0c…`, one clock hold at load 1.1–2.7).
Owner's display: read ends 2.40, 3.36, 3.42 s; first look 0.54, 0.54, 0.57 s; first frame **2.48,
3.47, 3.52 s**; last notch → frame 0.62, 1.12, 1.14 s; turn key → frame **2.60, 2.60, 2.63 s** (read
on the toolkit thread 2.03–2.13 s). Xvfb: 2.62–2.84 s; 0.56, 0.56, 0.74 s; **2.72, 2.85, 2.93 s**;
0.116–0.129 s; 2.58, 2.58, 3.35 s. Headless: read 1 739–1 929 ms pinned, 1 828–1 898 unpinned;
inflate 173–212 ms.

**Priced, one measurement each** (`read_split_price`). A, the lexing on many threads: ceiling the read
less its lexing, 1 397–1 487 ms, **1.24×**; the pieces lexed at once 53–107 ms. B, the pieces
interpreted at once from their depth's restated state: 4 pinned 474–521 ms, 12 unpinned **236–267
ms**, 16 229–253 ms; join 117–149 ms; inflate + pass + pieces + join 608 ms, **3.0×**, 3.9× without
the join. **Picked: B** (ADR 1873 §5 lists what its build owes); `doc/todo/69` §6 holds it.

**Unfinished.** No product code, by contract. B is shown on one page whose shape (4.4 M `q … Q` at
depth two) suits it; the corpus's divisible share and the drawer beside a parallel read are the
build's first measurements. The zoom notch's device frame is slot 6's and ADRs 1831/1860's.

**Files.** `crates/pdf-model/examples/read_split_price.rs` (new), ADR 1873, `doc/todo/69` (status,
section 6), this record.

**Gates.** In an export of HEAD plus the example, own target directory: `rustfmt --check --edition
2024` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model --all-targets` exit 0; `cargo nextest
run -p pdf-model` 2 043 passed, 20 skipped, exit 0 (two earlier runs exit 100 on the export's
missing untracked documents, trap 150). `cargo test -p conformance` exit 0. One `--clock --tree 12`
hold: queued 101 s, held 263 s, exit 0 — the example pinned and unpinned, Xvfb three runs, the
owner's display three runs (loop exit 0). Duration 1 900 s.
