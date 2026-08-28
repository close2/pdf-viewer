# ADR 0747 — A campaign at the documented lengths, and a seeder that counts its own vocabulary

Status: accepted, 2026-08-28. Session 816. Cites no clause: like ADR 0742, which it continues, this
is about an instrument and not about ISO 32000-2, and the conformance ledger is untouched. It sits
beside 0742 (a fuzz run that exits zero without fuzzing), 0264 (`page`, and why a target over
documents needs documents), 0223 (the confined transport and why its decoders are fuzzed) and
0399 (the last defect a fuzz target found here that became a test).

ADR 0742 ended with three things left in writing, and this round takes two of them:

> **`seed_confined_wire.py` still stops at discriminant 25**, four questions short […] It is four
> lines and a re-seed.
>
> **No target was fuzzed for its documented run length in this round.** The measurement was 30 000
> executions and a corpus execution apiece […] Nothing new was found because nothing new was looked
> for.

## Part one — the seeder was not four questions short

`doc/verify.md` said four, ADR 0742 said four, and the four-hundred-and-forty-fifth session — which
found it by counting `query_kind` against `QUERIES` — was right when it wrote it. Read against
`crates/viewer-confined/src/protocol.rs` today the number is **seven**, because three more questions
arrived in the three hundred and seventy-one sessions between that count and this one:

| discriminant | question | arrived |
|---|---|---|
| 26 | `Query::Offset` | ADR 0225 |
| 27 | `Query::FieldSelection` | ADR 0225 |
| 28 | `Query::Fields` | ADR 0235 |
| 29 | `Query::FreeTextAt` | ADR 0238 |
| 30 | `Query::Highlight` | ADR 0357 |
| 31 | `Query::Readback` | ADR 0422 |
| 32 | `Query::View` | ADR 0737 |

**The finding is not the three that were missed. It is that a written-down count decayed exactly
the way `CLAUDE.md` says a written-down count decays**, and that both places holding it decayed
together — the script's `QUERIES` list and the sentence in `doc/verify.md` describing it — so
nothing in the tree could tell anybody. `MAGIC` had already taught this lesson in the same file:
the seven-hundred-and-thirty-sixth session found a *copy* of the greeting one version behind, which
had been silently refusing every re-seed, and the fix was to read the constant out of the Rust.
That fix was applied to one constant and not to the table beside it.

So the table is now derived and the *coverage* is checked. `query_kind`'s constants are read out of
`protocol.rs` by name; `QUERIES` is keyed by those names and states only the **bytes that follow**
each discriminant; and a name the module states which the script has no entry for stops the script
before it spawns a worker, naming what is unasked:

```
crates/viewer-confined/src/protocol.rs's `query_kind` states 32 questions and this script asks 31.
Unasked: VIEW.
Add one entry per name above, stating the bytes that follow the discriminant, and re-seed. A
question nobody asks is an answer nobody has fuzzed.
```

**The hand-written half stays hand-written, and that is the point of the split.** ADR 0223's reason
for a Python seeder is that speaking the wire format by hand is a *second implementation* of
`viewer_confined::protocol`, and two implementations agreeing is a check the round-trip tests
cannot perform on themselves. A discriminant is not the format — it is a number, and a number is
what `CLAUDE.md` says must be counted rather than written. The *shape* after the discriminant — a
point as two `f32`s written as their bits, a machine-word count as a fixed 64 bits, `Zoom::Scale`
as discriminant 3 and a magnification — is the format, and every byte of it is still this script's
own reading. The frame layer stays written down too, deliberately: it is five kinds that do not
grow.

Two smaller things came with it, because the same argument reaches them:

- **The command payloads are seeds now.** `wire::command` is one of the four decoders
  `confined_wire` runs over every input and the seeder kept none of what it sent — it kept only the
  worker's replies. `Command::Resize` and `Command::View` are now kept; `Command::Open`'s is not,
  because it is the document and a corpus of documents is a different target's.
- **`Command::View` is sent before the questions**, so `Query::View`'s answer is a place the reader
  was put rather than the one a worker starts at.

`doc/verify.md`'s block lost its count and gained the sentence that replaces it: the answer to "is
this seeder complete" is now the exit status of a run.

## Part two — the campaign

Every one of the fifteen targets was run for the length `doc/verify.md` states for it, through
`tools/fuzz.sh`, sequentially, one at a time. The figures are in
`doc/history/816-what-a-campaign-at-the-documented-lengths-buys.md`; what belongs here is what they
mean.

### The instrument that makes the figures say something

`tools/fuzz.sh` reads libFuzzer's **final** `cov:`/`ft:` line, which is what ADR 0742 built it to
do. This round found that the *first* one is worth as much: `INITED` is printed after the corpus is
loaded and before a single mutation, so **`INITED → DONE` is exactly what the documented run length
bought on top of the seeds, measured inside one run**. ADR 0742 needed two runs to say that (an
empty directory, and `-runs=0` over the corpus) and its seeded column is reproduced to the unit by
this round's `INITED` figures on ten of the fifteen targets — which is also a check on the corpus
being the same corpus.

The rule worth carrying: **read `INITED` beside `DONE`, and quote the pair.** A single final figure
cannot distinguish a target that found a thousand features from one that was handed them.

### Three findings

**A campaign against a mature corpus mostly confirms it.** Eight of the fifteen targets added fewer
than a hundred features over their documented length, and `forms_data` added **none at all** — cov
488, ft 1375 at `INITED` and the same two numbers at `DONE`. That is not a broken target: it is a
saturated one, and `-runs=50000` is doing no work for it. **The consequence for how work is chosen
is the useful half**: a round that wants to find something in `pdf_syntax` brings *seeds*, not
iterations, which is trap 24's sentence — no amount of wall clock invents a header and a page tree
that agree — applied to a corpus that is already good rather than to one that is empty.

**Two documented lengths are too short, and they are the two targets whose input is a process.**
`display_list` gained +884 edges and +3549 features in its ten minutes — half again what its whole
seeded corpus had — and `confined_wire` gained +660 and +3189 while finishing its million runs in
**156 seconds**. Both were still climbing when they stopped. This ADR records that and does not
change the numbers: a run length is a decision about how much machine a round spends, and the round
that changes one should be able to say what the new one buys. The evidence is in the history file's
table.

**No crashers.** Fifteen targets, no `crash-`, `oom-` or `leak-` artefact; nine new artefacts and
all nine `slow-unit-`, and `page`'s fork parent reporting `oom/timeout/crash: 0/0/0` on every one of
its 109 jobs. The one slow unit cheap to check resolves the way `doc/verify.md` says it will: 25 s
under libFuzzer is **0.226 s** in a release binary. Principle 3's "every crasher found becomes a
permanent regression test" had nothing to bind on, and **that is reported as a result rather than as
an absence** — the `INITED → DONE` figures are what make it a claim about the code instead of a
claim about an exit status, which is the whole of ADR 0742.

## What this does not change

- **No ratchet.** ADR 0742 argued against a coverage floor per target and the argument holds: the
  corpus is gitignored, so a floor would be a ratchet on a fact about a disk. A campaign's figures
  are a record of a run, not a gate.
- **No corpus in the history.** `fuzz/corpus` is 1.2 GB on this disk and two thirds of it is `page`
  and `document`. It is not committable and it is not meant to be; what is committed is the
  *recipe*, which is `fuzz/seed_*.py`, the example beside `display_list`, and `doc/verify.md`'s
  block per target. A crasher is the exception and it does not arrive as a corpus file either: the
  precedent is `crates/pdf-model/tests/hostile_budgets.rs`'s ADR 0399 pair, where the artefact
  libFuzzer wrote was a mutation of a `SafeDocs` document and what went into the tree was a
  **generated minimal document** that reproduces the defect and a test named for what it guards.
