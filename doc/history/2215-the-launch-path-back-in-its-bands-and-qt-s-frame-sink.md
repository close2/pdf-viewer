# 2215 — The launch path back in its bands, and Qt's frame sink

Round 2215. ADRs 3561, 3562.

## What changed
- The regression (ADR 3561): e2008ed8 and the tree built in release and counted under callgrind,
  diffed by function; the cause is `ac03b9082`'s repair log in the lexer. The parser's look-ahead
  cloned the lexer at every integer and dictionary, and the clone carried the log: `Lexer::mark`
  and `Lexer::restore` look ahead on the lexer itself. The cross-reference stream's record loop
  tested every record for an unrepresentable field with the repair list live in it: the test is a
  pass of its own where `/W` makes such a record possible, and the loop is `read_records`.
- The content reader's per-token `keep_repairs` was measured and was not the cause.
- Qt (ADR 3562): `of_a_showing_window`; the picture is the frame sink's last frame, fitted by
  `/F` and placed from `Query::Screen` at each paint; no wake, the drawing timer looks at
  `Pictures::next_look_unwoken` (`VideoPlayer::next_due_unwoken`, `video::DECODING_LOOK`); Escape,
  Space, Table 218 and close reach it. No bridge function, C++ line or `unsafe` token added.

## Tests
`viewer-qt` `host/tests/shown_here.rs` (to the end; pause, resume, Space, Escape, stop, close);
`pdf-syntax/tests/xref_stream_record_repair.rs` (logged past `u32`, not inside it).

## Ledger
No status moves. §13.2.1 and §12.6.4.14: notes, code and tests restated for the Qt window.

**Gates.** Over `pdf-syntax`, `pdf-model`, `viewer-host`, `viewer-qt`, `viewer-ui`: fmt 0; clippy
`-D warnings --all-targets` 0; `cargo build -p pdf-sandbox -p pdf-media-worker --bins` 0; nextest
`--lib --bins --tests` under `xvfb-run -a` 4061 of 4061, none against the manifest; rustdoc of
`pdf-syntax`, `viewer-host`, `viewer-qt` 0; fuzz `cargo check --bins` 0; `cargo test -p
conformance` 0 after the record's gates; `state.sh launch` 101: all twelve counted figures
inside their bands, one figure outside, `xfa_filled_imm1344e` `peak_anon_mib` 3.469 of 3.000 (3.082 on HEAD).

## Left, in order
1. RFC 0015 section 6 round 8, the C ABI's frame sink.
2. `xfa_filled_imm1344e`'s `peak_anon_mib`, outside its band on HEAD and here: not read.
3. `bug1815476`'s first frame grew 1.2 million from image unpacking since e2008ed8, inside its band.

**Trap.** A function kept out of line to spare a loop the registers around it can cost more
than it spares: a struct passed in and returned crosses the call in memory, and every push in the
loop loads and stores it.
