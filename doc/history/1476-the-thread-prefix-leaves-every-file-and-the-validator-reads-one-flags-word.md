# 1476 — The thread prefix leaves every file, and the validator reads the one flags word

Slot 3 of batch seventy-five, 2026-10-09, a docs round. No ADR (1788, 1789, Q372 unused): the one
choice is argued beside its code. No status moved; §9.8.2's and §12.8.3.4.4's notes corrected.

**Premise** held: 12 paths, 13 copies (`raster_golden.rs` has two); `is_symbolic` did `flags & 0b100`
on `as_integer()`. **Hypothesis** held: every copy is a comment, and no test reads one (`git grep`).

**The prefix.** Each copy loses `RAYON_NUM_THREADS=4`; the list is `[&str; 0]`. One sentence beside
them was false the same way: `launch_path.rs`'s `run_phase` said "the heavy-walk lock's prefix sets
four", and now says the wrapper sets four for a locked run and a machine's share otherwise (ADR
1766). Calibrated (trap 13): the prefix planted back in `doc/checks/fixed-documents.toml`, the test
read 2 023 files and failed naming `doc/checks/fixed-documents.toml:[13]`.

**The flags word.** `pdf-archive`'s `is_symbolic` takes the word from `pdf_font::descriptor_flags`;
`None` still means no descriptor or no integer `/Flags`. A word §9.8.2's "unsigned 32-bit integer"
excludes is every bit clear, so the font is judged non-symbolic, as this tree draws it. `None` was
the other reading; it would leave such a font drawn through a base encoding judged by neither of
section 6.2.11.6's encoding rules. `a_flags_word_outside_thirty_two_unsigned_bits_is_judged_non_symbolic`
holds `-4` and `4294967300`; with the old line planted back it fails. Over `doc/veraPDF-corpus`,
6.2.11.6 agrees on 10 and 6.2.10.6 on 9, and every clause of every flavour has 0 `missed`, 0 `over`.
§9.8.2's note said the validator "still tests the integer's bit 3 itself"; it names reader and test
now, and the row's `code` and `test` lists carry both.

**`doc/todo/65`, re-derived.** 19 `partial` and 4 `reported`: the 12 aggregates the map lists and 11
leaves, each in its bucket; the 44 statuses its prose names agree with the ledger. One sentence was
false: §12.8.3.4.4's "no window calls it until `viewer_core` hands a host the policy's URL". Since
ADR 1738 `Event::SignaturePoliciesPublished` reaches `quorra`, `quorra-gtk` and `quorra-qt` (driven
by `tools/drive-windows.sh`'s `signature_policy`), and since ADR 1753 a C caller's copy is bound. The
row's note said the same ("no window calls it"); both corrected. **Not mine, reported**: §12.5's and
§12.5.6's aggregate notes say "§12.5.6.2's and §12.5.6.6's rich text formatting owe", and §12.5.6.2 is
`implemented` — slot 1's rows.

**Gates.** `rustfmt --edition 2024 --check` on the 11 `.rs` files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy --all-targets` for `pdf-archive` and `conformance`: exit 0; for `pdf-model`,
`pdf-script-worker`, `pdf-script`, `pdf-vfs` and `viewer-ui` (comment text only): exit 0.
`cargo nextest run -p pdf-archive`: 281 passed, 2 skipped. `cargo test -p conformance --no-fail-fast`
after the last ledger edit: exit 0, 427 passed. `--bin quotations`, `--bin pointers`: exit 0, no hit
in a line of mine. `cargo test -p pdf-archive --test corpus -- --ignored` as a `--tree 6` walk: exit 0,
wait 0.0 s, hold 33.2 s, peak 0.17 GiB. No nextest for the five comment-only crates. Duration 1141 s.
