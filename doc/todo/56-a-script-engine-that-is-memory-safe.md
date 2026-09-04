# A script engine that is memory-safe, and the exclusion it invites the owner to re-read

Status: **blocked on the owner's decision about the exclusion, and on nothing else.** The premise
this file was commissioned to test — *is it true that there is now a safe ECMAScript library?* — is
**true, with one qualification that matters and is stated in full below**. What is not settled is
whether the project wants the capability; `CLAUDE.md`'s exclusion list still closes it, and an
exclusion is amended by the owner's argument, never by a round's.
Priority: 56 — band 50–59, *blocked on a decision*. Not 20–29, though the corpus demand is there:
no round may start this while the exclusion stands.

**One of the two open questions this file left is now closed, and it is not the exclusion.** The
owner has ruled on *where the host object model is read from* — §3's *The source, settled by the
owner* below, which answers what §9's step 4 used to ask for and rewrites it. The exclusion itself
is untouched: §8 is still an argument awaiting ratification, with the four claims that cut against
it still standing, and the round that recorded the source decision changed no ledger status, no
`Cargo.toml`, no file under `doc/rfc/` and no line of `CLAUDE.md`.

Corpus: **run the census rather than reading a number here.** The instrument already existed
before this file did — `refused_action_census` walks every object the cross-reference table lists
and every dictionary inside one, so it sees a script action inside an object stream, which a
byte-level `grep` cannot:

```sh
find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' > /tmp/paths-56
cargo run --release -p pdf-model --example refused_action_census -- @/tmp/paths-56

# and for the entries that are not actions — /AA, /CO, the /JavaScript name tree —
# the objects layer, which sees a name inside a §7.5.7 object stream:
cargo run --release -p pdf-model --example witness_census -- --pdfjs JavaScript AA CO XFA AcroForm
cargo run --release -p pdf-model --example witness_census -- --crawl JavaScript AA CO XFA AcroForm
```

**`find -L`, and the `-L` is not decoration**: in a parallel worktree `corpus-cache` and
`doc/pdf.js` are symlinks into the main checkout, and `find` without it reports zero paths.

Clauses: **§12.6.4.17** (ECMAScript actions, Table 221), §12.6.2, **§12.6.3** (Table 197 an
annotation's additional-actions, Table 198 a page object's, **Table 199 a form field's**, Table 200
the document catalog's), §12.6.4 (Table 201), §7.7.4 (Table 32's `/JavaScript` name tree), §12.7.3
(**Table 224's `/CO`**), §12.7.4.1, §12.7.8.3.1 (Table 246's `/JavaScript`), §12.11.5,
§12.5.6.19. And the normative reference the standard makes for all of it: **ISO 21757-1:2020**,
*Document management — ECMAScript for PDF — Part 1: Use of ISO 32000-2 (PDF 2.0)*.

Code as it stands: `crates/pdf-model/src/action.rs` (the refusal), `crates/pdf-model/src/view.rs`
(`ViewState::perform_all`, the action log beside the document), `crates/pdf-model/src/forms_data.rs`,
`crates/pdf-model/src/requirements.rs`, `crates/pdf-sandbox/src/lockdown_linux.rs` (the budgets),
`crates/viewer-confined/`, `crates/pdf-model/examples/refused_action_census.rs` (the instrument).

---

## 1. The commission, and the honest answer

The project owner read that a safe ECMAScript library now exists and asked what it would mean.

**It is true.** As of 2026-08-28 there are at least three ECMAScript engines written in Rust that
are actively developed, and the leading one is *more* conformant than the C engine everybody
embeds. The qualification, which no round should let past: **"written in Rust" is not
`#![forbid(unsafe_code)]`, and for a garbage-collected language it never will be.** Every engine
below allocates a moving or tracing heap, and every one of them writes that heap in `unsafe`.
`CLAUDE.md` principle 3's compiler-enforced rule — `#![forbid(unsafe_code)]` on every crate that
touches PDF bytes — **cannot be met by any ECMAScript engine that exists**, and a round that files
this as "safe, so the principle is satisfied" has misread both the marketing and the principle.

What *is* true, and is the finding worth having, is that the **class** of failure changes. The
evidence is a single specification bug that both worlds hit:

- **CVE-2024-43357** (published 2024-08-15, CVSS 3.1 base 8.6 HIGH) is not a bug in any
  implementation. NVD's own text: it is "[a] problem in the ECMAScript (JavaScript) specification
  of async generators, introduced by a May 2021 spec refactor, [that] may lead to mis-implementation
  in a way that could present as a security vulnerability, such as **type confusion**".
- Boa's manifestation of it is **RUSTSEC-2024-0444 / CVE-2024-43367**, categorised
  `denial-of-service`: "a carefully constructed code could trigger a state transition from a getter
  method for the promise's `then` property, which causes the engine to fail an assertion of this
  assumption, causing an uncaught exception."

Same specification defect. In an unsafe language its ceiling is type confusion; in Boa its ceiling
is a panic. That is exactly the trade this project already makes everywhere else, and it is the
whole of the "is it true" answer: **not memory-safe in the sense the principle states, but
memory-safe in the sense that decides what an attacker gets.**

Contrast the C option over the same window. quickjs-ng carries **CVE-2024-13903** (stack buffer
overflow, published 2025-03-21), **CVE-2026-0821**, **CVE-2026-0822** (heap buffer overflow in
`js_typed_array_sort`, published 2026-01-10) and **CVE-2026-1145** (heap buffer overflow in
`js_typed_array_constructor_ta`, published 2026-01-19) — each reachable, in NVD's words, from
"specially crafted JavaScript input", which is precisely what a hostile PDF supplies. Boa's whole
advisory history in the same database is the one entry above.

Sources, with the dates they were read (2026-08-28): NVD
<https://nvd.nist.gov/vuln/detail/CVE-2024-43357>, <https://nvd.nist.gov/vuln/detail/CVE-2026-0822>,
<https://nvd.nist.gov/vuln/detail/CVE-2026-1145>, <https://nvd.nist.gov/vuln/detail/CVE-2024-13903>;
RustSec <https://github.com/RustSec/advisory-db/blob/main/crates/boa_engine/RUSTSEC-2024-0444.md>.

## 2. The engines, graded on the axes this project has

Every figure in this section was **measured on 2026-08-28** by cloning the repository and counting,
or read off a dated public instrument. Nothing here is a marketing sentence.

### 2.1 Conformance — test262, independently run and dated

`test262.fyi` runs the suite daily against many engines and publishes the raw counts at
`https://data.test262.fyi/index.json` with the engine revisions at `.../meta.json`. The run read
here was generated **2026-08-28T01:43:02Z** against test262 revision `d86b229`, total **53 578**
tests:

| engine | language | passing | rate | revision under test |
|---|---|---|---|---|
| JavaScriptCore | C++ | 52 927 | 98.78% | 320000 |
| SpiderMonkey | C++ | 52 740 | 98.44% | 157.0a1 |
| LibJS (Ladybird) | C++ | 52 333 | 97.68% | e343030 |
| **V8** | **C++** | **52 285** | **97.59%** | 15.4.67 |
| **Boa** | **Rust** | **51 012** | **95.21%** | ca7c8fb |
| Kiesel | **Zig** | 50 564 | 94.37% | 0.4.0-dev.34 |
| quickjs-ng | C | 44 745 | 83.51% | 5cbbc67 |
| **QuickJS (Bellard)** | **C** | **44 006** | **82.13%** | 04be246 |
| XS (Moddable) | C | 43 605 | 81.39% | b1f42a2 |
| **Nova** | **Rust** | **41 234** | **76.96%** | 4eea7c6 |

Brimstone is not in that run; its own README claims ">97% of the ECMAScript language in test262"
and is not independently dated, so it is recorded here as a claim rather than a measurement.

**The headline is the ordering, not the percentages.** Boa is 13 points above QuickJS — the engine
pdf.js's own sandbox is built from. The historical reason for reaching for C here (the only small
embeddable engine is a C one) has stopped being true.

**And for this application the ordering barely matters.** A PDF's scripts are ES3/ES5-shaped
expressions over a host object model: `AFNumber_Format`, a `getField().value` assignment, a
comparison. Nothing in ISO 21757-1's API is expressed in terms of generators, `Proxy`, `Temporal`
or `SharedArrayBuffer`, which is where the last five points of test262 live. **The conformance axis
is the one this project cares *least* about**, which is worth saying plainly because it is the axis
the marketing is written on.

### 2.2 Memory safety, counted rather than claimed

Shallow clone, `grep -rn --include=*.rs -w unsafe`, excluding `tests/` directories:

| engine | HEAD read | Rust lines | `unsafe` occurrences | `forbid`/`deny(unsafe_code)` | where the `unsafe` is |
|---|---|---|---|---|---|
| **Boa** | `337a3668`, 2026-08-28 | 242 401 | 623 (373 blocks, 124 `fn`, 89 `impl`, 2 `trait`) | **0** | 44 files in `core/engine`, **12 in `core/gc` (180 occurrences)**, 8 in `core/string` |
| **Nova** | `4eea7c6f`, 2026-06-13 | 164 197 | 1 155 (1 055 blocks) | **0** | the data-oriented arena heap |
| **Brimstone** | `e1a61566`, 2026-08-23 | 124 454 | 286 blocks | **0** | `src/js/runtime` (41 files) |
| **rquickjs** | — | bindings | 640 in `core/src` alone | **0** | it is an FFI binding; the engine is ~80 kloc of C |
| **v8 crate** | — | bindings | — | **0** | it is V8: a JIT, in C++ |

Brimstone's README states the position with a candour the others do not: its garbage collector is
"[a] compacting garbage collector, written in *very* unsafe Rust". That is the honest shape of all
three.

**So the ranking on this axis is: Boa < Brimstone < Nova by count, and all three are categorically
apart from rquickjs and the v8 crate**, which are not Rust engines at all but Rust *interfaces to*
a C and a C++ engine. For a project whose principle 3 says "[a]ny C dependency … must be isolated
in the sandboxed process and justified in writing", the last two are not shortcuts — they are the
thing the principle is about.

### 2.3 Maintenance and provenance

| | Boa | Nova | Brimstone | rquickjs | v8 crate |
|---|---|---|---|---|---|
| first commit | 2018-08-08 | 2023 | 2024 | — | — |
| latest release | **v0.22.0 prepared 2026-08-28**; v0.21.1 on crates.io 2026-03-29; v0.21 2025-10-21; v0.20 2024-12-05 | `nova_vm` 1.0.0, 2026-03-15 | none — version `0.1.0`, no tag | 0.12.2, 2026-07-27 | 152.2.0, 2026-08-20 |
| commits, last 52 weeks | **513** | active | active | — | — |
| stars / forks | 7 506 / 655 | — | — | — | — |
| CI | `rust.yml`, `test262.yml`, `test262_release.yml`, `codeql.yml`, **`security_audit.yml`**, nightly builds | — | — | — | — |
| fuzzing | own targets under `tests/fuzz`; **not in OSS-Fuzz** | — | own harness | — | OSS-Fuzz (via Chromium) |
| steward | `boa-dev`, funded by OpenCollective; no corporate owner | `trynova` | one author | one author | **Deno Land** |
| licence | MIT / Unlicense | — | — | — | — |
| self-description | **"experimental"**, in its own README | — | **"Not ready for use in production"** | — | shipped |

Boa's `temporal_rs` is used by V8 and by Kiesel for their `Temporal` implementations — a component
of Boa is in production inside Chrome, which is a maintenance signal of a different kind than a
release cadence.

**Two things to hold against the tempting reading.** Boa calls itself experimental and it is the
only one of the Rust three that does not disqualify itself outright. And a release cadence of two
to four a year on a codebase of a quarter of a million lines, funded by donations, with one
security advisory and no OSS-Fuzz seat, is a *smaller* maintenance base than every other dependency
this project has taken. `doc/stack.md`'s question — what is this dependency in a position to break
— has an unusually large answer here.

Sources: <https://github.com/boa-dev/boa>, <https://github.com/trynova/nova>,
<https://github.com/Hans-Halverson/brimstone>, <https://github.com/DelSkayn/rquickjs>,
crates.io API, all read 2026-08-28.

### 2.4 Resource containment — the axis that decides this

`CLAUDE.md` principle 3: "Memory safety is not enough. Explicit memory and time budgets guard
against decompression bombs, xref cycles, and pathological content — Rust does not prevent resource
exhaustion." **This is where the Rust engines are weakest and it is the finding that shapes the
recommendation.**

| mechanism | Boa | rquickjs / QuickJS | v8 crate |
|---|---|---|---|
| hard memory ceiling | **no** — `boa_gc`'s `GcConfig::threshold` (default 1 MiB) is a *collection trigger* that grows, not a cap | **yes**, `Runtime::set_memory_limit` | yes, isolate heap constraints |
| instruction / fuel budget | **`instructions_remaining` exists and is `#[cfg(feature = "fuzz")]`** — not available to a normal build | **yes**, `Runtime::set_interrupt_handler`, polled during execution | yes, `TerminateExecution` from another thread |
| stack ceiling | `RuntimeLimits::stack_size` (default 10 240 frames) | **yes**, `set_max_stack_size` | yes |
| recursion ceiling | `RuntimeLimits::recursion` (default 512) | yes | yes |
| loop-iteration ceiling | `RuntimeLimits::loop_iteration`, **default `u64::MAX`, i.e. off** | — | — |
| interruptible from another thread | **no** | yes, via the interrupt handler | yes |

Read the Boa column carefully. `RuntimeLimits` bounds *shapes* of runaway — recursion depth, stack
depth, and loop iterations if you turn the limit on. It does **not** bound work done inside a
built-in: `new Array(1e9).fill(0)`, a quadratic string concatenation, a catastrophic regular
expression. There is no wall-clock interrupt and no memory ceiling, and the one true fuel counter
is compiled out unless you enable a feature named `fuzz`.

**This is not the obstacle it looks like, and the reason is that this tree already solved it one
layer up.** `crates/pdf-sandbox/src/lockdown_linux.rs` puts `RLIMIT_AS` on a worker process — "so a
decompression bomb fails an allocation instead of taking the machine's memory" — with `RLIMIT_NOFILE`
and `RLIMIT_FSIZE` of zero beside it, under seccomp-BPF and Landlock; and `pdf_sandbox`'s channel
waits with "what is left of the budget", which is what makes `SandboxError::TimedOut` reachable at
all. **A process boundary is a memory budget and a time budget that no engine API has to provide**,
and it is enforced by the kernel rather than by the engine's own correctness — which is strictly
the stronger guarantee, because an engine that mis-accounts its own fuel has defeated its own
limit and cannot defeat `RLIMIT_AS`.

So the containment column is not "Boa fails and QuickJS passes". It is: **QuickJS can be contained
in-process and Boa cannot; this project does not want to contain anything in-process, and already
does not.**

## 3. What the standard actually requires

`doc/md/ISO_32000-2_sponsored_EC3.md`, §12.6.4.17, verbatim:

> Upon invocation of an ECMAScript action, a PDF processor shall execute a script that is written
> in the ECMAScript programming language.

and, on the document-level name tree:

> When the document is opened, all of the actions in this name tree shall be executed, defining
> ECMAScript functions for use by other scripts in the document.

Two `shall`s, both addressed to the PDF processor, in a clause of clause 12. `CLAUDE.md`'s in-scope
list says "**Clause 12, everything that displays**", and §12.6.4.17's own sentence says what a
script does: "[d]epending on the nature of the script, various interactive form fields in the
document may update their values or **change their visual appearances**." So this is not a clause
that sits outside what displays; it is a clause that reaches what displays through a field's value.

**The finding that changes the shape of the argument is the normative reference.** §12.6.4.17 does
not point at Adobe's *JavaScript for Acrobat API Reference*; it points at an ISO standard:

> JavaScript is referred to as ECMAScript throughout this document and is defined by ISO/DIS
> 21757-1.

and the front matter records the change as deliberate: "ISO/DIS 21757-1 replaces several Adobe,
ECMA and ISO publications related to ECMAScript in PDF 2.0". **`ISO/DIS` is what the 2020 text
printed while the document was a Draft International Standard; it was published as ISO 21757-1:2020
in December 2020, first edition** (<https://www.iso.org/standard/71559.html>, read 2026-08-28).

That matters for principle 5 more than for anything else. This file's commission anticipated that
"Adobe's JavaScript for Acrobat API is a *host object model* on top of the language — the half the
standard does not define". **It is defined, and by a normative reference of the standard this
project implements.** From the published document's own front matter (ISO 21757-1:2020, read from
the publicly available ISO preview, 2026-08-28):

> This document defines a set of ECMAScript object types for automating and interacting with PDF
> documents and the contents of such documents.

Its normative references for the language itself are **ISO/IEC 22275:2018** (the ECMAScript
Specification Suite) and ISO/IEC 22537:2006 — so "conformance to test262" is the right instrument
for the language half, and ISO 21757-1 is the source of truth for the host half. There is no point
in this design where another implementation has to be treated as the definition of correct.

**ISO 32000-2 refers to no other document for any of this, and that is checkable**: `grep` for
`ECMA-262`, `ECMA 262`, `JavaScript for Acrobat` and `32004` over `doc/md/` returns **nothing**. Its
one normative reference for the language and the API alike is the ISO 21757-1 line above. So the
division of labour is exact: **ISO 32000-2 specifies the *dispatch*** — where a script hangs, when
it fires, in what order — **and not one API name**; ISO 21757-1 specifies the API. An engine built
from ISO 32000-2 alone would execute correct ECMAScript against an empty host object graph, which is
the single sharpest reason this file's commission asked where the API is to be read from at all. The
owner has since answered that, and the answer and its cost are the three subsections at the end of
this clause.

The word counts say the same thing about where the work is: "ECMAScript" appears 95 times and
"JavaScript" 13, but the clusters are few — §12.6.4.17 (15), Table 246's FDF dictionary (14),
§12.6.3 (11), §12.11.2 (8), §12.11.5 (6) — and everything under clause 13 (18 occurrences) is
already excluded. **The in-scope surface is three clauses and six tables**: 197, 199, 200, 221, 224
and Table 32's name tree.

**And the security model is in the standard too**, which is the part this project should care about
most. ISO 21757-1:2020 clause 9, *Privileged versus non-privileged context*:

> Some ECMAScript methods, identified with a [Security] note, have security restrictions. These
> methods can be executed only in a privileged context, which includes console, batch and
> application initialization events. All other events (for example, page open and mouse-up events)
> are considered non-privileged.

That is the standard drawing the line this project would want drawn, at the same place: a script
that runs because a *document* was opened or a *widget* was clicked is non-privileged by the
standard's own definition. Clause 7, *Safe path*, is weaker and says so — "[d]evelopers of PDF
Processor software implementing ECMAScript support are **encouraged** to support the concept of a
safe path" — a permission-shaped clause, not a `shall`, and this project would go further than it
asks anyway.

### The `shall`s are not one clause's — §12.6.3 states four more, per field

§12.6.3 introduces four additional-actions dictionaries by name, and the tables are worth having in
front of the argument because **each of Table 199's four entries is a `shall` naming an ECMAScript
action outright**:

| table | dictionary | entries that name an ECMAScript action |
|---|---|---|
| **197** | an annotation's additional-actions | `/E` `/X` `/D` `/U` `/Fo` `/Bl` `/PO` `/PC` `/PV` `/PI` — actions of any type; this tree already **raises** four of the ten |
| **198** | a page object's additional-actions | `/O` `/C` |
| **199** | **a form field's** additional-actions | **`/K` `/F` `/V` `/C` — all four are ECMAScript actions by the table's own text** |
| **200** | the document catalog's additional-actions | `/WC` `/WS` `/DS` `/WP` `/DP` — five, all ECMAScript actions |

Table 199, verbatim on the two entries the first step in §7 is about:

> An ECMAScript action that shall be performed when the user modifies a character in a text field
> or combo box or modifies the selection in a scrollable list box. This action may check the added
> text for validity and reject or modify it.

> An ECMAScript action that shall be performed before the field is formatted to display its value.
> This action may modify the field's value before formatting.

and on the one that makes a form calculate:

> An ECMAScript action that shall be performed to recalculate the value of this field when that of
> another field changes. (The name C stands for "calculate." ) The order in which the document's
> fields are recalculated shall be defined by the CO entry in the interactive form dictionary (see
> 12.7.3, "Interactive form dictionary").

Table 224's `/CO` is the other half of that sentence, and its condition is worth reading: it is
"(Required if any fields in the document have additional- actions dictionaries containing a C
entry; PDF 1.3) An array of indirect references to field dictionaries with calculation actions,
defining the calculation order in which their values will be recalculated when the value of any
field changes".

**So the count of `shall`s the exclusion closes is not one but fourteen or so**, spread over five
tables — nine of which (Table 199's four, Table 200's five) name ECMAScript in the table cell
itself. **Table 200's five are all about saving, closing and printing**, and this project prints
nothing and writes only §7.5.6's incremental update, so they are the ones that would stay refused
longest for reasons that are this tree's rather than the standard's.

### The size of the other half, measured

ISO 21757-1:2020 is **253 pages of body**. Its structure:

- clauses 1–9: scope, notation, paths, safe path, privileged context — 3 pages;
- **clause 10, the 2D ECMAScript API: pages 3–192**, about **36 object types** — `Annotation`,
  `app`, `Bookmark`, `Certificate`, `color`, `collection`, `Data`,
  `Dialog`, **`Doc` (45 pages)**, `Error`, **`event`**, **`Field` (30 pages)**, `FullScreen`,
  `global`, `HostContainer`, `Icon`,
  `Link`, `Net`, `OCG`, `PrintParams`, `RDN`, `ReadStream`, `security`, `SecurityHandler`,
  `SecurityPolicy`, `SignatureInfo`, `SOAP`, `Span`, `Template`, `Thermometer`, `this`, `util`;
- **clauses 11–12, the 3D ECMAScript API: pages 193–252** — `Camera`, `Mesh`, `Scene`, the event
  handlers. **This falls squarely inside `CLAUDE.md`'s clause-13 exclusion** and is not in scope
  under any reading, which removes a quarter of the document at a stroke.

So the honest size of "the half the engine does not give you" is **about 190 pages of API, of which
a first useful subset is perhaps fifteen**: `event`, `Field`'s value and formatting properties,
`util`'s `printf`/`printd`/number-format helpers, and the `AF*` form-format functions. It is a large
piece of work and it is *specified* work, which is a different proposition from reverse-engineering.

For calibration on the world's side rather than the standard's — evidence about what a partial
implementation costs, never a target — pdf.js's `src/scripting_api/` is **5 660 lines of JavaScript**
across 18 files, and it runs them inside **QuickJS compiled to WebAssembly** with a string-only
`JSON.stringify` boundary to the host. Read at `doc/pdf.js` `2ea8820d`, 2026-07-26.

### The source, settled by the owner

The owner ruled on the question the paragraph above asks, and the ruling is recorded here in the
owner's own words rather than paraphrased:

> "we won't obtain ISO 21757-1:2020 (it costs something). The adobe javascript reference is good
> enough. ISO 21757-1:2020 directly standardized Adobe's existing Acrobat JavaScript object model
> into an ISO standard. The core objects you need in a viewer—such as Doc, Field, event, util,
> color, and app—are documented in depth in Adobe's JavaScript for Acrobat API Reference. There is
> https://github.com/pdf-association/pdf-issues should be treated as secondary reference."

Three decisions, and they are the owner's:

1. **ISO 21757-1:2020 will not be acquired.** It is a purchase, and the owner has declined it.
2. **Adobe's *JavaScript for Acrobat API Reference* is the working source for the host object
   model**, on the argument that ISO 21757-1 standardised Adobe's existing model directly.
3. **`https://github.com/pdf-association/pdf-issues` is a secondary reference.**

§9's step 4 asked a round to acquire the standard and called that step not optional. It is answered,
and it is rewritten there. What is *not* answered is anything in §8: the exclusion still stands.

### What principle 5 makes of a vendor document a standard adopted

This is the part worth the words, because principle 5's usual answer is the wrong one here and the
reason it is wrong is worth being able to state.

**The owner's argument, written out.** `CLAUDE.md` principle 5 demotes poppler, mupdf, ghostscript,
pdf.js and Acrobat to *evidence about our reading*, never the definition of correct. What puts them
in that position is where they sit: each is a program written **downstream** of ISO 32000-2, so what
it produces is another party's reading of the same text this project reads, and two readings
agreeing raises confidence in both. Adobe's *JavaScript for Acrobat API Reference* is not in that
position at all. ISO 21757-1:2020 was made **from** it: the vendor document is the standard's
*ancestor* rather than a competing reading of it, so consulting it is nearer to reading a draft of
the standard than to reading a competitor's source. The direction of derivation runs Adobe → ISO,
where principle 5's whole machinery is about inference running the other way.

**And a claim about a document is checkable, so it was checked.** ISO 21757-1's own table of
contents is Adobe's, heading for heading. From the publicly available preview (read 2026-08-28):
clause 10.5 `app`, 10.8 `color`, 10.13 `Doc`, **10.16 `event`** — with 10.16.2 "Event type/name
combinations", 10.16.3 "Document Event Processing", 10.16.4 "Form Event Processing" and 10.16.5
"event properties" — **10.17 `Field`**, with 10.17.2 "Field versus widget attributes" — and 10.37
`util`. Every one of those sub-headings is a heading Adobe's page prints, in Adobe's order. The
ancestry is not a story about the two documents; it is visible in their contents pages.

**Here is the standing limit, and it is exactly as load-bearing as the argument.**

1. **The derivation is visible and is nowhere *stated*.** Neither the Foreword nor the Introduction
   of the preview says the document derives from an Adobe publication. So *Adobe is the standard's
   ancestor* is **this project's inference from a structural match**, not a sentence either document
   prints, and it should be written down as an inference wherever it is relied on.
2. **Where the two diverge, this project cannot tell.** There is no copy of ISO 21757-1 in this tree
   and none is coming; the free preview stops at clause 10.2, which is before the first object type.
   Everything below that comes from Adobe alone, with no second side to compare it against.
3. **They demonstrably do diverge — and the secondary reference is what proves it**, which is the
   strongest single reason to have adopted it. `pdf-association/pdf-issues` records three (read
   2026-08-28):
   - **`console` is in Adobe's reference and absent from ISO 21757-1** — issue #744, open, filed
     2026-04-23, noting the object is used in a great many existing files, with nobody in the thread
     able to say whether the removal was deliberate.
   - **`ScreenAnnot`'s documentation is missing from ISO 21757-1** — issue #99, open — where Adobe's
     reference carries it at length.
   - **`XMLData` is referred to by ISO 21757-1 and defined nowhere in it** — issue #535, open — a
     gap on the ISO side rather than the Adobe one.

   Three measured divergences settle the question against the tempting reading: **the two documents
   are not interchangeable.** The ancestry argument earns Adobe's reference the *standing* of a
   primary working source. It does not make it the standard, and the first round that treats a
   sentence of it as an ISO requirement has made the mistake this subsection exists to prevent.

**So the rule, stated so that a later round cannot soften it by accident: every API detail taken
from Adobe's reference is a documented choice in `CLAUDE.md` principle 5's own sense — "say so
plainly, make a deliberate choice, and document it *as a choice*" — and never a derivation from ISO
32000-2.** What *is* derived from ISO 32000-2 is the **dispatch**: §12.6.4.17 and §12.6.3's tables
say where a script hangs, when it fires and in what order, and this clause has already quoted them.
Not one API name comes from there.

### How a round cites it, so that the instruments stay honest

`§` in this tree means *a clause of ISO 32000-2* and nothing else — that is what makes every one of
them checkable, and `tools/conformance/src/citation.rs` says so in the doc comment on its
`ForeignCitation`, which exists for the one failure that matters here: a *readable* citation of
something else, which "checks as ISO 32000-2's §5.2, which exists, so it passes in silence". Taking a
second and a third source therefore has an instrument cost, and this round priced it by running the
scanner rather than by reasoning about it (trap 13). Each line below was fed to
`conformance::citation::scan` and its verdict read off, with the clause index asked whether the
number it landed on exists:

| written as | what the checker records | verdict |
|---|---|---|
| `ISO 21757-1 §9` | a **foreign citation** of "ISO 21757-1" | **caught** — the gate fails, and the message teaches the spelling: write "ISO 21757-1 section N" |
| `ISO 21757-1:2020 §9` | a citation of **ISO 32000-2 §9**, which exists | **silent pass, onto the wrong standard** |
| `the JavaScript for Acrobat API Reference §12.5` | a citation of **ISO 32000-2 §12.5**, which exists | **silent pass, onto the wrong standard** |
| `ISO 21757-1 Table 113` in any comment | a reference to **ISO 32000-2's Table 113** — "Additional entries in Mac OS Roman encoding not in MacRomanEncoding" | **silently the wrong table** |

**The second row is the one to remember, because it is the spelling a round will reach for.** The
guard is `citation::another_document`, which recognises another document by an acronym followed by a
plain number — `RFC 3986 §5.2`, `ISO 15076-1 §6` — and `21757-1:2020` is not a plain number, so the
*year* defeats the guard. This file writes "ISO 21757-1:2020" a couple of dozen times, which is
exactly the string somebody will copy into a doc comment.

The shapes to write, therefore, and they are rules rather than preferences:

- **`ISO 21757-1 section 10.17`** — the word, never the sign, and never the year attached to a `§`.
  That is the spelling the checker's own failure message asks for.
- **`Adobe, JavaScript for Acrobat API Reference, "event properties" — event.rc`** — a title, a
  section name and a member, with no `§` anywhere near it. A `§` after a prose title is not caught
  at all, so nothing will tell you.
- **lower-case, as in `table 113 of ISO 21757-1`**, because `read_tables` matches the capitalised
  `Table ` and there is no way to tell it which standard a comment is about. Table 113 is a live
  example of a number the two standards use for unrelated things.
- **Never a rustdoc blockquote of Adobe's words.** This is the one place the instruments are already
  exactly right and it should be left that way: a blockquote is compared against `doc/md/`, and a
  blockquote with no `§` before it in the same comment is reported as unattributed — so Adobe's
  sentences fail either way, which is the gate correctly refusing to let a vendor document wear the
  standard's clothes. Quote Adobe in ordinary prose, with the source named on the same line.
- **Pin what was read**, which is about honesty rather than about the checker. Adobe's reference is a
  *floating* URL with no version in it (below), so "Adobe's reference says X" is not reproducible
  the way a clause number is. This tree already knows the answer — `doc/pdf.js` is cited at
  `2ea8820d`, Boa at `337a3668` in §2.2 — and the same applies here: cite the commit of
  `adobe/dc-acrobat-sdk-docs` that was read.

### Where Adobe's reference is, in what form, and what it holds

Read 2026-08-28, by fetching rather than by searching.

**The PDF edition is retired and the document is now an HTML page tree.**
`https://www.adobe.com/content/dam/acom/en/devnet/acrobat/pdfs/js_api_reference.pdf` answers 301 to
the SDK landing page; so does `js_developer_guide.pdf`. What is live is
<https://opensource.adobe.com/dc-acrobat-sdk-docs/library/jsapiref/index.html>, retitled *Acrobat
JavaScript API Reference*, and the body of it is **two pages**: `JS_API_AcroJS.html`, 2 727 020
bytes, holding every object type except `Doc`, and `doc.html`, 891 KB, holding `Doc`. Objects are
addressable by fragment — `#app`, `#color`, `#event`, `#field`, `#util`.

**It is floating rather than versioned, and there is no edition to cite.** No version number, no
version segment in the URL, and three signals put the content's vintage at Acrobat XI (2012):
"Changes Across Versions" stops there, the compatibility table stops at Acrobat 10.0 / JavaScript
1.8, and the page footer reads a 2023 site build date. Adobe's own canonical short link,
`https://www.adobe.com/go/acrobatsdk_jsapiref`, redirects to a **404**. That is the whole of why the
pinning rule above exists.

**It is MIT-licensed, which is better than the old PDF's terms and matters for quoting it.** The
site is published from <https://github.com/adobe/dc-acrobat-sdk-docs>, whose `LICENSE.md` is the MIT
text and covers "this software **and associated documentation files**"; the repository is public and
still being pushed to. The retired PDF and the 1999 ancestor technote carry Adobe's restrictive
informational-use notice instead, so a round should read the site rather than a mirror of the PDF.

**What it covers, for the six objects the owner named.** All six are there and all six are
documented at length: `Doc` (177 headings, including `getField`, `calculateNow`, `resetForm` and the
`calculate` and `dirty` properties); `Field` ("Field versus widget attributes" plus 47 properties
and 25 methods, including every property this file's §7 needs); `event` (the "Event type/name
combinations" table of 35 `Type`/`Name` pairs, "Document Event Processing", "Form event processing",
and the full property list); `util` (11 methods, with `printf`'s conversion specification and
`printd`'s pattern table given in full); `color` (the colour-array forms, the twelve named constants,
`convert` and `equal`); `app` (24 properties and 45 methods). **The owner's "documented in depth" is
accurate**, and §7 below turns it into a checkable list.

**What it does not cover, which is the half that decides the first step.**

1. **The `AF*` form-format functions are not in it.** Measured rather than assumed: a search for
   `AF[A-Za-z]*_[A-Za-z]+` over the whole of `JS_API_AcroJS.html` and `doc.html`, and over the
   JavaScript Developer Guide, returns **nothing at all**; the reference's object list runs
   `Alerter … XMLData` with no such entry. **ISO 21757-1 does not have them either** — its contents
   run `Annotation` … `util` with no form-format clause — so this is not a gap the standard would
   have closed.
2. **Adobe's only published statement about them is an argument table in a different book**, found
   through the site's own search index: the *Interapplication Communication* guide's Acrobat Forms
   Plug-In → `Field` → `SetJavaScriptAction`, under "Calculation script" and "Formatting scripts"
   (<https://opensource.adobe.com/dc-acrobat-sdk-docs/library/interapp/IAC_API_FormsIntro.html#setjavascriptaction>).
   It gives parameter menus and **no algorithm**: `AFSimple_Calculate(cFunction, cFields)` with
   `AVG SUM PRD MIN MAX`; `AFDate_Format(cFormat)` with fourteen enumerated format strings;
   `AFSpecial_Keystroke(psf)` with `0 = zip, 1 = zip+4, 2 = phone, 3 = SSN`;
   `AFNumber_Format(nDec, sepStyle, negStyle, currStyle, strCurrency, bCurrencyPrepend)` with
   `negStyle` enumerated 0–3 and `currStyle` marked as not used. It is also **internally sloppy** —
   it uses the name `sepStyle` for two different arguments, documents only two of that argument's
   values, and omits `AFRange_Validate`, `AFMergeChange`, `AFExtractNums`, `AFMakeNumber`,
   `AFParseDateEx` and every `*Ex` variant.
3. **The trigger firing order is published only as a picture.** The reference's "Form event
   processing" section is two sentences and an image, `_images/formsevent.png`. The order the diagram
   states, read off it: Mouse Enter → Mouse Down → Focus → Mouse Up → Keystroke (or Selection Change,
   list box only) → Validate → Calculate → Format → Blur, with Mouse Exit branching off, a self-loop
   on Keystroke and an edge from Validate back to it. Nothing in Adobe's prose states that order.
4. **Adobe contradicts itself on `Validate` and `event.rc`, in consecutive sentences**: the Validate
   entry says the event does not listen to the return code, and then that a return code of false
   makes the value invalid and leaves the field unchanged. Which of the two a reader should implement
   is not stated anywhere.
5. **Nothing says what happens when a trigger script throws.** Across roughly a hundred occurrences
   of "exception" and "throw" in the reference, not one answers whether the value commits, whether
   the remaining triggers run, or whether a `/CO` chain continues.
6. **Locale is undefined.** `util.printd`'s third numeric format is described as a localised string
   and an example; which locale, and the mapping, are unstated, as are the month names
   `AFDate_Format` produces. The only locale machinery actually specified is the XFA picture-clause
   extension, which defers to a document `CLAUDE.md` excludes.
7. **`Field.value`'s string-to-number conversion is undefined** beyond one worked example
   distinguishing it from `valueAsString`; and **`Calculate` re-entrancy** — a calculation script
   writing a field earlier in the `/CO` order — is not addressed.

Items 3 to 7 are the ones to hold on to: they are places where **the standard would not have helped
either**, so the decision to read Adobe rather than ISO costs nothing there. Item 1 is different and
is what re-prices §7.

### `pdf-association/pdf-issues`, and what this tree already reads of it

**What it is.** The PDF Association's public errata tracker, described by the repository itself as
"Industry-based resolutions for issues and errata reported against any PDF-related specification",
published at <https://pdf-issues.pdfa.org/> and licensed **CC-BY-4.0**. Anyone may file; blank
issues are disabled and the template demands a clause, paragraph or table number and a proposed
correction. Resolutions are reached in PDF Association Technical Working Groups, published to the
site, and **then** the issue is closed — so *closed* is the resolved state rather than the rejected
one, and `wontfix` is the label for "this produced no specification change".

**How it is organised — it is both a tracker and a set of files.** Under `docs/` there is one
directory per standard, named for its ISO number and year, and inside each one file per top-level
clause. The errata themselves are HTML `<ins>`/`<del>` runs carrying `data-issue` (the GitHub issue
number) and `data-iso` (`approved`, `submitted`, or absent for TWG-resolved), one edit per line by
the repository's own publication policy so that the files can be scripted. There is a directory for
ISO 21757-1 as well as one for ISO 32000-2.

**And this tree already reads its output under another name, which is the thing worth knowing before
adopting it.** `tools/spec-errata` reads the annotated PDFs in `doc/`, which are the PDF
Association's *errata collection* — the same resolutions delivered as annotations on the standard
itself — and every annotation carries the issue number. That is what the `#181`, `#293`, `#236` and
`#374` in `doc/errata-read.md`'s tables **are**: `pdf-association/pdf-issues` issue numbers, read by
this project since ADR 0252 without the repository ever being named. So the owner's "secondary
reference" is not a new source. It is the upstream of an instrument this tree has had for four
hundred rounds, named at last.

**Which makes the duplication question worth answering precisely, because there is one and it has a
clean boundary:**

- **For ISO 32000-2, do not adopt it as a second errata feed.** `spec-errata`'s population is the
  annotated PDFs, and the repository's `docs/32000-2-2020/` is the same resolutions in a different
  container. Two instruments over one population is how a project comes to believe the one that
  agrees with it, and `doc/errata-read.md` already records what happens when a single collection
  disagrees with *itself* (Table 161's two accepted annotations that cannot both be applied). The
  repository's **issue threads** are a different thing and are not duplicated by anything here: they
  carry the argument, which the annotation does not.
- **For ISO 21757-1 it is the only channel this project can have.** `spec-errata` reads `doc/*.pdf`,
  there is no annotated ISO 21757-1 among them, and after the owner's decision there never will be.
  The repository's ISO 21757-1 directory is small — a handful of files over clauses 2, 3, 10 and 12
  — and it is free, so it is a strict gain rather than an overlap.

**What it says about the JavaScript question, specifically. Three of its thirteen ISO 21757-1 issues
are load-bearing and one of them is the most useful page either source has:**

1. **Issue #185, closed and ISO-approved, pins the language version — and neither ISO document
   states it.** As published, ISO 21757-1's normative reference for the language is a dated
   reference to ISO/IEC 22275:2018, which itself refers to ECMA-262 *undatedly*: so the published
   standard specifies **no fixed ECMAScript version at all**. The erratum replaces that reference
   outright with **ECMA-262, 11th edition, June 2020 — the ECMAScript 2020 Language Specification**.
   That is the conformance target §2.1's test262 table should be read against, and it is a fact
   available from nowhere else this project can reach.
2. **Issue #70, closed and ISO-approved: E4X is deprecated in PDF 2.0** and the ISO/IEC 22537
   reference is deleted. So no ECMAScript-for-XML, decided rather than assumed.
3. **Issue #100, closed `wontfix`, is the record of a standards body declining to specify the layer
   real forms depend on.** The reporter enumerated some ninety identifiers that existing form
   documents call — the whole `AFNumber_Format` / `AFNumber_Keystroke` / `AFDate_Format` /
   `AFSimple_Calculate` / `AFPercent_Format` / `AFSpecial_Keystroke` / `AFRange_Validate` family,
   plus regular-expression and message-string constants — and reported that implementing the
   standardised API is not enough to make a large share of existing files work. Adobe's answer in
   the thread is that these are not JavaScript APIs but undocumented private methods of the Acrobat
   products, which Adobe has chosen not to document for standardisation; the working group closed it
   as no fix.

   **That is a documented decision, not an omission, and it is exactly `CLAUDE.md`'s two
   denominators pulling apart**: the specification denominator says these functions do not exist,
   and the world denominator says a large share of form documents call them. A project that reads
   only the first will build a script engine that runs and formats nothing.

Four more are open and are worth knowing about because they are places where **no answer exists
anywhere**: #744 (`console` unspecified), #535 (`XMLData` undefined), #270 (the `Runtime` clause
under-specified — how many runtimes a document has, which annotations attach to one), #99
(`ScreenAnnot` undocumented). Everything else in the ISO 21757-1 errata is 3D and rich-media
cosmetics and touches no form-field scripting. On the ISO 32000-2 side there is essentially nothing
JavaScript-specific: the only relevant edit is one correcting the `/JavaScript` name tree's value
type in Table 32 from name strings to strings, which is a typing fix rather than a scripting one.

## 4. What it would mean for compliance — measured, and smaller than it sounds

### 4.1 The ledger

`doc/conformance/ledger.toml` holds 875 rows. 113 are `out-of-scope`, and their `exclusion` tags
split:

```
   86  clause-13-multimedia
   23  writer-side
    3  xfa
    1  script-behaviour
```

**Exactly one row is out-of-scope because of the JavaScript exclusion**, and it is §12.6.4.17
itself. Its note, verbatim from the ledger:

> ECMAScript actions run a script. Principle 5's closed list excludes "JavaScript and script-driven
> form behaviour — a sandboxed script engine is a separate project with its own security argument".
> This is the row that exclusion was written for, and the one place in clause 12 where being
> unimplemented is a decision rather than a debt.

**That is the whole of the exclusion's visible cost, and it is the wrong place to look.** The
exclusion's actual cost is distributed across `partial` rows that name it as part of what they still
owe. Fourteen rows mention a script, ECMAScript, `/JS`, a calculation order or an additional-actions
dictionary; of those, these name the exclusion as a *debt* rather than in passing:

| row | status | what the exclusion costs it |
|---|---|---|
| §12.6.4.17 | `out-of-scope` | the clause entire |
| §7.7.4 | `partial` | Table 32's `/JavaScript` name tree, one of the three trees this row still owes |
| §12.6.4 | `partial` | one of the nine refused action types in the Table 201 summary |
| §12.7.3 | `partial` | Table 224's `/CO`, "which is §12.6.3's calculation order and needs the script engine principle 5 excludes" |
| §12.7.8.3.1 | `partial` | Table 246's `/JavaScript` in an FDF file |
| §12.11.5 | `out-of-scope` | the clause entire. **It was `partial` on the premise "[t]his program runs no ECMAScript (principle 5), so there is nothing to disable" until session 928**, which read the clause and found every requirement in it addressed to a processor that *invokes* a handler — Table 276 admits only `JS` and `NoOp` — so the exclusion covers it whole rather than leaving a debt (ADR 0896) |
| §12.6.3 | `partial` | Table 197–200's triggers are *read and raised*; what is missing is what an `/AA` entry's ECMAScript would do |

**§12.6.3's row is the highest-leverage of them and says so in its own words**, which is worth
quoting because it is this project telling itself what it decided:

> Tables 199 and 200 are excluded rather than owed: every entry of both is an ECMAScript action,
> which is on principle 5's closed exclusion list, and saying so is the difference between an
> exclusion and an oversight.

All ten of Table 197's events are *raised* already and Table 198's two are read. **The plumbing
exists and the payload is what is missing**, which is a much cheaper starting position than any
other refusal in this tree.

**Two rows would move the wrong way, and a proposal that hid that would be worth nothing.**
§12.11.1 and §12.11.2 are `implemented` and §12.11.5 is `out-of-scope` — `partial` when this was written, and moved in session 928 (ADR 0896) — *because* nothing runs. §12.11.1's
note says it outright: Table 273's `/RH` "is unread, and the requirement it carries is met by
construction rather than skipped … `CLAUDE.md` excludes ECMAScript, so every handler a file could
name is disabled here whatever the file says." Add an engine and that construction collapses: `/RH`
becomes an entry that must be found and evaluated, the `JS`/`NoOp` distinction becomes operative,
§12.11.2's `EnableJavaScripts` requirement becomes one this program meets rather than declines, and
§12.11.1's "[a] PDF processor that supports document requirements shall evaluate them before
execution of any ECMAScripts" becomes an **ordering obligation nothing in this tree schedules**.
**A capability that arrives turns two settled rows into debts**, which is `doc/habits.md`'s ledger
section from the other direction and is the kind of cost an amendment argument has to carry.

**Judgement, marked as judgement**: on a first step of the shape recommended in §7 below,
§12.6.4.17 would move `out-of-scope` → `partial` (the action executes; the API is a subset),
§12.7.3 would lose its `/CO` debt, §7.7.4 would lose one of its three trees, §12.6.4's summary
would move a name from the refused list to the performed one, and §12.11.5 would need rewriting
from the ground up — and would have to come back *into* scope, which is a sharper cost than it was when this said `partial`, because its argument is that there is nothing to disable. §12.7.8.3.1's debt is
an FDF import and would not move.

**So the compliance case is real but small: one row settled, five amended, and three that go
backwards.** A round that argued for this on the ledger alone would be overselling it, and this file
says so on purpose.

### 4.2 The corpus

Measured 2026-08-28 with `refused_action_census` — the object-walking instrument, not a `grep`:

| population | opened | documents stating `/S /JavaScript` | dictionaries |
|---|---|---|---|
| `doc/pdf.js/test/pdfs` | 964 of 974 | **57** (5.9%) | 250 |
| `doc/corpora/*` + `corpus-cache/openpreserve` | 537 of 542 | **7** (1.3%) | 21 |
| `corpus-cache/safedocs` | **65 703 of 65 944** | **276** (0.42%) | 9 442 |

**On the SafeDocs crawl, `/S /JavaScript` is the most-refused action type this program has, by
document count** — 276, against 115 for `/GoToR`, 93 for `/Launch`, 34 for `/SubmitForm` and 7 for
`/Rendition`. On the pdf.js corpus the gap is wider still: 57 against 2, 1 and 1. Whatever else is
true, this is not a refusal nobody meets.

`/CO` and `/AA` are not action dictionaries, so `refused_action_census` does not see them — and this
file's first draft recorded them as raw-byte `grep` counts. **That was wrong twice over and the
instrument already existed**: `crates/pdf-model/examples/witness_census.rs` asks a term at three
layers, and its `objects` layer walks every object the cross-reference table names *including the
ones inside §7.5.7 object streams*, matching a `Name` as a **token** rather than as text. Over the
974:

```sh
cargo run --release -p pdf-model --example witness_census -- --pdfjs JavaScript AA CO XFA AcroForm
```

| term | `raw` (what a `grep -a` sees) | **`objects` — what the document *states*** | `streams` |
|---|---|---|---|
| `/JavaScript` | 18 | **57** | 46 |
| `/AA` | 354 | **32** | 273 |
| `/CO` | 224 | **12** | 199 |
| `/XFA` | 5 | **7** | 9 |
| `/AcroForm` | 163 | **165** | 6 |
| `/ObjStm` | 281 | **276 (28.3%)** | 2 |

**A byte `grep` is wrong in both directions here, which is why neither direction bounds the other.**
It undercounts `/JavaScript` by 68% (18 against 57) because 28.3% of these documents put their
objects in streams; and it *over*counts `/AA` elevenfold (354 against 32), because `AA` is a
substring of arbitrary compressed bytes. A lower bound and an upper bound at once is not a bound.
ADR 0403 is the round that paid for this lesson the first time — a ledger row spent thirty-one
rounds recording a measurement of `grep` rather than of the corpus.

Two structural counts the `objects` layer cannot separate on its own — a `/JavaScript` name-tree key
from a `/JavaScript` action value, an AcroForm `/CO` from §12.5.6.7's line-annotation caption offset
of the same name — were settled by dumping the object graph structurally (`qpdf --json`, as an
instrument and nothing else, in the role `pdftotext` already plays in the text gate). 944 of the 974
parse there; 30 are fuzzed or damaged. **19 documents state a document-level `/JavaScript` name tree**
(a floor: two of the 30 unparseable ones do state JavaScript), **3 carry an `/AA` on the catalog
itself** — Table 29's entry, whose value is the Table 200 dictionary — and **all 12 of the `/CO`s
are AcroForm calculation orders — not one is a line
annotation's caption offset**, which is worth knowing because the ledger flags that collision.

The same instrument over the crawl (`--crawl`, 65 703 of 65 944 opened):

| term, as a stated name | pdf.js (964 opened) | share | SafeDocs (65 703 opened) | share |
|---|---|---|---|---|
| `/JavaScript` | 57 | 5.91% | **378** | 0.58% |
| `/AA` | 32 | 3.32% | **210** | 0.32% |
| `/CO` | 12 | 1.24% | **82** | 0.12% |
| `/XFA` | 7 | 0.73% | 37 | 0.06% |
| `/AcroForm` | 165 | 17.1% | **4 766** | 7.25% |

**Three things fall out of that table and none of them was guessable.**

- **This is a forms feature, not a corpus-wide one.** Against the population that has an `/AcroForm`
  at all, the script action is **34.5% in pdf.js (57 of 165)** and **5.8% in the crawl (276 of
  4 766)**. Both figures are right and they answer `CLAUDE.md`'s two different questions: pdf.js's
  corpus is the regression suite of *a viewer that implements JavaScript*, so it over-samples the
  feature by construction, and the crawl is the honest denominator for robustness. The ratio between
  them — a factor of six on the forms-relative figure, fourteen on the whole-corpus one — is itself
  a measurement of that bias, and it is the reason this file quotes both.
- **`/AA` is *not* where most of the world's scripts hang.** The crawl states a script action in 276
  documents and an additional-actions dictionary in only 210. So a majority of real script actions
  are reached through `/OpenAction`, an outline item's `/A`, a link, or §7.7.4's name tree — not
  through a field trigger. §12.3.3's ledger row already recorded the outline half of that: of 281
  corpus outline items carrying an `/A`, **18 are JavaScript**. **A first step scoped to `/AA /F`
  and `/AA /K` therefore reaches a minority of the demand on purpose**, and §7 should be read
  knowing that — it is scoped by *safety*, not by coverage.
- **`/CO` is rarer than `/AA`**, 82 against 210 in the crawl. Calculation chains are the smaller half
  of what forms scripting is used for, which ranks §7's second step below its first for demand as
  well as for risk.

## 5. What it would mean for functionality

**What a reader gains**, and each of these is a thing a person can see:

- **Field formatting.** A currency or date field whose stored value is `1234.5` displays as
  `$1,234.50` because `/AA /F` runs `AFNumber_Format`. Today this program draws the raw value —
  correctly, per the appearance stream it is given, and differently from every other reader.
- **Field validation** (`/AA /V`) — a value out of range is refused as the document asks.
- **Keystroke handling** (`/AA /K`) — the mask that stops a letter reaching a numeric field.
- **Calculation chains** — Table 224's `/CO` says the order, `/AA /C` says the arithmetic; a total
  field that updates when a line item changes. This is the one that makes a form *work* rather than
  merely accept typing, and this tree already lets a person fill a field.
- **Document-level scripts** (§7.7.4's name tree) — the function library the field scripts call.
  Note that these are `shall`-executed on open, which puts them on the *launch path*, and
  `CLAUDE.md` principle 2 has something to say about that (§8 below).

**What it does not gain, and this list is as important:**

- **No new pixel is drawn by the engine.** A script changes a *value*; the appearance is still
  constructed by §12.7.4.3's variable-text machinery, which this tree already has. The engine is
  upstream of drawing, never inside it.
- **Nothing in clause 13.** ISO 21757-1's clauses 11–12 are the 3D API and stay excluded.
- **No XFA.** Annex K stays excluded on the permission §K.1 grants, and the AcroForm this tree
  reads is still the form.
- **Nothing for a document that has no script**, which the census says is 99.6% of the world's
  files and 94% of the pdf.js corpus. This buys the tail, not the body.

**And the obligations it creates, each of which is a decision somebody has to take:**

1. **A script that changes a field's value must reach appearance regeneration.** This is the
   architectural constraint and it is already satisfied in shape: `CLAUDE.md` requires
   `pdf_syntax::Document` to stay immutable, and `view.rs` already keeps an *action log beside the
   document* for §12.6.4's actions. A script's effect is another entry in that log; `interpret`
   stays a pure function of the bytes, the viewer state and what the user did — which the oracle's
   whole comparison rests on. **Any design that lets a script mutate the document is wrong on this
   tree** and should be refused at the first sketch.
2. **A script that opens a URL, writes a file, submits a form or mails a document is a security
   decision, not a feature.** ISO 21757-1 clause 9 already classifies these; `CLAUDE.md`'s
   *A document's restrictions are the reader's to set* already says such a thing is asked of a
   *policy* a host supplies, with four levels — `off`, `on`, ask, warn. **The four-level shape is
   the right home for every privileged method in ISO 21757-1**, and this is the strongest reason
   the two rules were written: a refusal that cannot become an "ask" is exactly what the standard's
   privileged-context clause needs.
3. **`app.alert`, `app.response`, a `Dialog`** — these need a host, and `doc/ui-boundary.md`'s rules
   apply. The confined window refuses everything outside its scope by name (ADR 0713); a script
   asking for a dialogue is one more thing it refuses in words.
4. **A panic in the engine must not be a crash in the viewer.** RUSTSEC-2024-0444's own workaround
   is `std::panic::catch_unwind`. `CLAUDE.md` principle 1 forbids `unwrap()` outside tests; Boa's
   `core/engine/src` contains 253 `.unwrap()`, 219 `.expect(`, 90 `unreachable!(`, 31 `panic!(` and
   294 `assert!(` in 153 266 lines. **This is a dependency whose panics are reachable from hostile
   input**, which is the fourth independent argument for the process boundary in §6.

## 6. The security argument, which is what the exclusion is actually about

The exclusion's stated reason is one sentence: "a sandboxed script engine is a separate project with
its own security argument." **The claim to test is whether it is still a separate project.** When
that sentence was written this tree had no sandbox. It now has:

- **`pdf-sandbox`** — a separate binary under seccomp-BPF and Landlock, with `RLIMIT_AS`,
  `RLIMIT_NOFILE` and `RLIMIT_FSIZE` of zero, and a channel that waits with a wall-clock budget and
  reports `SandboxError::TimedOut`. There is deliberately no in-process fallback.
- **`viewer-confined` + `pdf-view-worker`** — the *whole viewer* confined, with the host holding
  only a window (ADRs 0218, 0223, 0607, 0626, 0633, 0713).
- **an interruptible drawing path** — `pdf_render::Interrupt`, with the policy for stopping decided
  (ADRs 0650, 0657).

So the honest answer is: **the security argument is no longer a separate project; it is a placement
question with three candidates, and the machinery for two of them already exists.**

### Where it would sit

**(a) Inside the confined worker (`pdf-view-worker`).** The script engine is a library of the crate
that already interprets the document, in the process that is already under seccomp and Landlock with
no filesystem and no network. `RLIMIT_AS` is the memory budget Boa does not have; the channel
timeout is the wall clock it cannot be interrupted on; a panic kills a process the host already
knows how to lose.
*Against*: the worker is also what draws, so a script that spins takes the page with it — the
budget kills the wrong thing. Mitigable by running scripts on a thread the render loop does not
share, but then the `RLIMIT_AS` is shared and a script's allocation can starve a raster.

**(b) A third process of its own — `pdf-script-worker`.** One more binary on the pattern
`pdf-sandbox` already established: its own seccomp filter (much tighter — a script needs no
`mmap` beyond the heap and no file descriptors at all), its own `RLIMIT_AS` sized for a script
rather than for a page, its own wall-clock budget per invocation, and a typed protocol carrying
*only* the ISO 21757-1 operations the host chose to expose. A runaway script is killed without
touching a raster; a compromised engine has reached a process holding no document bytes it was not
handed and no pixels at all.
*Against*: a third worker is a new part of this tree (ADR 0709's sweep population), a launch-path
cost if it is spawned eagerly, and a protocol to design.

**(c) Host-side.** Refused outright. The host is the process with the window, the display
connection, the filesystem and whatever the toolkit dragged in. Running a document's script there
inverts the whole of principle 3.

**This file's reading is that (b) is right and that (a) is the tempting wrong answer**, for the
reason `pdf-sandbox` exists at all: the budget must bound the *hostile* thing without bounding the
work the user is waiting for, and a shared process cannot do that. But this is a design decision
and belongs in an RFC the owner rules on, not in a todo file — see §9.

### What it would be allowed to reach

The answer that follows from ISO 21757-1 clause 9 and from `CLAUDE.md`'s four-level rule together:

- **the document's own field values and their appearance-relevant properties** — always;
- **`util`'s pure formatting helpers, `color`, `this`, `event`** — always, they reach nothing;
- **`app.alert` and friends** — a message to the host, which the host may decline; never a modal the
  document controls;
- **`Net`, `SOAP`, `Doc.mailDoc`, `Doc.submitForm`, `Doc.saveAs`, `Doc.exportDataObject`, `Doc.print`,
  the `security` and `SecurityHandler` objects** — every one of them asked of a *policy* the host
  supplies, defaulting to `off`, capable of becoming *ask*;
- **the filesystem and the network** — nothing, at any level, because the process has neither. The
  policy above decides whether the *host* performs the operation, never whether the engine can.

That last line is the whole design in one sentence, and it is why the placement matters more than
the engine.

## 7. The recommendation

**Worth doing — and worth doing in a strict order, with a first step small enough to be judged.**
The reasoning, in the order the evidence supports it:

1. The specification states two `shall`s at a PDF processor in §12.6.4.17 and nine more in
   §12.6.3's Tables 199 and 200, and it defines the entire host API in a normative reference that
   is a published ISO standard. Principle 5 has a source of truth here, which it did not appear to
   have when the exclusion was written.
2. The corpus demand is the largest of any refusal this program has — 276 SafeDocs documents, 57 of
   the pdf.js corpus.
3. A memory-safe engine at 95.21% of test262 exists, is ahead of the C engine everyone embeds, and
   has one DoS advisory against QuickJS's four memory-corruption CVEs in two years.
4. The security argument the exclusion deferred has been built in the meantime, for another reason.

**The smallest useful first step**, and it is deliberately narrower than the commission's guess:

> **Formatting only, and only on a value the user already changed.** `/AA /F` (format) and `/AA /K`
> (keystroke) on a *single* field, evaluated in a third worker process with a wall-clock budget of a
> few milliseconds and an `RLIMIT_AS` in the low megabytes; the host object model limited to `event`,
> `this`, `util` and the read-only parts of `Field`; **no document-level scripts** (they are the
> launch-path risk and the `shall` that runs on open); no `/AA /V`, no `/CO`, no `app`, no `Doc`, no
> network, no filesystem, no dialogue. A script that exceeds any budget, throws, or names anything
> outside that set is **refused by name and reported**, which is trap 5 and is what this tree does
> with every other refusal.

That step is judgeable: it either makes a currency field read `$1,234.50` on the 32 pdf.js documents
that state an `/AA` at all, or it does not, and it costs one worker, one protocol and a subset of
`util`. It touches no pixel that a document without a script draws. **If it lands and the report
stays quiet, the calculation chain (`/CO` + `/AA /C`) is the second step**, on a smaller population
(12 documents in pdf.js, 82 in the crawl) and with the harder obligation — a chain has an order, and
Table 224 makes stating that order the *document's* duty rather than the reader's.

### The step re-priced against the settled source — and as written it fails its own test

The step above was written before the API source was decided. Reading it against what Adobe's
reference actually documents makes it a **short, checkable list** rather than a gesture, and it also
finds that **the step as stated cannot pass the criterion it sets for itself**. Both halves are
below; the second is the finding.

**What the step needs and Adobe documents.** Every member here is in the reference, under the
section named beside it, and a round can tick them off:

| member | what the step needs it for | Adobe's section |
|---|---|---|
| `event.name`, `event.type` | which trigger fired — the pairs `Field`/`Keystroke` and `Field`/`Format` | Event type/name combinations |
| `event.value` | the Keystroke script reads it; the Format script writes it | event properties |
| `event.change` | the incoming keystroke text | event properties |
| `event.changeEx` | the uncropped text, paired with `fieldFull` | event properties |
| `event.fieldFull` | says which of the two above is the whole of what the user typed | event properties |
| `event.willCommit` | marks the final Keystroke call, the one before Validate | event properties |
| `event.rc` | Keystroke listens to it; Format does not | event properties |
| `event.selStart`, `event.selEnd` | where in the value the change goes | event properties |
| `event.target`, `event.targetName` | the field the trigger fired on, and its name for the report | event properties |
| `event.commitKey` | 0–3: escape, click away, Enter, Tab | event properties |
| `Field.value` | read-only in this step | Field properties |
| `Field.valueAsString` | the distinction Adobe's own example turns on: a stored `"020"` is the string, not the number | Field properties |
| `Field.name`, `Field.type` | the report, and the restriction to a text field or combo box | Field properties |
| `Field.readonly`, `Field.display` | whether the trigger may fire at all | Field properties |
| `util.printf` | the whole conversion specification, with conversion characters `d f s x` | util |
| `util.printd` | the pattern table and the three numeric formats | util |
| `util.printx`, `util.scand` | the date round trip, including `scand`'s two-digit-year horizon | util |

**Seventeen members over three objects, and every one of them is documented.** `color` and `app` are
not needed by this step and `Doc` is needed only for `getField`, which the step excludes; the owner's
six objects are therefore three-and-a-fraction for a first step, which is a smaller surface than §7
guessed and is good news.

**Now the bad news, and it is the round's real finding.** The step is judged on whether a currency
field reads `$1,234.50`. What makes a field read that is `AFNumber_Format`, because a field's `/AA
/F` value in a real document is almost always a single call to one of the `AF*` functions and
nothing else. And:

- **`AF*` is in neither source.** Not in Adobe's reference (measured: zero occurrences), not in ISO
  21757-1's contents, and issue #100 records the working group declining to specify it — see §3.
- **Acrobat supplies those functions from its own shipped script library**, which is why the calls
  work there and why a document does not carry them.
- **The step excludes document-level scripts**, correctly, for the launch-path reason §7 gives. So
  there is no route by which the document could supply them either.

Put together: **a worker holding `event`, `this`, `util` and a read-only `Field` will evaluate
`AFNumber_Format(2, 0, 0, 0, "$", true);` and raise a reference error, on every document the step is
aimed at.** The step is judgeable, and judged as designed it fails. That is not an argument against
the step; it is an argument that its *criterion* was chosen before its source was known.

**The adjustment, with the option this file recommends named.** Three ways out, and they differ in
how much of the result is a documented choice:

1. **Implement the `AF*` family in the host.** It is what makes the step's criterion reachable, and
   it is what the two implementations in issue #100's thread did. **Every line of it is a documented
   choice**: the only published description is an argument table with no algorithm, which documents
   one argument's values partially, mislabels another and omits half the family — so the rounding
   rule, the locale, the separator styles and what counts as a valid telephone number are all this
   project's, decided with nothing to derive them from. It would be the largest block of
   documented-choice code this tree has ever taken on, and it should not arrive attached to a step
   whose purpose is to prove a worker and a protocol.
2. **Narrow the step to what the documented sources decide, and recommend it.** Run the trigger,
   evaluate the script in the third worker under its budgets, apply an assignment to `event.value`
   through the action log beside the document, and **refuse anything the object model above does not
   hold — by name, out loud**, which is trap 5 and is what this tree does with every other refusal.
   On the documents §7 aims at, that reports `AFNumber_Format` refused rather than a formatted
   field. **The criterion changes with it, and that is the point**: the step is then judged on
   whether the script runs, whether the budget holds, whether the log stays beside the document, and
   whether every refusal is named — four things the two sources fully determine — instead of on a
   formatting result that nothing specifies. The `AF*` layer becomes the second step, ahead of the
   calculation chain, with its own argument and its own documented choices.
3. **Implement `AFNumber_Format` and `AFNumber_Keystroke` alone.** Rejected: the argument table
   documents the separator argument for two of its four values, so even one function is mostly
   choice, and doing it inside a step scoped by safety hides that.

**So §7's ordering stands and its first step's criterion does not.** The recommendation is option 2,
and the step's own sentence above — that it either makes a currency field read `$1,234.50` or it does
not — should be read as the thing this subsection corrects.

**It is scoped by safety and not by coverage, and §4.2's second bullet says what that costs**: most
of the world's script actions hang off `/OpenAction`, an outline item or the name tree rather than
off a field trigger, so this step reaches a minority of the demand deliberately. The step that
reaches the majority is the document-level one, and it is the one that must not be first.

**What must not be done first**, each for a stated reason:

- **not document-level scripts** — §7.7.4's tree is `shall`-executed on open, so it is on the launch
  path, which principle 2 protects hardest; and it is the largest attack surface for the smallest
  visible gain;
- **not rquickjs or the `v8` crate**, however much better their containment APIs are — they are the
  C and C++ dependency principle 3 is about, and this file's whole finding is that the Rust option
  has become viable;
- **not an engine written in this tree.** The `bigint` precedent settles it: the owner chose
  reviewed dependencies over in-tree arithmetic for cryptography, and an ECMAScript engine is a
  larger and less-testable thing than a bignum.

**And if the owner declines**, the finding that survives regardless is §4.2's last paragraph: `/CO`
and `/AA` have never been counted with a real instrument, and a census in the shape of
`refused_action_census` is owed either way.

## 8. The amendment argument, for the owner to ratify or refuse

`CLAUDE.md` says an exclusion is revisited "by argument, never by attrition", and this section is
the argument in the form the owner would have to accept. It is written to be *refusable*: three of
the four claims below cut against it.

**The exclusion as it stands:**

> **JavaScript and script-driven form behaviour** — a sandboxed script engine is a separate project
> with its own security argument. Field *appearance* is not excluded; field *behaviour* is.

**What has changed since it was written, in the order that matters:**

1. **The security argument is no longer separate.** It was written when nothing in this tree was
   sandboxed. `pdf-sandbox` and `viewer-confined` are now built, and §6 shows the placement is a
   design question with existing machinery, not a project. **This is the sentence the exclusion
   rests on, and it has expired** — the same shape as the entry that said transfer functions
   "describe a marking device", and the same shape as the XFA entry whose second half was wrong.
2. **The half the standard was thought not to define is defined.** ISO 21757-1:2020 is a normative
   reference of ISO 32000-2 and specifies the whole host object model in 190 in-scope pages. Not
   Adobe's convention; an ISO standard. Principle 5's "find the clause" has a clause to find.
3. **It is not one clause and not one `shall`.** §12.6.4.17 states two, and §12.6.3's Tables 199
   and 200 state nine more that name an ECMAScript action in the table cell itself — including the
   four field triggers that decide what a filled form *shows*. All of it is in clause 12, which
   `CLAUDE.md` puts in scope as "everything that displays", and §12.6.4.17's own sentence says a
   script may "change their visual appearances".
4. **The demand is measured and is the largest of any refusal**: §4.2.

**What cuts against it, honestly:**

- **The standard itself contemplates a processor that cannot run script, and provides for it in as
  many words.** §12.6.4.14's rendition action, Table 218: "Either the JS entry or the OP entry shall
  be present. If both are present, OP is considered a fallback that shall be executed if the
  interactive PDF processor is unable to execute ECMAScripts." That is not a licence to decline —
  §6.3.2.1's conformance clause is the floor and `CLAUDE.md` says so — but it is the standard
  admitting the category, and an honest amendment argument has to put it in front of the owner
  rather than leave it for them to find.
- **The ledger delta is one row settled and three made worse.** §12.11.1, §12.11.2 and §12.11.5 are
  satisfied *because* nothing runs; §4.1 has the mechanism. Anyone arguing this on coverage is
  overselling; the argument is the corpus's and the `shall`'s, not the ledger's.
- **No engine can satisfy principle 3's `#![forbid(unsafe_code)]`**, and this would be the first
  crate in the render path whose transitive graph contains a hand-written garbage collector. That
  is a genuine weakening of a compiler-enforced rule into a process-enforced one, and it should be
  written down as a deliberate decision with its cost, not smuggled in.
- **The dependency is large and lightly funded**: 54 new packages in the lock file (measured — see
  below), a codebase of 242 401 lines calling itself experimental, no OSS-Fuzz seat, ~500 commits a
  year, donation-funded.
- **99.6% of the world's PDFs would be unaffected.** This is a tail feature, and principle 2's
  launch-path rules mean it must cost nothing at all for the documents that have no script.

**The dependency, measured** (2026-08-28, `cargo tree` against this tree's `Cargo.lock`):
`boa_engine` with `default-features = false` pulls **122 transitive packages**, of which **68 are
already in this workspace's 521** and **54 would be new** — 9 of them Boa's own crates, the rest
`icu_*` (10), `time`/`num-*`/`rand` and small utilities. With default features it is 124. That is a
14% growth in the third-party graph for a feature 0.42% of documents use, and `doc/stack.md`'s
question deserves that number in front of it.

**The amendment this file proposes, if the owner is persuaded** — narrower than deleting the
exclusion:

> **JavaScript and script-driven form behaviour** — in scope as far as §12.6.4.17's `shall` and ISO
> 21757-1's non-privileged API reach a field's value and therefore its appearance, evaluated in a
> confined worker under a memory and a wall-clock budget. Every privileged method ISO 21757-1
> clause 9 identifies is asked of the host's policy under *A document's restrictions are the
> reader's to set*, defaulting to `off`. ISO 21757-1's clauses 11–12 (the 3D API) stay excluded
> under clause 13. **Excluded still**: any script effect that would mutate `pdf_syntax::Document`,
> and any engine that is not memory-safe in the sense §2.2 measures.

**The owner decides. Nothing in this file changes a ledger status, a `Cargo.toml` or the exclusion,
and the round that wrote it added no dependency and no engine.**

## 9. What a round would do next, and in what order

1. **The owner rules on §8.** Nothing below may start first.
2. **An RFC, not a todo.** The placement question in §6 and the API subset in §7 are a design
   several rounds long, and `doc/rfc/`'s conventions were written for exactly this — a proposal the
   owner marks up, with the standing restriction named and its rationale given. **The next free
   number is 0006**; this round did not take it, because `doc/rfc/` is awaiting the owner's review
   and a round does not add to a queue it was told not to touch.
3. **The `/CO` and `/AA` census** — owed whatever the owner decides, per §4.2.
4. ~~**Acquire ISO 21757-1:2020.**~~ **Answered by the owner, and answered against acquiring it.**
   This step used to read that the standard had to be bought and that principle 5 could not be
   satisfied without it. The owner has decided otherwise: ISO 21757-1:2020 will not be obtained,
   Adobe's *JavaScript for Acrobat API Reference* is the working source for the object model, and
   `pdf-association/pdf-issues` is a secondary reference — §3's *The source, settled by the owner*.
   What replaces the step is not nothing, and it is three obligations rather than a purchase:
   - **The API source is Adobe's and its details are documented choices**, never derivations. §3's
     *What principle 5 makes of a vendor document a standard adopted* is the argument and its limit;
     a round that writes an API member without that framing has quietly promoted a vendor document
     to a standard.
   - **The citation shapes in §3 bind**, because two of the four wrong spellings pass the checker in
     silence and land on real clauses of the wrong standard. Nothing will tell a round it got this
     wrong.
   - **The language version comes from the errata, not from either standard.** ISO 21757-1 as
     published pins no ECMAScript version; issue #185 replaces its normative reference with
     ECMA-262's eleventh edition. Any engine-conformance claim is against that.
5. Only then: the worker, the protocol, the subset, the budgets, the report — and §7's re-priced
   first step rather than its original one, since the original's criterion needs a function neither
   source specifies.

## Sources, all read 2026-08-28

- ISO 32000-2, §12.6.4.17 and the normative-references front matter, from `doc/md/`.
- ISO 21757-1:2020 — <https://www.iso.org/standard/71559.html>; publicly available preview
  (front matter, contents, clauses 1–10.2) via
  <https://standards.iteh.ai/catalog/standards/iso/5684a307-2e80-4bf1-a943-ab0cfa0cc85c/iso-21757-1-2020>.
- test262 daily results: <https://test262.fyi/>, data at <https://data.test262.fyi/index.json> and
  `meta.json`, run generated 2026-08-28T01:43:02Z, test262 `d86b229`.
- <https://github.com/boa-dev/boa> at `337a3668` (2026-08-28); <https://github.com/trynova/nova> at
  `4eea7c6f`; <https://github.com/Hans-Halverson/brimstone> at `e1a61566`;
  <https://github.com/DelSkayn/rquickjs>.
- RustSec advisory database, advisory RUSTSEC-2024-0444 (its file sits under that repository's own
  `crates/` directory, which is *not* a path in this tree).
- NVD: CVE-2024-43357, CVE-2024-43367, CVE-2024-13903, CVE-2025-69654, CVE-2026-0821, CVE-2026-0822,
  CVE-2026-1145.
- crates.io API for `boa_engine`, `nova_vm`, `rquickjs`, `v8`, `deno_core`, `quick-js`.
- `doc/pdf.js` at `2ea8820d` (2026-07-26) for `src/scripting_api/` and `external/quickjs/` — evidence
  about the world, never a target.

### And the sources the owner settled on, added the same day

- **Adobe, *JavaScript for Acrobat API Reference***, as an HTML page tree:
  <https://opensource.adobe.com/dc-acrobat-sdk-docs/library/jsapiref/index.html>, with the body in
  `JS_API_AcroJS.html` (every object but `Doc`) and `doc.html` (`Doc`), and the form-event diagram at
  `_images/formsevent.png`. **Floating, with no version to cite** — pin the commit of
  <https://github.com/adobe/dc-acrobat-sdk-docs>, whose `LICENSE.md` is MIT and covers the
  documentation files. The PDF edition at
  `https://www.adobe.com/content/dam/acom/en/devnet/acrobat/pdfs/js_api_reference.pdf` redirects to
  the landing page and is retired; the short link `https://www.adobe.com/go/acrobatsdk_jsapiref`
  redirects to a 404.
- **Adobe, *Interapplication Communication API Reference***, Acrobat Forms Plug-In →
  `SetJavaScriptAction`:
  <https://opensource.adobe.com/dc-acrobat-sdk-docs/library/interapp/IAC_API_FormsIntro.html#setjavascriptaction>
  — the only published description of the `AF*` arguments, and a parameter menu rather than an
  algorithm.
- **Adobe, *Developing Acrobat Applications Using JavaScript***:
  <https://opensource.adobe.com/dc-acrobat-sdk-docs/library/jsdevguide/index.html>, and its forms
  chapter `JS_Dev_AcrobatForms.html`. It adds the calculation-order discussion and nothing about
  firing order or the `AF*` functions.
- **`pdf-association/pdf-issues`**, the secondary reference:
  <https://github.com/pdf-association/pdf-issues>, published at <https://pdf-issues.pdfa.org/>,
  content **CC-BY-4.0**. The ISO 21757-1 errata are at
  <https://pdf-issues.pdfa.org/21757-1-2020/index.html>; the issues cited above are #185, #70, #100,
  #744, #535, #270 and #99 under the repository's `ISO 21757-1` label.
- **ISO 21757-1:2020's publicly available preview**, for the contents page the ancestry argument
  rests on:
  <https://cdn.standards.iteh.ai/samples/71559/99406ba1f07d4e79b97b14071c9eda06/ISO-21757-1-2020.pdf>
  — first edition, 2020-12, 253 pages, cover through clause 10.2. The full document is behind
  <https://www.iso.org/standard/71559.html> and is the purchase the owner declined.
