# 1570 — A long command is waited for in calls inside the cache's lifetime

Slot 7 of batch ninety, 2026-10-19, a docs round on the project's working method, answering the
owner's ask of the same day about prompt caching. ADRs 1976 and 1977; no question; no ledger row moves.

**Premise.** It held in direction and was low in size. The brief counted records, and a request is
several records repeating one `usage`: once per `requestId`, the 48 live round transcripts made
7 945 requests that wrote 103.3 M tokens and read 1 680.9 M, and the 332 after a gap over 300 s
wrote 88.2 M — 85 %, not 52 % (all 445 of the session's subagents: 79 %). A read is 0.05×, not 0.1×,
on Opus 5.5 (Anthropic's pricing page). The miss rate by gap calibrates the census: 0 of 7 457
under 300 s, 21 of 25 in [330, 400) s, 295 of 295 past 400 s. **Hypothesis.** Half held: the gaps
are waits on bounded runs, but 219 of the 332 (61.0 M) were the rounds' own wait-loops written to the
Bash tool's 600 s ceiling (`timeout 590 …`), 95 (23.1 M) a foreground `tools/bounded.sh`.

**Price.** The 330 gaps after a command cost 110.0 M base-input equivalents of the cache's 213.2 M;
polled every 240 s they cost 15.0 M with 585 more requests: 94.9 M saved, 45 %, about $380 over the
48 rounds at $4 / MTok. A poll pays for itself on any wait under about 88 minutes.

**Built.** `tools/wait.sh`: `--start <log> -- <command>` runs it in a process group of its own and
waits, `<log>` waits again, and every call returns within a wall — 240 s, refused past 270 s; exit is
the command's, 75 at the wall, 70 for a group gone without its end line; `--stop`, `--until` and a
twelve-case `--self-test` that `cargo test -p conformance --test wait` runs. The runner writes its
own pid file once `setsid` has made it a leader: read by the starter, `ps` once answered first and
named the caller's group, which `--stop` would have ended; planted back, the self-test fails.
`tools/cache-census.py` is the census, kept so the next batch counts the saving (`--since`). The
rule is in `doc/todo/02` section 0 item 3, the environment rule block, the brief template and
`tools/round.sh`'s printed lines; habit 104 is its incident. The orchestrator's hour-long cache lost
18.1 M after 36 gaps over an hour, 35 of them turn ends of 45 269 s mean: the rule is the rounds'.

**Practised.** Every wait went through `tools/wait.sh`, an uncached release build of the workspace
under `--lock --tree 12` among them: 255 s, two calls (75, then 0). This round's own transcript has
no gap over 300 s and no miss.

**Files.** `tools/wait.sh`, `tools/cache-census.py`, `tools/conformance/tests/wait.rs`,
`tools/round.sh`, `doc/todo/02-every-round.md`, `doc/environment.md`, `doc/todo/_brief-template.md`,
`doc/habits/measuring.md`, ADRs 1976 and 1977, this record.

**Gates.** `cargo test -p conformance`: exit 0, 437 passed. `--bin pointers` exit 0, `--bin
quotations` exit 0. `tools/state.sh --round 1570 docs` exit 0. rustfmt `--check` on `wait.rs` clean;
`-D warnings` clippy `-p conformance --all-targets` exit 0. `tools/wait.sh --self-test` exit 0.
