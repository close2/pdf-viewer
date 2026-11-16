# 2035 — A window asks for the trusted lists, and hands the core what it kept

Session 2035, a single ledger round, the signatures lane (`doc/questions/A800` item 3, `doc/todo/77`
item 4; §12.8.3.4.4, §12.8.3.4.5). ADR 2886; no question, ADR 2887 unused.

**The slice, and why.** The whole of item 4, as the brief named it: the key, the act in every host
and the C ABI, the question, the thread, `Command::Trust` read again. It is one path end to end, and
half of it would leave a key that fetches into a directory the verdicts are not worded from.

**The premise.** Held: `trusted_list_fetch` had no caller outside its tests; `WindowAct` had no
variant for it; nothing owner-blocked (A98 set the level, ADR 2882 the port). No `A*` after A800
cites item 4.

**Built.** Shift + T is `WindowAct::TrustedLists`; `QUORRA_ACT_TRUSTED_LISTS` 23 (count 24), header
and C# constant. `reader::Trusting` keeps the three trust words past launch and builds
`Command::Trust`. `trusted_list_fetch::Refresher` runs the refresh and the reading again on a
`fetch-trusted-lists` thread, one at a time; `Subject::TrustedLists`; `requests::trusted_lists` and
the answer for the three shared-trait windows (GTK, Qt, the C ABI's, carried out inside the
library), `quorra`'s own copy in its dispatch; `WindowState::answers_interval` for the two timers.

**Tests.** `viewer-host` `trusted_list_fetch.rs`: three over lists `pdf-signature`'s generator signed
with its own keys (`tests/trusted_lists/`), served by a transport. `viewer-ffi`
`window::trusted_list_tests`: five on a real window, loopback listener, each level. Key table test.
**Plants**: the policy read before the refresh, and `ask` fetching without the question: four fail;
reversed, they pass.

**Gates.** fmt, clippy, rustdoc, fuzz `--bins`, conformance (474) and `state.sh docs`: 0. Nextest
of the five viewer crates under `xvfb-run`: 647 of 647. Tier 2: `corpus` 101 (61 incomplete) and
`raster_golden` 101 (moved 23), HEAD's; the two censuses and `awkward_classes` 0.

**Habit, and what is left.** A key added to the shared table is carried out in the library where
the C ABI can, so the C hosts need a constant and not an arm. §12.8.3.4.4 stays `partial` (policy
enforcement, another lane's); §12.8.3.4.5 stays `implemented`. Left in `doc/todo/77`: Q855's verbatim
text, Q856's TS 119 312, GPR-4.0-03's *should* (refusal re-read and held), lists outside the LOTL.
