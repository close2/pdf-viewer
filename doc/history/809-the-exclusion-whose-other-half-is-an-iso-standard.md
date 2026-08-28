# 809 — the exclusion whose other half turned out to be an ISO standard

**Finding:** the project owner asked whether it is true that a safe ECMAScript library now exists;
it is — Boa passes 95.21% of test262 against QuickJS's 82.13%, on an independent run dated
2026-08-28 — but **no engine forbids `unsafe` and none has a memory or wall-clock budget of its
own**, and the larger finding is a different one: **the host object model this project assumed the
standard did not define is defined, by ISO 21757-1:2020, a normative reference of ISO 32000-2**.

Date: 2026-08-28. Branch `round-809`, from `main` at `f8ccf50c`.
ADR: **none.** A research round decides nothing, and this one deliberately did not — the exclusion
it is about is the project owner's to amend.

A second agent ran the clause-and-ledger census in parallel; the two measurements were taken
independently and agree. **Every number written down was re-run by the round itself first.** Where
the two differed — the raw-`grep` figures for `/AA` and `/CO` — the better instrument won, and the
worse one is kept in the todo as the foil that shows why.
Files: `doc/todo/56-a-script-engine-that-is-memory-safe.md` (new), `doc/todo/README.md` (its index
row), this file. No `Cargo.toml`, no ledger status, no source.

## What was asked

Verbatim in substance: *"I think I have read that there is now a safe ecmascript library. please
use a round to find out if this is true, and what it would mean for compliance and functionality
improvements. if it sounds good the round should add a todo."*

## The five answers, in the order they matter

**1. Is it true?** Yes, with a qualification the todo states at the top so no later round can read
past it. Three ECMAScript engines are written in Rust and actively developed — Boa, Nova,
Brimstone — and the leading one is *ahead of the C engine everybody embeds*. But "written in Rust"
is not `#![forbid(unsafe_code)]`, and for a garbage-collected language it will not be: counted by
`grep` over shallow clones on the day, Boa carries 623 `unsafe` occurrences (180 of them in
`boa_gc`), Nova 1 155, Brimstone 286 — and Brimstone's own README says its collector is "written in
*very* unsafe Rust". None declares `forbid(unsafe_code)` anywhere.

What changes is the *class* of failure, and the cleanest evidence is one specification bug that
both worlds hit. CVE-2024-43357 is a defect in ECMA-262's async generators that NVD says "may lead
to mis-implementation in a way that could present as a security vulnerability, such as **type
confusion**". Boa's manifestation of it, RUSTSEC-2024-0444, is categorised `denial-of-service` — an
uncaught exception. Same bug; different ceiling. Over the same two years quickjs-ng carries four
heap- and stack-overflow CVEs reachable from "specially crafted JavaScript input", which is exactly
what a hostile PDF supplies.

**2. Compliance.** Measured rather than guessed, and the honest result is *smaller than the
argument for it*: of 875 ledger rows, 113 are `out-of-scope` and **exactly one** carries
`exclusion = "script-behaviour"` — §12.6.4.17. Five `partial` rows name the exclusion as part of
their debt (§7.7.4, §12.6.4, §12.7.3, §12.7.8.3.1, §12.11.5) and §12.6.3 is a sixth in a weaker
sense. **Anyone arguing this on ledger coverage is overselling it**, and the todo says so.

The specification is where the weight is. §12.6.4.17 states two `shall`s at a PDF processor — one
for the action, one for the document-level name tree, which "shall be executed" *when the document
is opened* — and §12.6.3's Table 199 states four more that name an ECMAScript action in the table
cell itself (`/K` `/F` `/V` `/C`), with Table 200 adding five for close, save and print. Fourteen
or so, across five tables, all in clause 12.

**Three rows go the wrong way, and the todo says so where a reader will hit it.** §12.11.1 and
§12.11.2 are `implemented` and §12.11.5 is `partial` *because* nothing runs — §12.11.1's own note
says Table 273's `/RH` "is unread, and the requirement it carries is met by construction rather than
skipped". An engine collapses that construction and gives §12.11.1's "shall evaluate them before
execution of any ECMAScripts" an ordering obligation nothing schedules. **And the standard itself
contemplates a processor that cannot run script**: §12.6.4.14's Table 218 says a rendition action's
`/OP` "is considered a fallback that shall be executed if the interactive PDF processor is unable to
execute ECMAScripts". Neither is a reason to decline, and both belong in front of the owner rather
than in a footnote.

**3. Functionality.** Field formatting, validation, keystroke handling, calculation chains. The
todo is equally concrete about what it does *not* buy — no new pixel is drawn by the engine, since
a script changes a *value* and §12.7.4.3's machinery still constructs the appearance — and about
the three obligations it creates, of which the first is architectural: a script's effect must be an
entry in `view.rs`'s action log beside the document, never a mutation of `pdf_syntax::Document`,
because the oracle's whole comparison rests on `interpret` being a function of the bytes.

**4. Security.** The exclusion's stated reason is one sentence: "a sandboxed script engine is a
separate project with its own security argument." **That sentence has expired.** It was written
when nothing in this tree was sandboxed; `pdf-sandbox` now puts `RLIMIT_AS`, `RLIMIT_NOFILE` and
`RLIMIT_FSIZE` of zero under seccomp-BPF and Landlock with a wall-clock channel budget, and
`viewer-confined` confines the whole viewer. So the placement is a design question with three
candidates rather than a project, and the todo argues for a *third* worker process rather than the
existing one — because a budget must bound the hostile thing without bounding the work the user is
waiting for, which a shared process cannot do. It also disposes of the containment axis: Boa has
no memory ceiling and no interrupt (its one true fuel counter, `instructions_remaining`, is
`#[cfg(feature = "fuzz")]`), and **it does not need one**, because a kernel-enforced `RLIMIT_AS` is
strictly stronger than an engine's own accounting of its own fuel.

**5. Recommendation.** Worth doing, in a strict order, with a first step deliberately narrower than
the commission's guess: `/AA /F` and `/AA /K` on a single field, in a third confined worker, with
`event`/`this`/`util` and read-only `Field` only — **no document-level scripts**, because §7.7.4's
tree is `shall`-executed on open and therefore sits on the launch path principle 2 protects
hardest.

## The measurements this round made

Three populations, with the object-walking instrument rather than a `grep`
(`refused_action_census`, which already existed):

| population | opened | documents stating `/S /JavaScript` |
|---|---|---|
| `doc/pdf.js/test/pdfs` | 964 of 974 | 57 |
| `doc/corpora/*` + `corpus-cache/openpreserve` | 537 of 542 | 7 |
| `corpus-cache/safedocs` | 65 703 of 65 944 | **276** |

**On the SafeDocs crawl this is the most-refused action type this program has, by document count** —
276 against 115 for `/GoToR`, 93 for `/Launch`, 34 for `/SubmitForm`, 7 for `/Rendition`.

`/AA`, `/CO` and §7.7.4's name tree are not action dictionaries, so that census does not see them.
**The round's first draft recorded them as raw-byte `grep` counts and that was wrong twice over**:
`witness_census` already existed, its `objects` layer walks inside §7.5.7 object streams and matches
a `Name` as a token, and the two errors run in *opposite* directions — the grep undercounts
`/JavaScript` by 68% (18 against 57) because 28.3% of the corpus compresses its objects, and
overcounts `/AA` elevenfold (354 against 32) because `AA` is a substring of arbitrary bytes. A lower
bound and an upper bound at once is not a bound. ADR 0403 is where that lesson was first paid for.

Measured properly, as stated names:

| | pdf.js (964 opened) | SafeDocs (65 703 opened) |
|---|---|---|
| `/JavaScript` | 57 | 378 |
| `/AA` | 32 | 210 |
| `/CO` | 12 | 82 |
| `/AcroForm` | 165 | 4 766 |

**Two readings the round did not expect.** First, against the population that has an `/AcroForm` at
all this is a **34.5%** feature in pdf.js and a **5.8%** one in the crawl — *it is a forms feature,
not a corpus-wide one*, and the factor of six between the two denominators is a measurement of
pdf.js's own bias (it is the regression suite of a viewer that implements JavaScript). Second, the
crawl states a script action in 276 documents and an `/AA` in only 210, **so most of the world's
script actions hang off `/OpenAction`, an outline item or §7.7.4's name tree rather than off a field
trigger** — which means the todo's recommended first step reaches a minority of the demand on
purpose, scoped by safety rather than by coverage. That sentence is in the todo because a
recommendation that hid it would be dishonest.

`cargo tree` against this tree's own `Cargo.lock`: `boa_engine` with `default-features = false`
pulls 122 transitive packages, 68 already among this workspace's 521 and **54 new** — a 14% growth
in the third-party graph for a feature 0.42% of the world's documents use. That number belongs in
front of any decision and is why it was taken.

## What the round did not do, and why

- **No ADR.** The instruction anticipated this and reserved 0741; the round declines it. Nothing
  was decided. When the owner rules on the exclusion, *that* is the decision, and its ADR belongs
  to the round that implements the ruling.
- **No RFC.** `doc/rfc/` is awaiting the owner's review and the round was told not to touch it. The
  todo says an RFC is the right home for the placement design and names 0006 as the free number
  without taking it.
- **No change to `CLAUDE.md`.** The exclusion is amended "by argument, never by attrition", and a
  round editing the sentence it is arguing about would be the attrition. The argument is written
  out in the todo's §8, in the form the owner would have to accept — including the four claims that
  cut *against* it, because a proposal that hides its own counter-evidence is worth nothing.
- **No dependency, no engine, no feature code.**

## The gates and the sweeps

**§2 by the change→gate map, documents only**: the six core lines — `cargo fmt --all --check`,
`RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`, `cargo nextest run --workspace`,
`cargo test --workspace --doc`, `cargo fmt --manifest-path fuzz/Cargo.toml --check`,
`RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — plus
`cargo test -p conformance`, which the map names for a documents-only change because it reads
citations and quotations out of the tree. All green, including round 807's
`tools/conformance/tests/workspaces.rs`. `tools/round.sh` says this is not a fifth round and the
change can move no pixel, so the rest of the sequence is not owed.

**§5 is not owed either, and the reason is worth stating rather than assuming.** Its rule is
*before any measurement* — of the launch path, a page turn, a frame, a memory high-water. This
round's measurements are corpus censuses run from `pdf-model`'s own examples, built in this
worktree from this round's `HEAD`, so nothing here is a measurement of a stale binary.
`tools/round.sh` does report that this worktree's `target/` holds none of §5's binaries, which is
true and is what a fresh worktree looks like.

**§4 sweeps, before and after, against a pristine checkout of the base commit `f8ccf50c`** in
`.claude/worktrees/r809-base` with the same gitignored data linked into it. All fifteen
`conformance` sweeps run in both. **Eleven are byte-identical.** The four that move do so only in
their summary line, and every delta is accounted:

| sweep | delta | reading |
|---|---|---|
| `tables` | +15 sentences naming a table, +9 attributed key citations, **all 9 agreeing**; `absent` 101, contradicted 6, no-such-table 0 — **all unchanged** | the new file's table citations are all right |
| `quotations` | +52 quotations in +2 documents, **+14 verbatim in a specification**; **diverging unchanged at 38** | every quotation of ISO 32000-2 in the new file is verbatim, and the ISO 21757-1, NVD and ledger quotations correctly match nothing in ISO 32000-2 |
| `pointers` | +45 path pointers: +18 live, +18 unrooted, +2 a form, +7 not carried; **`absent` unchanged at 98, `undefined` at 13** | no new dead pointer |
| `counts` | +22 sentences, +4 attributed counts, all in *attributed to a clause with no rows below it*; agreeing and *counted no such way* **unchanged** | the sweep's benign class |

**Two false starts are recorded because the next round should not repeat them.** The first
before/after showed `pointers` moving 124 live and 102 not-carried, which was **the baseline's
fault, not the change's**: `tools/worktree.sh` symlinks the fourteen `doc/*.pdf` specifications into
a round's checkout and a hand-made `git worktree add` does not, so a hundred pointers at
`doc/ISO_32000-2_sponsored_EC3.pdf` read as *not carried* on one side and *live* on the other. **A
baseline is only pristine if it is provisioned the same way**, and `tools/worktree.sh`'s `linked`
list is not the whole of what it does. The second: `tables` and `pointers` each caught a real error
in the new file — a source line reading `` `crates/boa_engine/RUSTSEC-2024-0444.md` `` that the
pointer sweep correctly read as a path into *this* tree, and a sentence attributing `/AA` to Table
200 when Table 200 *is* the dictionary and Table 29 states the key. **Both were the sweeps doing
exactly their job on a document, which is the population `doc/todo/48` added them for.**

## One thing worth keeping beyond this item

**"The specification defines nothing here" decayed again, and this is the third recorded instance.**
`CLAUDE.md` already carries two — `DeviceCMYK` → RGB, and the transfer functions that "describe a
marking device". This one is the same shape from a different direction: the exclusion did not
*claim* the standard was silent, but the reasoning everybody carried around it did — the host
object model was assumed to be Adobe's convention. §12.6.4.17 says otherwise in one sentence, and
ISO 21757-1:2020 has been published since December 2020. **The check that would have found it is
the one `CLAUDE.md` already prescribes: read the titles around the subject in `doc/md/`, and read
the normative-references list.** It took a `grep` for `21757`.
