# 1394 — A comment about `doc/md/` is read against it, and the Tier 1 column is a gate

Instruments slot of batch sixty-one. ADRs 1624 and 1625; no question written.

**Four stale comments, now stating what is (comment-only hunks).** `pdf-colour/src/mesh.rs`: no
patch budget is derived from the fineness; `MAX_PATCHES` is its own bound and the triangle bound
reaches only a mesh tessellated here (ADR 1217). `pdf-colour/src/colour.rs`: in `ColourSpace::ink`
the `k` terms cancel only where `BG(k) = UCR(k)` and no clamp bites; the direct §10.4.2.2 grey is
taken (ADRs 0217, 1207). `pdf-model/src/attachment.rs`: Table 409a is in `doc/md/` under
`Issue #374`, names `MCAF` alone, `AF` read after it. `xmp.rs`: Issue #296's UTF-8 requirement
stated, UTF-16 and UTF-32 named a departure (ADR 1004). `doc/todo/42`: every digit-form session.

**The sweep** (`tools/conformance/tests/doc_md_absences.rs`, ADR 1624) reads every comment
segment saying something is absent from `doc/md/` against `doc/md/`: 7 claims, 5 with nothing
searchable, 0 false; the unit is a segment, because whole sentences misread `xmp.rs`'s.

**The Tier 1 column is `t2-script_corpus_engine`** (ADR 1625): RFC 0008 section 6.7 names it and
section 6.3's `on` question waits on it. Bounded; constants `HELD_EXCEEDED` 0, ceilings 9 593
threw, 0 refused, 8 unparsed, floor 20 789 runs. At branch it read 9 575 and 20 734; an hour later,
with siblings' edits in the tree, it failed on 18 new throws — the instrument working. 90 763
documents in 107 s after a 33 s build. `run()`'s zero-tests check finds `cargo test` behind a
wrapper; `tools/state.sh scripts` prints both columns; `doc/todo/02` section 2 has the line and a
map row. ADR 1616 makes the worker reached, so `install_featured` builds `pdf-script-worker` with
`engine` in a Cargo run of its own, held by `tests/batch.rs` to every `required-features` binary.

**`gates-cost` names every gate.** `gates()` tested `/dev/stdout` inside a command substitution,
its own pipe, so the summary overwrote the log's first line (`build-sandbox`'s). Now `test -ef`.

**Documents.** `doc/PLAN.md`: the script bullet (ADRs 1602, 1603, 1608, 1609, 1616, 1625), the
seventh frame (ADR 1607), the drive's waits (ADR 1605); `doc/HANDOVER.md` the same and the Tier 1
gate; `doc/state-of-play.md` and `doc/todo/02` section 5 the installed worker. Spelled ordinals,
before → after: `51` 13 → 0, `43` 12 → 0, `10` 12 → 0, `22` 11 → 0 (113 → 65). `doc/todo/65`,
re-read after record 1393, passes the frontier gate, unchanged. Records 1383–1388 are within budget.

**Gates.** `rustfmt --check --edition 2024` on the seven `.rs` files: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy` for `conformance`, `pdf-colour`, `pdf-model` and `pdf-script --features
engine`, `--all-targets`: exit 0. `cargo nextest run -p pdf-colour -p pdf-model -p pdf-script`:
exit 0, 1996 passed. `cargo test -p conformance --no-fail-fast`: exit 0, 395 passed. Behind the
lock, `tools/state.sh scripts`: Tier 0 347 held, Tier 1 exit 101 before the re-take, then the
column alone exit 0, 2 passed. `ps -u AI -o nlwp=` summed: 171 threads.
