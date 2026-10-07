# 1378 — A value is committed from every window, and a map says where you are

UI slot of batch fifty-nine. ADRs 1592 and 1593; no question. The round was cut by the session's
out-of-memory kill and resumed from the worktree.

**The commit (ADR 1592).** `viewer-core` commits a widget's field where its focus leaves it, before
Table 197's `/Bl`; `Command::CommitField` is Enter in a single-line field (Table 231 bit 13) and a
toolkit control losing the keyboard. The commit is `Done::CommitField` in the log, so a replay keeps
it. A refused commit and a keystroke `/K` refuses are both `Event::Reported`; the keystroke is not
logged. Wired: `quorra`'s Enter, GTK's `activate` and focus-leave (from an idle), Qt's
`editingFinished` (deferred while busy), and the confined wire's `COMMIT_FIELD` — the confined window
now types: Tab, characters, Enter, Control and S. **Premise checked**: `7x` is refused at the keystroke,
not the commit, and a save draws an uncommitted value through its format too, so the drive's proof
of a commit is a lone `-` refused at the commit and put back.

**The position (ADR 1593).** `viewer_core::Located`, carried beside `Answer::Measured` and over the
wire: `/PCSM` where it applies, else the least-squares affine map of the registration, its departure
said; written `13.574120° S, 171.193325° E` by `viewer_host::measuring::degrees`. The census over
`doc/pdf.js` finds `bug1146106.pdf` (WKT1 `GEOGCS`); over `doc/corpora` one projected map, refused by
its Hotine method. The confined window measures on `m`.

**OOM audit.** My diff spawns no thread: the two new callbacks are one GTK idle and one Qt
single-shot timer per focus event. Before the kill I ran a release build, the tier 1 tests unbounded
and a 183 s drive of the two new steps; `cargo test -p conformance`'s bounded self-test then failed
its ceiling (exit 0, wanted 137) on a sibling's `tools/bounded.sh`. Threads of user AI before the
resumed first run (the release build, under `ulimit -u 8192` and `bounded.sh --tree 12`): 69;
after: 212, with siblings' builds running beside it.

**Rows.** §12.10.2 stays `partial` (Q271, `/DCS` on another datum); its note says the position reaches
a reader. `doc/state-of-play.md` and `doc/todo/56` say what is.

**Gates.** Each behind the lock under `bounded.sh --tree 12` and `ulimit -u 8192`. `rustfmt --check`
on my files exit 0; `cargo clippy -D warnings` over the seven viewer crates exit 0; `cargo nextest`
over them exit 0, 848 passed; `cargo test -p conformance` 389 passed, 1 failed on this record's own
unfinished Gates line, then exit 0. The whole drive: 141 works / 0 wrong / 3 not offered, exit 0,
1213 s (verdict column 1130 s); the census over `doc/pdf.js` and `doc/corpora` exit 0, 2 s each.
