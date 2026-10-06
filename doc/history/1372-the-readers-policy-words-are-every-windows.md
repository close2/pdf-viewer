# 1372 — The reader's policy words are every window's

UI slot of batch fifty-eight. ADRs 1580 and 1581; no ledger row in the contract; no question written.

**What was built.** `viewer_host::ReaderWords` (`crates/viewer-host/src/reader.rs`) reads the seven
words — `--trust-anchors`, `--accept-unknown-revocation`, `--reference-files`, `--reader-name`,
`--reader-title`, `--reader-organisation`, `--interface-language` — and `commands` turns them into
`Command::Trust`, `Command::References` and `Command::Audience`, with every file that did not take
as a sentence. `quorra-gtk`, `quorra-qt` and `quorra-confined` now take the words, and `quorra`'s own
parsing of them is replaced by the same function (ADR 1581: one reading, not the toolkits' parsers).
Each window sends the three on its opening thread before the first `Open` (ADR 1580). The confined
window's wire already carried all three. It reads the directories on the host side and sends them
again to a worker that replaces a dead one. `viewer-ffi` already had the three entry points and is
unchanged.

**The brief's premise, checked.** The audience observable it named, the reader's name in a new
annotation's `/T`, does not exist: `Command::Markup` writes no `/T`, and §8.11.4.4's answer is which
content a reader is shown. The drive witnesses each word by its own clause instead. Trust: a
signature made by the drive, certified by a root the drive issues, is called valid under
`--trust-anchors` plus `--accept-unknown-revocation`, and without them the third question is not
asked. References: the reference `XObject` draws the supplied blue page, and its grey proxy
without the word. Audience: a Table 100 `/User` group is drawn for Ada and not for Bob.

**The drive.** `tools/drive-windows.sh` gained `reader_words` (steps 34 to 36) for all four
windows, its fixtures (`drive-signed.pdf` signed with `openssl cms`, `anchors/`, `targets/`,
`drive-reference.pdf`, `drive-audience.pdf`), and `coloured`, which counts pixels through the alpha
channel: `quorra`'s all-grey photograph is stored as grey and defeats a red fill.

**Docs.** `doc/todo/30` loses the item and states what is; `doc/running-the-viewer.md` documents the
words; `doc/state-of-play.md` names the steps. `tools/state.sh windows` drops the three readings,
and its two remaining debts are `Query:FreeTextAt` and `Query:PrintPage`, both other files' items.

**Gates.** `rustfmt --check --edition 2024` on each touched Rust file: exit 0.
`cargo clippy -p viewer-host -p viewer-gtk -p viewer-qt -p viewer-ui --all-targets`: 0 warnings
in these crates. The `-D warnings` run stops at siblings' `pdf-model` mid-edit. `cargo nextest
run` on the same four: exit 0, 398 passed. `cargo test -p conformance --no-fail-fast`: exit 101.
Every binary passes except `records`, on siblings' records `1375` and `1376` still being
written. The whole drive behind the lock: exit 0, 133 works, 0 wrong, 3 not
offered, 1012 s.
