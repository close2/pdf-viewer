# Q286 — May `CLAUDE.md`'s script exclusion become a confined engine under the reader's level?

Asked by round 1390, which built RFC 0008 section 11 item 3 (e): the `Scripts` level in
`viewer_host::policy`, the menus of the three windows, `--scripts off|ask|warn|on`, and the host
supplying `pdf-script-worker` as the runner (ADR 1616). The answer `A193` gave the build and kept
one step for the owner: its `Owes` line ends "`CLAUDE.md`'s exclusion amended with §10's sentence",
and the row that moves on it is the owner's to move.

## What is true now, that was not when the exclusion was written

The exclusion reads: *"JavaScript and script-driven form behaviour — a sandboxed script engine is a
separate project with its own security argument. Field appearance is not excluded; field behaviour
is."* Every part of that security argument is now built and stated in an ADR:

- **The engine runs in a third confined process**, `pdf-script-worker`, under
  `pdf_sandbox::lockdown::Profile::Script` — one thread, no descriptor, no executable memory, a
  96 MiB address-space ceiling — holding no byte of the file, with a 250 ms deadline per trigger
  and a lost worker named and replaced (ADRs 1608, 1609).
- **It is started at the first trigger the reader's level lets run, never at open**, and never at
  all under `off` (ADR 1616).
- **The level is the reader's, at `CLAUDE.md`'s four** — `off`, `ask`, `warn`, `on` — in every
  window's menu and on every command line, `off` by default as `A193` answered, and
  `quorra-confined` pinned to `off` and saying so (ADR 1616).
- **A script's every write is an edit in the view state's log**, never a change to
  `pdf_syntax::Document` (RFC 0008 section 6.4, ADR 1603).
- **Every call that leaves the process is refused by name** at every level: Tier 2's list is not a
  level (RFC 0008 sections 4.3 and 6.3).

The drive holds it in all four windows: at `off` the scripted total stays empty and the window
names the script that did not run; at `on` the worker computes it; at `ask` the question is put
once at the first script, and a `yes` computes the total while a `no` leaves it empty and says so.

## The sentence proposed

RFC 0008 section 10 question 8 wrote the replacement before any of this was built. Written against
what is built, the entry becomes:

> **JavaScript and script-driven form behaviour beyond this document and this viewer** — a script
> runs in scope as far as it reaches *this document's* field values and appearance properties and
> *this viewer's* state, evaluated in a confined worker of its own under a memory and a wall-clock
> budget, and only at the level the reader sets under *A document's restrictions are the reader's
> to set* — `off` by default, and pinned `off` in a window that cannot ask. Acrobat's `AF*` form
> library is in scope as a documented choice with no engine. **Excluded still**: every call that
> leaves the process — the network, the filesystem, mail, printing, other documents, the
> application's chrome, persistence across documents, privileged functions — each refused by a
> named exception the script can see; any effect that would mutate `pdf_syntax::Document`; ISO
> 21757-1's 3D API under clause 13; XFA under Annex K.

Field *appearance* is in scope today and stays so; what the amendment moves is field *behaviour*,
inside the boundary the second half of the sentence draws.

## The row that moves on it

**§12.6.4.17, ECMAScript actions**, `out-of-scope` with `exclusion = "script-behaviour"`. The
clause's requirement is "Upon invocation of an ECMAScript action, a PDF processor shall execute a
script that is written in the ECMAScript programming language", and of the name tree "When the
document is opened, all of the actions in this name tree shall be executed, defining ECMAScript
functions for use by other scripts in the document." Both are executed at `on`, `warn` and an
answered `ask`. Under the owner's ruling `A100` — a requirement executed on request, through a
control a host supplies, is executed, and the default is recorded in the note — the row would go
to **`implemented`**, its note stating the `off` default and `A193`'s condition for asking it again.

## Recommendation

**Amend the entry with the sentence above, and move §12.6.4.17 to `implemented`.** Nothing in the
build depends on the answer: until it comes, the row stays `out-of-scope` and its note says that a
host supplies the runner, and every window still starts at `off`. What the answer decides is
whether the project's own definition of *done* says what the tree does — a restriction lifted by
argument, as `CLAUDE.md` asks, rather than left standing over code that no longer matches it.
