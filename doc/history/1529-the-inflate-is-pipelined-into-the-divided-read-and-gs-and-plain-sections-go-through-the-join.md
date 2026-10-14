# 1529 — The inflate is pipelined into the divided read, and `gs` and plain sections go through the join

Slot 2 of batch eighty-four, 2026-10-14, a parsers round under the owner's performance target; the
partial-row word yields to it. ADR 1894; no question. No ledger row moves.

**Premise, held**: `content/divide.rs` was HEAD's, and nothing in `pdf-model` or `pdf-syntax` moved
from `77fa741b` to `3fc3e7f1`. **Hypothesis, half held**: on 24 threads the read is 386–443 ms, not
under 350; first pages divided rose from 50 to 84 of 974, past 50.

**Built (ADR 1894).** A producer on the pool inflates in place and cuts after a `Q` a byte scan
found between delimiters, at least four segments a thread. Each segment's lex confirms its cut, the
effects compose in order under one lock, and a piece starts once its segment and every fold before
it are in. Anything unsettled restores the interpreter and goes to ADR 1880's plan after the
inflate. `gs` is a spine statement like `cm`. A cut may fall inside a section that only hides, and
the spine opens it again. A caller on a pool thread runs pool work while it waits.

**What did not hold, and was fixed.** The sweep found two defects: a font a spine loaded first lost
its note, and a given-up reading's font memo silenced the re-read. The first fix is
`absorb`; the second is `forget_memos`. **The join hands nothing over**: round 1530 measured an
offer after each join slowing the read from 0.57–0.60 s to 1.005–1.009 s, so the hook is not built.

**The figures moved (one clock hold, `3fc3e7f1` against this round's export, witness
`166bb8be…`):** the read on 24 threads went from 533–575 ms after the inflate to 386–443 ms
pipelined, and on 12 from 547–587 to 433–480 ms. `quorra` under Xvfb: read end 0.743–0.772 →
0.463–0.471 s, first frame 1.846–1.860 → 1.262–1.287 s. The owner's display: read end 0.742–0.772
→ 0.473–0.497 s, first frame 1.918–1.971 s (processor) → 0.980–1.043 s (device chosen).

**Files.** `crates/pdf-model/src/content/{divide.rs,divide/pipeline.rs,reader.rs,run.rs,marked.rs}`,
`src/progress.rs`, `examples/read_split_price.rs`, `tests/drawn_while_read.rs`, ADR 1894,
`doc/state-of-play.md`, `doc/todo/69` §7, this record.

**Gates.** `rustfmt --check` on the eight Rust files: 0. Clippy `-D warnings` on `pdf-model`
(all targets): 0. `cargo nextest run -p pdf-model`: 0, 2048 passed, 21 skipped. `cargo test -p
conformance`: 1 failure, `records`, on round 1528's 43-line record, which is not this round's.
`t2-drawn_while_read`: 0, 84 divided, 84 pipelined, 83 drawn byte for byte, 910 of 974 finished
from parts. `t2-raster_golden`: 0. `t2-corpus`: 0. `callgrind_interpret --digest`: `07eb7445…68e3`,
776 097 064 bytes. The plant (sections left out of the spine, in an export) fails the new fixture.
`launch_path`: 101 on PDF20's `peak_anon_mib`, 19.43 against 19.5, the machine state ADR 1902
guards; 53 banded, 1 outside. The first hold was granted before the build and measured nothing.
Duration 10 900 s.
