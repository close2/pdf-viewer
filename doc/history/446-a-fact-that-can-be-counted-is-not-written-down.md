# Session 446 — A fact that can be counted is not written down

Date: 2026-08-12
Argument: ADR 0281
Touched: `CLAUDE.md`, `doc/HANDOVER.md`, `doc/todo/02-every-round.md`, `doc/todo/README.md`,
`doc/habits.md`, `doc/stack.md` (new), `doc/environment.md` (new), `tools/state.sh` (new)

**A fact that can be counted is not written down; what is written down is the command that counts
it.** The project owner's round, and the rule is its product.

**The evidence was the round before it.** A closing session whose whole job was verification spent
itself correcting **28 stale numbers across 11 files**, and six of the seventeen before it caught
one each — every one a *derived* fact written beside the gate that derives it, and **no gate reads
a sentence**. `doc/HANDOVER.md` had been halved to 909 lines by ADR 0232 and stood at **1845**, of
which **816** — the whole span from *The gates, today* to *The ledger* — were per-round narrative
inside a table's cells. The owner's own example was live: `CLAUDE.md` said seven of Annex O's
eleven `shall`s were carried out where `doc/todo/39` and the ledger had said eight since session
414.

**`tools/state.sh` is the counting mechanism.** Named sections, `quick` in seconds and the whole of
`doc/todo/02` §2 in minutes, printing each command's own summary lines and **performing no
arithmetic** — because arithmetic beside a gate's figure is exactly the thing that goes stale while
the figure is current. A shell script rather than a Rust binary, so that no compile stands between
a question and its answer and its source stays a readable list of the commands the documents used
to state in prose. Two of its sections are not gates: `annex-o` answers the owner's example by
reading `Parameter::unhonoured` — the variants reaching `return None` are carried out, the arms
after it are the refusals, each printed with its own reason — and `counts` is `ls` and `find` over
the populations documents used to state.

**The arithmetic.** `CLAUDE.md` 365 → **344**, with the numeric tokens that are not a clause, a
table, an ADR or a principle number down to **none**, and its 15 session references down to **one**
— the anchor of an argument rather than a date. `doc/HANDOVER.md` 1845 → **718**.
`doc/todo/02-every-round.md` 401 → **269**. `doc/todo/README.md`'s index cells cut to one line
apiece, that being the place the longest false claim this project has recorded lived — §10.5 called
*ignored* and `silent` for eighty-seven rounds after it was implemented. Two new files, because
`CLAUDE.md` should hold principles and nothing else: `doc/stack.md` and `doc/environment.md`.

**The handover is split by *reader* rather than by topic**, which is what ADR 0232 did not do: what
every round needs, then one row per kind of round — reads a clause, judges a page, **measures
anything**, writes a host, takes a dependency, runs the program, asks *when*. The measuring row
points at the script rather than at a table, which is the whole point of the round.

**No lesson was deleted.** The five shapes a refusal takes when it has outlived its reason moved
verbatim into `doc/habits.md`'s ledger section; every trap kept its number so that the several
dozen source comments citing "trap 8 in `doc/HANDOVER.md`" still resolve; and every relative link
in `CLAUDE.md`, `doc/`, `doc/todo/` and all 281 ADRs resolves against the filesystem. Nine
documents and two source comments were pointed one hop on.

**And the history became a directory**, on the project owner's question mid-round: a round writes a
file rather than a line in a shared table, because a shared row is a merge conflict and an
invitation to edit its neighbours — which is how the gate table grew in the first place.
`doc/history.md` keeps sessions 5 to 445 and the two block summaries unchanged.

**Documentation only**, and the gates were run to prove it: `fmt` clean, `clippy --workspace
--all-targets` with no lint, **1619 tests run, 1619 passed, 11 skipped** and the one doctest beside
them, and `cargo test -p conformance` at **6559 citations / 631 quotations** over **875 ledger
rows** — unmoved, which is what a round that touches one doc comment and no code owes. The full
`tools/state.sh` reproduced every figure the deleted gate table had carried.
