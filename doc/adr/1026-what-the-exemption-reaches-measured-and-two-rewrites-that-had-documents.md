# 1026 — What the exemption reaches, measured, and two rewrites that had documents all along

Session 1007. Status: **accepted**. Audits ADR 1021's `reach::exemption_narrows` from the two sides
the closing round could not: the reach it *permits*, subclause by subclause, and the reach it
*took*, requirement by requirement, over every corpus on this disk. Adds
`crates/pdf-archive/src/withdrawal.rs` and `crates/pdf-archive/examples/withdrawn.rs`; amends
`crates/pdf-archive/src/reach.rs`, `crates/pdf-archive/src/lib.rs`,
`crates/pdf-archive/tests/reach.rs` and `doc/todo/62`. Does **not** answer
`doc/questions/Q62` — it prices the three options the question offers, and §5 is that pricing.

`§N` is ISO 32000-2 and nothing else; ISO 19005 clauses are written out. Every figure below was
produced in this session by the command beside it.

## 1. What was missing, in one sentence

`reach::exemption_narrows` keeps section 5.1 and each part's file-structure carve-out and narrows
**everything else** — one `else` arm over a few hundred rows. That is what the two published
sentences say, and it was checked by seven fixtures and by a corpus that does not move either way.
What nothing could say was *which rows it actually reaches*, and the closing round said so: "the
corpus cannot rank an exemption that withdraws failures no `-fail-` document depends on". A rule
the exemption withdraws on every document that would otherwise fail it is a rule no document is
ever judged against — which is exactly the shape `doc/questions/Q62` found twice in the converter,
and nothing in this tree could have found a third.

## 2. The instrument: `examples/withdrawn.rs`

`examples/unreferenced.rs` counts the population — documents stating a named resource nothing
references, and the objects only such an entry reaches. This one counts what that population does
to the requirement **table**: every row the target binds is run twice where the narrowing could
bite, once over the whole document and once over the population section 6.2.2 leaves, and the
difference is counted per row identifier in documents. `crate::check` already runs a failing
predicate twice; what is new is keeping the two answers apart instead of only the verdict.

Three things are worth recording about it:

- **The exempt population is computed once per document and lent to every target.** It is a
  function of the object graph and the content streams and of no field of the target, so an example
  judging one file against six targets pays one walk where `check` would pay six. That is the
  difference between the corpus finishing in a minute and in six.
- **Every bound is reported and its documents named**: unreadable, larger than `--max-mb`, panicked
  under a predicate, slower than ten seconds, skipped by name. A run that quietly dropped a
  thousand files would report a reach that is an artefact of what it dropped.
- **It prints the reading beside the measurement.** Each narrowed row carries what §3's audit says
  its subclause is, so a row narrowed under a subclause the audit calls *not a resource* raises
  itself as `over-narrowing?` in the run rather than waiting for somebody to compare two documents.

## 3. The reading: `src/withdrawal.rs`, every bound subclause of both parts

147 subclauses — the ones `coverage::subclauses()` calls `Bound`, which are exactly the ones a row
cites — each with one of four answers and a reason:

| answer | what it says | count |
|---|---|---|
| `Kept` | the part's carve-out, or section 5.1, keeps it; `exemption_narrows` is `false` | 19 |
| `Resource` | a named resource, or an object only one reaches, can be its subject | 58 |
| `NotAResource` | its subject is the file, a page, an annotation or the catalog — the narrowing is a fall-through that **cannot fire** | 43 |
| `NoFileCanFail` | every requirement in it binds a conforming processor or is outside validation | 27 |

Six tests hold it up. The population is `coverage`'s in both directions; `Kept` is checked against
`exemption_narrows` for every target of the part, which is a claim about the code rather than a
reading; `NoFileCanFail` is checked against the table, so a row promoted out of `Check::Processor`
there fails the build by name; every answer must state a reason; and Q62's two subclauses are
pinned by name, so that the question's evidence does not move under it while it is open.

**`NotAResource` is the variant this round exists to write down.** The clause does exempt the
resource from those requirements too — there is simply no object for the exemption to be about,
because a header, a trailer, an output intent, an annotation or a catalog key is what the
requirement judges. That is *meaningless rather than wrong*, and a fall-through does not
distinguish it from the fifty-eight subclauses where the narrowing is live.

Two entries are worth naming on their own:

- **ISO 19005-4 section 6.1.5, hexadecimal strings, is `Resource`** — and its part 2 equivalent,
  section 6.1.6, is `Kept`. Part 4's published carve-out keeps only its sections 6.1.6 to 6.1.9,
  and hexadecimal strings are its 6.1.5; A010's range for part 2 starts at 6.1.2 and covers them.
  So a hexadecimal string written inside an exempt object is outside part 4's requirements and
  inside part 2's. Nothing moves today, because both rows report a span of the file rather than an
  object — but the asymmetry is the two published texts' and it is now stated rather than latent.
- **ISO 19005-4 Annex A.2** carries §4's finding.

## 4. The one thing the code got right for a reason that was written down wrong

`reach::components` returns an empty reading for a clause number that is not digits and dots, and
its comment said two things:

> A component this crate's own table did not write as digits stops the reading, which makes the
> clause fall outside every carve-out above and so keeps the requirement. Every clause in
> `crate::table` is digits and dots; a future one that is not would be kept rather than silently
> exempted.

**Both halves were wrong.** An empty reading falls outside both carve-outs, so `exemption_narrows`
answers *yes* — the requirement is narrowed, not kept. And fifteen rows cite such a clause today,
three of them with a predicate: ISO 19005-2 Annex B.1 (`signatures/digest-covers-the-whole-file`),
ISO 19005-4 Annex A.2 (`embedded-files/pdfa-4f-carries-embedded-files`) and ISO 19005-4 Annex B.2.2
(`annotations/three-dimensional-stream-format`).

The *answer* is right, and the clause is why: both parts exempt the resource from every requirement
of the **document**, and a normative annex is part of the document, so nothing in Annex A or Annex B
is carved back out. What was wrong was that nobody had decided it — a reader of that comment would
have believed the opposite of what the function does. So the comment now states the annex answer and
says what it used to claim, `tests/reach.rs` pins all three annex clauses, and `withdrawal.rs`
carries the reason at ISO 19005-4 Annex A.2.

**No over-narrowing was found.** Every row the measurement saw narrowed sits in a subclause §3
calls `Resource`; not one `over-narrowing?` was printed over the 4148 documents of §5.1 or the
264 of the crawl a checkpointed run reported (§5.2). Every
annex subclause is `NotAResource` or `NoFileCanFail`, so the corrected fall-through withdraws
nothing from any document either — which is why this is a comment fixed and a test added rather
than a behaviour changed.

## 5. What the exemption took, and `doc/questions/Q62` priced

### 5.1 Over the seven read corpora

`cargo run --release -p pdf-archive --example withdrawn` over `doc/veraPDF-corpus`, `doc/pdf.js`,
the four `doc/corpora/` submodules and `doc/corpora-own` — 4148 documents against all six targets,
one skipped by name (§7), 12 unreadable, none bounded, none panicking:

**107 documents were asked for the exempt population and had one** — the question is put only
where a row the exemption narrows has already failed at a place naming an object — and **fourteen
rows** had a finding withdrawn from at least one document. In documents, `failed /
narrowed / cleared`: `graphics/device-rgb-needs-a-default-or-an-rgb-output-intent` 868/1/0 and its
part 4 twin 853/1/0; `fonts/font-programs-embedded` 448/4/0;
`metadata/extension-schemas-embedded` 214/1/1; `fonts/to-unicode-present` 137/1/1;
`fonts/cidset-lists-every-cid-in-the-program` 63/3/1; `fonts/to-unicode-values-are-usable` 53/1/0;
`fonts/symbolic-truetype-states-no-encoding` 43/1/1;
`graphics/content-streams-have-an-explicit-resources-dictionary` 39/1/0;
`fonts/non-symbolic-truetype-uses-a-standard-encoding` 28/1/1;
`fonts/non-symbolic-truetype-differences-are-listed-names` 14/1/1;
`graphics/no-postscript-passthrough-in-a-form-xobject` 9/2/2;
`graphics/no-postscript-xobjects` 5/1/1; `graphics/no-form-xobject-opi` 4/1/0.

**Eleven of the 175 implemented rows fail no document in any of the seven**, and the run lists
them: two ICC-profile rules, two output-intent rules, one blend-mode rule, two font rules, two
implementation limits and two XMP serialisation rules. That is the *other* way a requirement goes unjudged and it has nothing to do
with the exemption — the corpora do not hold the fault. One of the eleven is an artefact of this
run's own bound and says so: `implementation-limits/indirect-object-count` is unexercised because
the document that exercises it is the one §6 skips by name.

Two readings of that table matter more than the numbers:

- **The exemption narrows twenty row–document pairs out of 4148 documents, and clears a row
  outright on nine of them.** It is a rule about a handful of files, and it was worth implementing
  for the coverage question rather than the robustness one — ADR 1021 §4 predicted exactly that,
  and this is the prediction measured from a third side.
- **"Judged by no document" is corpus-relative, and the run shows it.** Over
  `doc/corpora/format-corpus` alone, `fonts/non-symbolic-truetype-uses-a-standard-encoding` fails
  one document and the exemption withdraws it: a row judged by nothing in that corpus. Over all
  seven it fails 28 and stands on 27. A round that measured one corpus would have recorded a
  finding that the next corpus refutes, which is `doc/habits/measuring.md`'s negative-claim decay
  in its smallest form.

### 5.2 Over the crawl, and the bound that ate the first answer

**The first pass was lost to a bound, and that is a finding about the instrument.**
`--threads 8 --targets 2a,4 --max-mb 256` over `corpus-cache` as a single root — the SafeDocs,
tika-issue-tracker and openpreserve archives, 89 286 documents — walked **87 000** of them in
3247 s and was then stopped by `tools/bounded.sh`'s twelve-gibibyte `RLIMIT_DATA` (exit 134, the
tree at 10.8 GiB with eight workers). It printed nothing, because a report is printed per *root*
and the root was the whole crawl. Six workers and `--max-mb 96` is the configuration that fits;
naming the corpus's subdirectories as separate roots is what makes a long walk survivable, and the
lesson is written at `sweep` in the example so the next round does not pay it again.

**The second pass, checkpointed by root, ran while this round was asked to converge**, so what it
reported is `corpus-cache/openpreserve` — 264 documents, two targets — and `tika-issue-tracker` was
at 23 000 of its 23 075 when the round stopped it by pid. The openpreserve figures, `failed /
narrowed / cleared` in documents: `fonts/font-programs-embedded` 160/4/0;
`fonts/to-unicode-present` 38/1/1; `graphics/device-rgb-needs-a-default-or-an-rgb-output-intent`
75/1/0 and its part 4 twin 75/1/0; `graphics/no-form-xobject-opi` 2/1/0;
`fonts/non-symbolic-truetype-uses-a-standard-encoding` 1/1/1; and **a row §5.1 never saw** —
`graphics/rendering-intent-entries-name-one-of-four` 1/1/0, ISO 19005-2 section 6.2.6, on
`AIAA-2002-4016-606.pdf`. Every one of the seven is audited `a resource`; no `over-narrowing?` was
printed here either. Thirty of the 264 were asked for the exempt population and had one.

**Two cautions about that corner.** Its govdocs1 files are the same documents as
`doc/corpora/format-corpus`'s, so its figures are not independent of §5.1's; and 75 of 171 rows
fail no document in it, which is what a 264-document corpus does to the coverage question.

What is left of the crawl is a command rather than an unknown, and `doc/todo/62` §7 carries it:

```sh
cargo run --release -p pdf-archive --example withdrawn -- --threads 6 --max-mb 96 \
  --targets 2a,4 corpus-cache/tika-issue-tracker corpus-cache/safedocs/cc-main-2021-31/*/
```

### 5.3 Q62's two rewrites, priced

The question records both as reachable by no document, each with a *drawn* case it says is refused
and a *not drawn* case the exemption exempts. The measurement says otherwise, and the disagreement
is about the drawn case in both rows.

**`Rewrite::PostScriptXObject` (ISO 19005-2 section 6.2.9.3).** Five documents in the seven corpora
fail `graphics/no-postscript-xobjects`, all in `doc/veraPDF-corpus`; the exemption withdraws it on
**one** (`PDF_A-1b/…/6-2-5-t03-pass-a.pdf`) and it stands on four. Running the converter at `--to
2b` over all five: the answer is the mechanical rewrite on **five of five**, and on two of them the
rewrite ran and a file was written — `Isartor test files/PDFA-1b/…/isartor-6-2-7-t01-fail-a.pdf`
and `PDF_A-2b/…/6-2-9-3-t01-fail-a.pdf`, one site each.

**`Rewrite::SymbolicTrueTypeEncodingRemoved` (ISO 19005-2 section 6.2.11.6, ISO 19005-4 section
6.2.10.6).** Forty-three documents fail `fonts/symbolic-truetype-states-no-encoding`; the exemption
withdraws it on **one** (`doc/pdf.js/test/pdfs/bug1883609.pdf`) and it stands on 42. Running the
converter over all 43 at `--to 2b` and at `--to 4`, with identical answers under both: **25 are
refused** — the proof `sites::selects_the_same_glyphs` makes, not ADR 0816's fence — and **18 have
the rewrite as the converter's answer**, of which **7 wrote a file with the rewrite applied**:
`8.4.5.7-t03-fail-a.pdf`, `6-2-11-6-t03-fail-a.pdf`, `6-2-10-6-t03-fail-a.pdf`,
`bug1027533.pdf`, `issue8229.pdf`, `bug1337429.pdf` and `bug1151216.pdf`. The other eleven are
documents some *other* row refuses, so no file is written and the rewrite is chosen without running.

**So the question's premise holds for neither rewrite**, and the shape of the error is the same in
both: Q62 reasoned from the twelve synthetic fixtures ADR 1021 §7 names — documents whose page
declares a resource and draws nothing — and the corpora hold the drawn case in numbers. What the owner is choosing
between is therefore not "retire dead code" but "keep two rewrites that nine documents on this
disk actually run" — and the third option, reopening ADR 0816's fence, is about a *different* row
(`fonts/symbolic-truetype-program-has-a-usable-cmap`) than the one the rewrite answers. The question
stays open and this ADR does not touch it; `doc/questions/Q62` is the owner's.

## 6. Cost

The instrument's own numbers, on a machine running five other rounds. The seven read corpora —
4148 documents, six targets, eight workers — take **21 s** end to end when the machine is quiet,
and the same run was minutes under load; `doc/veraPDF-corpus`'s 2907 documents are two seconds of
that. The crawl's cost is §5.2's.

**One corpus document costs more than every other document of `doc/veraPDF-corpus` together.**
`Isartor test files/PDFA-1b/6.1 File structure/6.1.12 Implementation Limits/isartor-6-1-12-t01-fail-a.pdf`
is the implementation-limits torture file, and judging it against six targets ran for **more than
eighteen minutes** before this round stopped it by pid; the other 2907 documents take two seconds
between them. It is 4 MB of indirect objects, and the six targets fetch the object population six
times over. This round skipped it by name — `--skip`, reported in the run — rather than pretend it
was measured, and it is not this round's item: the fix is a shared `Examination` population across
targets, which is `crate::check`'s shape rather than an example's. `doc/todo/62` §7 has the command;
whoever takes the cost item has the witness.

Three further documents take longer than ten seconds apiece and the run names them
(`doc/pdf.js/test/pdfs/bug1978317.pdf` at 40 s, two `govdocs1-error-pdfs` at 19 s).

## 7. What this round did not do

- It did not answer `doc/questions/Q62`, and it did not touch `crates/pdf-transform/`: session 1006
  holds that crate this batch. The converter was **run**, never edited.
- It did not change what any document's verdict is. `over` is 0 on all six targets, the frontier
  stays empty, and `withdrawal.rs` is a reading with tests rather than a predicate.
- It did not read the subclauses a row does not cite. `coverage`'s `Container`, `Scoping`,
  `Restated` and `StatesNoRequirement` subclauses have no row, so nothing there can produce a
  finding for the exemption to withdraw; the audit says so by leaving them out, and a test refuses
  an entry for one.
