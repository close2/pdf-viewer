# 1498 — The read is lexing first, and a content stream's two tokens have a body of their own

Slot 3 of batch seventy-nine, 2026-10-10, a measure round of the owner's performance target (the
partial-row word yields to it). ADR 1833; no ADR 1834, no question. No ledger row moved: §7.2.3,
§7.3.3 and §7.8.2 are where they were, and no token, operand or command changes.

**Premise.** Held: one stream of 278 579 415 bytes, 78 175 395 tokens, 43.41 G to interpret.

**Hypothesis.** Did not hold. Counted as prefixes (`callgrind_interpret --stage=read|lex|operands`,
one thread): lexing 15.33 G (35.3 %), the operators' loop body, a residual, 12.31 G (28.4 %), the
display-list push 4.59 G (10.6 %), the operand stack 3.52 G (8.1 %), `CS`/`SC` colour 2.86 G (6.6 %);
inflating 2.50 G, `q`/`Q` 2.31 G. Push and operand stack are 18.7 %, and the list's vector grows 22
times for 0.24 M instructions, so a reserve has no fifth to cut. The lexer dominates; the cut is there.

**Built.** `Lexer::next_token` reads a fixed-format number (`fixed_format_token`, the one statement
`read_number` also asks first) and a keyword in a 730-byte body; every other token goes to
`next_token_at`, the old `match` unchanged. `callgrind_interpret` gained `--stage=` and `--digest`.

**Built and not kept.** A cold `salvaged` let `next_content_token` inline into `run_reader`: 1.09 G
fewer, 0.41 s slower. Four arms in one `--clock` hold, eight fast cores, one thread, minimum of three
interleaved: HEAD 2.180 s, the lexer split 2.153 s, the wrapper alone 2.586 s, both 2.566 s. Also
dropped: `#[inline]` on the wrapper alone (no change), the allocating reads out of line (+0.15 G).

**Moved.** Lexing 15.33 → 14.39 G (−6.1 %); the interpretation 43.41 → 42.48 G (−2.1 %); the read,
wall, 2.180 → 2.153 s (−1.2 %), no display involved (the owner's display is not this contract's); the
list's digest `07eb7445…68e3` on both. The six launch documents' open and first read moved −0.015 % to
+0.22 % of each process's instructions, inside every `open_kinstructions` band.

**Toolkit thread.** Not the next thing here: a zoom never interprets (`arrange` reads only pages not
yet read; `Open::stale`'s triggers are view states, never a magnification) and the first read is off
the toolkit's thread already, so ADR 1811's shape saves 0 s a zoom step; it would move a turn's whole
read, about 2.2 s, off the thread without shortening it. Next floor: row 2, keywords classified once.

**Gates.** Tier 1, shared worktree: `rustfmt --check --edition 2024` on the two Rust files exit 0;
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-syntax -p pdf-model --all-targets` exit 0; `cargo
nextest run -p pdf-syntax -p pdf-model` 2 330 passed, 20 skipped, exit 0; `cargo test -p conformance`
exit 101 on 1497's record alone (41 lines). Tier 2, behind the lock, from a private worktree of HEAD
and this change: `raster_golden` exit 0, held 974, moved 0, division test ok, queued 4 096 s, held 46
s; the six arms each exit 0 against `/home/AI/arms-1496/`, 0 pages moved by digest or mean error,
queued 3 542 s, held 1 452 s; the `--clock` A/B exit 0, queued 1 470 s, held 31 s. About 12 000 s.
