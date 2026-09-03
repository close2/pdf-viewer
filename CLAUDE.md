# Project: PDF Viewer

A PDF viewer in Rust, targeting Acrobat-class fidelity, with the goal of being the
noticeably fastest PDF viewer available — and clean enough to be taught from.

> **New here?** Read `doc/HANDOVER.md` after this file. It is an index: which file this round
> opens, and which of `doc/traps/`'s five groups this round is in a position to spring.
> `doc/state-of-play.md` is what the program already does.

**This file holds the principles and nothing that a command can print.** A fact that can be
counted is not written down here; what is written down is the command that counts it, and
`tools/state.sh` is that command. Where a number appears below it is the standard's, not
ours.

| moved out of this file | now in | opened when |
|---|---|---|
| the stack table, and why `rustybuzz` is not in it | [`doc/stack.md`](doc/stack.md) | you are choosing or questioning a dependency |
| the working agreements, the machine, the agent user, the display | [`doc/environment.md`](doc/environment.md) | you are about to run something |

## Non-negotiable principles

These are stated by the project owner and override convenience, velocity, and any
default habit. When a suggestion conflicts with one of these, say so explicitly rather
than quietly compromising.

### 1. Quality first — no shortcuts

- No placeholder implementations, no `todo!()` left in merged code, no "we'll fix it
  later" paths. If something cannot be done properly now, it is not started now.
- No silent error swallowing. Every error is typed, propagated, and handled somewhere
  deliberate. No `unwrap()` outside tests and provably-infallible cases (and then with a
  comment naming why it cannot fail).
- Every public item documented, with `#![warn(missing_docs)]` enforced.
- `clippy::pedantic` clean. Warnings are errors in CI.
- If a shortcut is genuinely the right call, it is documented as a deliberate decision
  with its cost written down — never taken silently.

### 2. Fast — including startup

- Performance is a feature owned from day one, not an optimization phase later.
- Parallelism (rayon) and GPU offload (vello/wgpu) are used wherever they genuinely help.
- "Genuinely" is decided by measurement, never by assumption. For an interactive viewer,
  *latency* usually matters more than throughput — a parallel path that improves
  throughput while worsening time-to-first-page is a regression.
- Perf gates run in CI: cold open, time-to-first-page, page-turn latency, memory
  high-water. A regression fails the build.
- Remember where the time actually goes: parsing, xref resolution, and font loading
  usually dominate time-to-first-page — not rasterization.

#### Startup time is a first-class requirement

Launch latency is what a user judges the program by before it has rendered anything, and
it is the easiest thing to lose gradually to unnoticed initialisation. Rules:

- **Nothing eager.** No system font enumeration, no full page-tree walk, no
  configuration or recent-file scanning, no thumbnail generation on the launch path.
  Anything not needed to show page one is deferred until first use (`OnceLock`, not
  startup).
- **No parsed data at startup.** The Arlington-generated tables are compiled-in `static`
  data, so the object model costs zero parse time at launch. Any future data resource
  follows the same rule.
- **Incremental parsing.** Opening a document reads the trailer and the objects page one
  needs — not the whole file. A 500-page document must open no slower than a 5-page one.
- **Page one goes to the GPU, and GPU bring-up is therefore on the critical path by
  choice.** Stated by the project owner: drawing the first page on the graphics device is
  what is wanted, so the alternative — showing page one from the processor while the
  device initialises on another thread — is *not* the rule here. What follows from that
  choice is an obligation rather than a licence: **creating the device and compiling the
  pipelines is now part of time-to-first-page, so it is a number to measure and to keep
  small.** Tens to hundreds of milliseconds of it are the driver's, which is exactly why
  it may not be left unmeasured.

  Three things follow, and they are requirements:

  - **The graphics library must return a usable device before it is warm.** Shaders
    compile in the background and the first frames go through whatever is ready; a
    library that blocks until every pipeline exists has put the driver's worst case in
    front of the first page.
  - **Nothing on the launch path waits for warmth.** No `wait_until_warm` before the
    first present, no probe frame, no pipeline pre-compilation "to be safe".
  - **Cold bring-up is its own gate**, separate from time-to-first-page, so that a
    regression in the driver, the adapter selection or the shader set is legible as
    itself rather than as a slower page.

  **The CPU backend keeps its other two jobs and loses this one.** It is still the
  correctness oracle — the whole cross-backend comparison rests on it — and it is still
  what draws a frame the graphics device refuses (a coverage or budget refusal, not a
  swapchain state), reported out loud. What it is no longer is the startup path.
- **No heavy runtime.** No async runtime unless something genuinely requires one; a
  thread pool for rasterisation is not a reason to pull in one.
- Cold-start and time-to-first-page are CI gates with numbers attached, measured with a
  cold page cache. Targets are set once Spike A gives a real baseline, rather than
  invented now.

### 3. Secure from the start

- Untrusted input never reaches unsafe code. `#![forbid(unsafe_code)]` on every crate
  that touches PDF bytes; this is compiler-enforced, not a convention.
- Multi-process sandbox: renderer runs unprivileged under seccomp-BPF + Landlock, with no
  filesystem and no network access.
- Memory safety is not enough. Explicit memory and time budgets guard against
  decompression bombs, xref cycles, and pathological content — Rust does not prevent
  resource exhaustion.
- Fuzzing from the first parser commit. Every crasher found becomes a permanent
  regression test.
- Any C dependency (notably JBIG2 / JPEG2000, both historically severe attack surfaces)
  must be isolated in the sandboxed process and justified in writing.

#### A document's restrictions are the reader's to set, and they have levels

Stated by the project owner. This is about the permissions a *document* asserts over the
person reading it — Table 22's `/P` flags, §12.8.2.2's `/DocMDP`, §12.8.6's usage rights —
and not about the sandbox, which is the opposite direction and is not negotiable.

- **They are low priority**, and **it shall always be possible to turn them off.** A
  restriction a reader cannot switch off is a restriction imposed on the reader by somebody
  else's file, and this program is the reader's.
- **The finished product has four levels**: `off`, `on`, *ask before the operation*, and
  *warn before the operation*. There is no user interface for them yet and none is to be
  built now.
- **What binds today is the shape rather than the feature.** Whenever a restriction is
  implemented or touched for any reason, it is written so that those four levels can be
  added later without revisiting the decision: the *policy* is asked, once, in a place a
  host can supply — not hard-coded as a refusal at the point of the operation, and not
  decided inside `pdf-model` where no host can reach it. A refusal that cannot become an
  "ask" is the thing to avoid.

### 4. Exemplary — a project others can learn from

The aim is for this codebase to be worth showing to students as an example of how to
build an application cleanly.

- Architecture is legible: clear layer boundaries, no circular dependencies, each crate
  with one stated responsibility.
- Names say what things are. Comments say *why*, never *what*.
- Every non-obvious decision gets an ADR in `doc/adr/` — the reasoning matters as much as
  the result.
- Prefer the clear construction over the clever one.

### 5. The specification is the only source of truth

Stated by the project owner, and absolute:

> Never use the other libraries as source of truth. The truth is the spec only. If we have
> the same results as the other libraries, we can assume that we understood the spec
> correctly — but if not, we don't try to match what the others do, we find out what the
> spec says.

poppler, mupdf, ghostscript, pdf.js and Acrobat are **evidence about our reading**, never
the definition of correct. The direction of inference runs one way: agreement raises
confidence that we understood the specification; disagreement is a question to take back to
the specification, never a target to move toward.

In practice:

- A test's expected value must be derivable from the specification, and its comment must
  say *from where*. "This is what poppler produces" is not a justification for anything.
- When another implementation disagrees, find the clause. Either we misread it, or they
  did, or it is genuinely unspecified — and those three have different consequences.
- Where the specification genuinely defines nothing (a `Text` annotation's icon is the
  standing example: §12.5.6.4 requires "predefined icon appearances for at least the
  following standard names" and states not one line of their artwork), say so plainly, make
  a deliberate choice, and document it *as a choice*. Presenting a de-facto convention as
  though it were derived is the failure mode this rule exists to prevent.
- **"The specification defines nothing here" is itself a claim about the specification, and
  it decays.** The standing example used to be `DeviceCMYK` → RGB, on the evidence of
  §8.6.4.4, which says only that the components are "concentrations of process colourants".
  §10.4.2.5 defines that conversion outright, and §10.4.2.1 ranks it below §10.3's ICC route
  — so the question is answered twice, in an order, and the code was right for a reason
  nobody had found. Before recording a silence, read the *titles* around the subject in
  `doc/md/`; it takes a minute, and that claim survived thirty-two sessions in this file.
- Curve-fitting to another renderer's output is forbidden outright. An implementation is
  correct for all valid PDFs or it is not correct; tuning constants until a corpus matches
  produces neither correctness nor knowledge.

#### What *done* means

**Every PDF that exists renders as its producer specified.** That is the target. It follows
from "Acrobat-class fidelity" at the top of this file, not from anything the standard
permits us to skip.

Scope is therefore stated the other way round from the usual: **everything is in scope
except a closed list of exclusions, decided once, each with a reason.** A clause may not be
declared out of scope after the fact because it turned out to be hard — that is the
corpus-going-quiet failure wearing better clothes.

In scope, without exclusions:

- **Clauses 7, 8, 9 and 11** — syntax, graphics, text, transparency. These decide whether a
  page is correct. Complete means complete, including encryption and every filter.
- **Clause 12, everything that displays** — every annotation subtype's appearance including
  synthesised ones, form field appearance construction, optional content, outlines,
  destinations, page labels.
- **What a reader does with an open document**: selecting its text, filling a field, adding an
  annotation or a markup. A viewer that can only look is not the target and never was; the
  exclusion it amends is below.
- **Clause 10 where it applies to a screen**, and the standard decides which clauses those are
  rather than this file. **This entry used to say "[h]alftones and transfer functions describe a
  marking device; those are *inapplicable*", and it was wrong about the second of the two** —
  found on a corpus document that draws wrong because of it, on evidence the standard states
  three times over. So **§10.6's halftones are inapplicable on the standard's own condition**, and
  **§10.5's transfer functions are in scope**: they decide what a screen shows. Flatness and
  smoothness are **not** inapplicable either, and the difference is worth the words: §10.7.2 makes
  ignoring flatness an explicit permission, and §10.7.3 says "each output device may have internal
  limits" on smoothness — a clause that permits is a clause that has been read, and it is a
  stronger answer than one that does not apply. The reading itself, with the standard's own words
  under each clause number, is `doc/todo/13-the-transfer-function.md` and §10.5's ledger row.

  **The lesson is more general than the clause**, and it is the project owner's: the restrictions
  in this file were drawn early, around the most important functionality, and they come off step
  by step as each is read against the standard rather than staying because they are written down.
  An entry here that says a clause does not apply is a *claim about the specification*, and it
  decays exactly the way a ledger row's does.
- **Clause 14** — output intents, and tagged PDF as far as accessibility needs it.
- **The normative annexes**, which this list did not mention for a long time and which were
  therefore in scope all along with nothing looking at them. The normative ones are **D**
  (character sets and encodings), **E** (extending PDF), **F** (linearised PDF), **I** (versions
  and compatibility), **K** (XFA), **L** (structure element nesting), **O** (fragment identifiers)
  and **Q** (a method for determining transparency) — and the ledger carries a row for each because
  a requirement is a requirement wherever the standard prints it. The rest say *informative* on
  their own title line and state nothing. Annex O is the reason this entry is not a formality: its
  `shall`s are addressed to "the PDF processor", and when this entry was written not one of them
  was implemented and not one reported. `tools/state.sh annex-o` counts where they stand;
  `doc/todo/39` and ADR 0209 say what each of the remainder needs.

The exclusions, closed, each with its reason:

- **Clause 13, multimedia and 3D** — a media engine, not a rendering question.
- **XFA** — the standard hands this one over itself: "[t]he implementation of such a schema
  driven page generation involves considerable effort beyond that for a simple PDF viewer and
  therefore **a PDF processor may choose to not implement this feature**" (§K.1). **This entry
  used to read "deprecated by ISO 32000-2 itself and specified outside it", and the second half
  was wrong**: Annex K is normative and it is *in* ISO 32000-2. What is documented separately is
  the XFA template architecture, not the annex — and the exclusion is stronger for resting on a
  permission the standard grants than on a claim about where it was printed. Annex K also says
  what makes declining it safe, which is worth knowing rather than assuming: in a conforming
  hybrid file "[t]he other entries in the interactive form dictionary shall be consistent with
  the information in the XFA resource", so the AcroForm this tree reads *is* the form.
- **JavaScript and script-driven form behaviour** — a sandboxed script engine is a separate
  project with its own security argument. Field *appearance* is not excluded; field
  *behaviour* is.
- **Authoring content from nothing** — we do not compose pages: no layout engine, no
  text-setting, no chart drawing, no "HTML to PDF". No clause whose subject is deciding what
  marks a page should contain falls on this project.

  **Assembling documents from existing documents is in scope.** Splitting, merging, reordering,
  rotating, extracting and optimising operate on content some producer already specified; every
  content stream in their output is a producer's, carried byte for byte or recompressed without
  reinterpretation. The writer this requires is §10 of RFC 0002's serializer: it emits
  structure (object table, streams containers, cross-reference, trailer, identifiers), never
  content. §7.5.6's incremental update remains the only writing that touches a file a user is
  *editing in place*; the serializer is how a *new* file is derived from old ones.

  Generator obligations come into scope only where the serializer actually emits the construct:
  §7.5.4/§7.5.5/§7.5.7/§7.5.8 on the way out, §14.4, §7.6 encryption on the way out. Annex F
  stays excluded until linearisation is separately ratified. The ledger's `writer-side` status
  narrows accordingly.

  The boundary line that keeps the exclusion enforceable: **does the operation invent marks?**
  Rotate does not (it writes an integer the producer's renderer already honours); a watermark
  stamp *does* (it composes new content over pages), which is why qpdf's `--overlay`/`--underlay`
  and Stirling's watermarking are **deliberately not in this suite** despite being conventional —
  they are the first feature on the far side of the redrawn line, and taking them later must be
  its own argued amendment, not scope creep. Variable-text field appearances
  (`crates/pdf-model/src/variable_text.rs`) already sit on this line today, sanctioned by
  §12.7.4.3's own requirement; the line is where it always was, now written down.

  **This exclusion read "we do not create files", then "we do not *create* PDFs", and has been
  amended twice — both times by argument rather than by attrition** (the second on 2026-09-03,
  when the owner ratified RFC 0002 §11.1 with "RFC 002 and 003 are approved"). What a *user* does
  to a document already open — an annotation added, a field filled — is not authoring, and it is
  written back by §7.5.6's incremental update: the new objects and a new cross-reference section
  appended, never a rewrite of what was there. The
  producer's bytes stay in the file, byte for byte, under whatever the user added. This tree
  already reads that construction (ADR 0100), which is why it is the one form of writing it is
  placed to get right.

  **`pdf_syntax::Document` stays immutable, and that is not a style preference.** An edit is a
  log beside the document rather than a change to it — the pattern `view.rs` already uses for
  §12.6.4's actions — so `interpret` remains a pure function of what the file says, what the
  viewer state is, and what the user did. The oracle's whole comparison rests on the first of
  those three being a function of the bytes alone, and an editor that mutated the document
  would cost it silently.

Revisit an exclusion by argument, never by attrition.

One honest limit on "as its producer specified": where the standard defines nothing — an
annotation icon's artwork, how a fractional page becomes a whole number of pixels — done
means a documented choice, not a match with anyone.

**ISO's own conformance clause is the floor, not the goal.** §6.3.2.1 lets any processor
choose which subsets it supports; that is what the standard owes a thumbnail generator, and
adopting it as our definition of done would hand the project an escape hatch. It stays useful
for one thing — **ranking**. §6.3.2.2 places three obligations on a rendering processor:
render the page contents as defined, respect the default or user-specified optional content
configuration (§8.11), and draw the appearance stream of every annotation whose flags call
for one (§12.5.3, §12.5.5). Where that ordering disagrees with the corpus's, the
specification's wins.

#### Two questions, two denominators

"Every PDF renders correctly" contains two claims, and conflating them is how a project comes
to treat a corpus as a specification:

| | asks | denominator | instrument |
|---|---|---|---|
| **Coverage** | which of the standard's requirements are implemented | the specification | the conformance ledger |
| **Robustness** | what share of the files that actually exist render correctly | the world | corpus + oracle |

Real files are malformed, truncated and written by twenty-year-old generators; the standard
describes *valid* files and says nothing about the rest. The corpus and the oracle are not
demoted by any of the above — they are the only instrument for the second question, and no
amount of clause coverage answers it. Equally, a corpus cannot rank a requirement no document
exercises, and it declares success the moment the last file goes green.

**Work is chosen from both, never one.** A session that ships code while leaving the clauses
it touched `unreviewed` has answered only the robustness question.

#### The specification appears in the code

- **Every item implementing a normative requirement cites its clause** — `ISO 32000-2
  §9.6.5.4` — in its doc comment, its module comment, or the comment above the block.
- **Quotation marks mean verbatim.** A load-bearing normative sentence goes in as a rustdoc
  blockquote, exact, under its clause number, so that the conformance checker can verify it
  against `doc/md/`. Anything less than verbatim is prose *without* quotation marks:
  paraphrase is fine and often clearer, paraphrase that claims to be a quote is not.
- **Quote the rule, cite the rest.** The load-bearing sentence, or an algorithm's steps, sit
  beside the code; the clause number carries the remainder. Copying the standard wholesale
  would drift from `doc/md/` and would bury the *why* comments principle 4 asks for.
- **Every clause a change touches leaves the conformance ledger non-`unreviewed`.** This
  binds new and modified code. The existing tree is retrofitted clause-family by
  clause-family as work reaches it, never as a separate marathon.

The ledger's format, statuses and checker are infrastructure and live in `doc/PLAN.md` §5a.

### On the tension between 2 and 4

The project owner has noted that speed and exemplary clarity partly conflict. They
conflict less than they appear to, and the resolution is a rule:

**An optimization must be justified by a benchmark and explained by a comment.**

Clean architecture at the top, optimized code in measured hot spots, with every
optimization carrying (a) the benchmark number that justifies it and (b) a comment
explaining what it buys and what it costs in readability. Optimized code that is
*explained* teaches more than naive code does — a student learns both the technique and
the discipline of proving it was needed. What is forbidden is unexplained cleverness and
speculative optimization of code nobody measured.

Where the conflict is real and unresolvable, clarity wins in cold paths, speed wins in
hot paths, and the choice is written down.

## Where knowledge lives

A rule about these documents, and it is the project owner's:

> **A fact that can be counted is not written down. What is written down is the command that
> counts it.**

It binds *derived* facts — counts, rates, gate results, "N of M", session numbers, dates. It
does not bind, at all, the things no command can produce: the principles above, the argument
for a decision, a trap, a clause reading. Those are the project's memory and they are why this
works; deleting one is the only unrecoverable mistake a round can make.

The sharpest consequence: **a round asked to measure something must not be able to read the
answer in a document.** A table of gate numbers is what lets a round write "unchanged" without
running anything. So the numbers are not in the instruction files — `tools/state.sh` prints
them — and session bookkeeping is one file per round under `doc/history/`, which no round reads to do its
work.
ADR 0281 has the argument, ADR 0232 its predecessor.
