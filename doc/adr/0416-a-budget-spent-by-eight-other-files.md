# ADR 0416 — A budget spent by eight other files

Status: accepted, 2026-08-18. Session 581. Attributes `tools/safedocs survey`'s nondeterminism to
`pdf_model::colour::MAX_PRESSES`, splits the refusal so a caller can tell a process-decided verdict
from a file-decided one, makes the survey say which of its verdicts are its own, and re-establishes
§11.4.7's population with an instrument that has no shared state. Amends the ledger rows for
§11.4.7, §11.7.2 and §12.6.4.3, and `doc/todo/03` and `doc/todo/49`.

## The instrument was not deterministic, and session 580 left it standing

Re-running `safedocs survey` over the crawl moved roughly ten documents in and out of §11.4.7's
report. `doc/habits.md`'s *Measuring* section is why that is worse than its size: an instrument
whose answer changes between runs cannot establish anything, and every population this project has
measured over the web corpus was measured with this one.

Reproduced on the 287 crawled documents whose page-one blending space is named by an ICC profile
this tree evaluates — a directory of symlinks, the machine otherwise quiet, three runs of one
unchanged binary:

| run | incomplete | documents carrying the §11.7.2 press report |
|---|---|---|
| a | 46 | 30 |
| b | 52 | 36 |
| c | 49 | 33 |
| under twelve spinning cores | 51 | 35 |

So it is **nondeterminism and not load sensitivity**: three quiet runs disagree, and the loaded run
sits inside their range. Load changes the interleaving, which is the mechanism; it is not the cause.
(The survey's `slow` count *is* load-sensitive — 1 to 9 across those runs — and that was already
known and written down in `doc/todo/03` §1.)

## The cause, attributed by removing the suspect

`colour::MAX_PRESSES` is 8. A press is a four-component blending space sampled onto a grid a
backend can interpolate; the table is `static`, filled from the front, and **nothing is ever
evicted**. A document naming the ninth distinct press a process meets is therefore refused, drawn
on the device's three components, and reported —

> the press it names is one more than this process samples (§11.7.2), so its four components are
> not converted out

— and *which* documents those are is decided by the order rayon happened to run them in.

`doc/habits.md`'s rule is to attribute by removing the suspect rather than by reading a profile. In
a scratch build with `MAX_PRESSES` at 256, two runs over the same 287 documents produced **19
incomplete both times, with every verdict line byte-identical** and not one press report. The whole
flipping population is this one bound.

## What is actually wrong, which is bigger than the survey

**Every other budget in this tree is spent by the document that reaches it.** `MAX_TILES`,
`MAX_OPERATIONS`, `MAX_FORM_DEPTH`, `MAX_STATE_DEPTH` — each is a fact about the file, the same on
every run and on every machine, which is what let ADR 0271 open the documents that reach them and
say what each bound costs. This one is spent by whatever the process interpreted *first*, so the
same file draws differently depending on what else was opened before it, and says so only in prose
inside a report nobody was reading as prose.

That is a product defect as well as an instrument one: a viewer with several documents open gives
the ninth a page the first would have drawn correctly. It is not fixed here — the fix is an
architecture change, and `doc/todo/49`’s third-bound section now prices the three roads with what each costs. What is
fixed here is that nothing can any longer mistake such a verdict for the file's.

## What was changed

**One refusal became two.** `PagePress::Beyond` carried a `&'static str`; it now carries
`BeyondPress { why, this_process }`. A space this tree cannot sample is a fact about the document;
a spent press table is a fact about the process. The sentence has always said "this process" —
what was missing was a way for a caller to tell them apart without matching on prose.

**The reading order among the reasons is now a rule.** `blending_undrawable` returned the first
reason it found and the press table's was first, so a page that would be reported *whatever* the
table held was reported for the process's reason on some runs and for its own on others —
`2637516.pdf` and `3006744.pdf` are witnesses, both §11.7.5.3's black generation. A file-stated
reason now wins, and the process's sentence is returned only when the file supplies none. That
makes the sentence itself say something.

**`Interpretation` gained two fields, neither of them a report.** ADR 0311's precedent: a count
beside a report rather than a second one. `press_beyond_this_process` says the budget was met
anywhere, page or group; `reports_beyond_this_process` counts the entries of `unsupported` that
exist for that reason and no other, and is counted where each report is *made* rather than where
the press was refused — a page that meets the budget and composites nothing is not reported at all,
and there is nothing about it for an instrument to call unstable.

**The survey says which of its verdicts are its own**, per document and in the summary:

```
287 documents in 47.0s: … 43 incomplete, 1 slow
  of those incomplete, 24 are this process's press budget (§11.7.2) and not the document's — it
  sampled 8 of its 8 presses, and which documents meet it is decided by the order the scheduler
  ran them in, so this figure differs between runs. 19 is the file-decided count, which two runs
  agree on …
```

and each such document's line carries `[this process's press budget]`. A **mark** rather than a
bucket of its own, because a document can be incomplete for both reasons at once — `3990833.pdf`
is, and moving it out of the incomplete list would have hidden §11.4.4's report it carries whatever
the press table holds. That same document is why the subtraction uses the exact count rather than
the flag: three runs print **19, 19, 19**, and 19 is what the `MAX_PRESSES = 256` build printed
with the bound removed altogether.

## The population, re-established with an instrument that has no shared state

`examples/press_census` reads each document's page group and the profile behind it. It shares
nothing between documents, so its answer is a function of the files. Over all 65 703 crawled
documents that open, one process per archive, **run twice and byte-identical over all 145
archives**:

- **2296 documents state §11.4.7's condition** — a page group whose blending colour space is not
  the device's — which is **3.49% of the crawl**.
- **287** of those name their press through a four-component ICC profile this tree evaluates.
- They name **28 distinct presses**. One process samples eight.

Twenty-eight against eight is the whole story: the corpus needs three and a half times the table,
so a survey of it saturates within the first few archives and every later press-naming document is
judged by a budget it did not spend.

**The gates are not affected, and that was checked rather than assumed.** The 974 pdf.js documents
state the condition on 7 pages and name **0** distinct ICC presses — all of them reach the assumed
inks — and the four submodule corpora state it on none at all. `MAX_PRESSES` is therefore never
reached by `tests/corpus.rs`, which is why no ratchet has ever moved for this reason. The census
reads a *page* group; a group inside a content stream can name a press of its own, which is what
`press_beyond_this_process` is for and what running the corpus gate twice checks.

## What was not done, and why

Making the survey deterministic *without* changing the model was considered and rejected. A serial
pre-pass that claimed the eight slots in document order would make the answer stable and arbitrary —
stable in the sense that a coin nailed to the table is stable — and it would still be eight
documents deciding for the rest. Determinism has to come from the budget being per-document, which
is `doc/todo/49`’s third-bound section, or from the instrument saying which half of its answer is its own, which is
this.

## No gate pins the ordering, and that is stated rather than hidden

The reading-order rule only bites when the press table is *full*, and a test cannot arrange that:
filling eight slots needs eight distinct four-component ICC profiles and the table is `static`, so
the test would decide the answer for every other test in its binary. So what holds the rule is the
three-run measurement above and the `MAX_PRESSES = 256` build it is checked against, and what would
make it gateable is the road that scopes the budget to an interpretation — the same road that fixes
the defect. Recorded here because a rule with no gate is `CLAUDE.md` principle 1's kind of debt.

## The spec half: §12.6.4.3, and a reason that named the wrong thing

A `reported` row read against the code. `GoToR` is refused with "a destination in another file,
which this reader has no filesystem to open", and that is true of `pdf-model`, which is handed
bytes. `action.rs`'s §12.7.6.4 comment said the same thing about **this program**, and the program
has had a filesystem in every host since ADR 0244: all three write a file when a person clicks an
embedded one, and `viewer-host` carries §12.7.6.4's file policy for two of them.

The refusal has not expired — but its reason is smaller and nameable now. What `GoToR` needs is a
second `Document` in `viewer-core`'s vocabulary and a **host's decision** about which files a
document may name, which is `CLAUDE.md`'s restriction-policy shape (ADR 0212) with a precedent
already in the tree. The comment is corrected and the row says what would close it.
