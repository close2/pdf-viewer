# A script engine that is memory-safe, and the exclusion it invites the owner to re-read

Status: **built.** The owner accepted `doc/rfc/0008` (`doc/questions/A193`) and ratified the
exclusion's amendment (A286, ADR 1904), and RFC 0008 section 11's order is run to its end: Tier 0's
`AF*` library runs natively in `pdf_model::aform` where a field's Table 199 script is one call with
literal arguments (ADRs 1578, 1579), and Tier 1 evaluates a document's scripts in Boa behind
`pdf-script`'s `engine` feature, in a third confined worker under budgets, at the reader's `Scripts`
level, `off` by default (ADRs 1590, 1591, 1608, 1609, 1616). What each site and member does is
`doc/state-of-play.md`'s to say; `pdf_script::surface::NOT_BRIDGED` is empty (ADR 1762), and the
Tier 1 column (`crates/pdf-script/tests/script_corpus.rs`, run with `--features engine`) is a merge
gate whose `one_document` test prints one document's throws with their scripts (ADR 1625). **What is
next** is chosen from *The Tier 1 column's throws, by cause* below, within `CLAUDE.md`'s exclusion as
amended; A193's rule holds — the target is as much of the pinned reference's surface as is secure
and reasonable, the census ranking the work and never capping it.

The rest of this file is the study of 2026-08-28 that RFC 0008 builds on and cites by section — the
engines graded, the clause read, the source the owner settled and how a round cites it — and its
figures are what was found then. Its premise, *is there now a safe ECMAScript library?*, held, with
the qualification section 1 states.
Priority: 56 — band 50–59.

Corpus: **run the census rather than reading a number here.** The instrument that answers *at
which site* and *what each script calls* is `crates/pdf-model/examples/javascript_census.rs`
(RFC 0008 §3 has its command); the two below answer *how many* — `refused_action_census` walks
every object the cross-reference table lists and every dictionary inside one, so it sees a script
action inside an object stream, which a byte-level `grep` cannot:

```sh
find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' > scratchpad/r<round>/paths-56
cargo run --release -p pdf-model --example refused_action_census -- @scratchpad/r<round>/paths-56

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

Code as it stands: `crates/pdf-model/src/aform/` (Tier 0's library, its one-call grammar and where
a field's script is read), `crates/pdf-model/src/view/scripts.rs` (Table 199's triggers raised,
`/CO` walked, and the `ScriptRunner` hook), `crates/pdf-script/` (Tier 1's engine, bridge, budgets
and wire), `crates/pdf-script-worker/` (the third confined program that runs it),
`crates/pdf-model/src/action.rs` (Table 201 read, and the sentence a script action gets where no
runner runs it), `crates/pdf-model/src/view/actions.rs` (`ViewState::perform_all`, the actions
performed beside the document), `crates/pdf-model/src/forms_data.rs`,
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
thing the principle is about. The count over the `boa_engine` this tree pins is ADR 1590 section
2's, with the command that re-takes it.

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

**This is not the obstacle it looks like, because this tree bounds a run one layer up.**
`crates/pdf-sandbox/src/lockdown_linux.rs` puts `RLIMIT_AS` on a worker process — "so a
decompression bomb fails an allocation instead of taking the machine's memory" — with
`RLIMIT_FSIZE` of zero and a descriptor ceiling per profile beside it (none at all for the script
worker, ADR 1608), under seccomp-BPF and Landlock; and `pdf_sandbox`'s channel waits with what is
left of the budget, which is what makes `SandboxError::TimedOut` reachable at all. **A process
boundary is a memory budget and a time budget that no engine API has to provide**, and it is
enforced by the kernel rather than by the engine's own correctness — the stronger guarantee,
because an engine that mis-accounts its own fuel has defeated its own limit and cannot defeat
`RLIMIT_AS`. So the containment column is not "Boa fails and QuickJS passes". It is: **QuickJS can
be contained in-process and Boa cannot; this project contains nothing in-process.**

**The ceilings a run is held to, as built**, are ADR 1590 section 4's `Budget::FIELD_EVENT`
(`crates/pdf-script/src/request.rs`): a wall time and a step count polled between the slices
`Script::evaluate_async_with_budget` yields — the fuel the `fuzz` feature withholds, recovered
without it — Boa's `RuntimeLimits` with the loop limit turned on, and guards before the built-ins an
argument can make large; past them, the script worker's deadline and `RLIMIT_AS` (ADRs 1608, 1609).
ADR 1590 names what no engine-side budget bounds — a native call that calls back into script many
times, growth through an operator — and those are the process's.

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
document may update their values or **change their visual appearances**." So it is a clause that
reaches what displays through a field's value.

**The finding that changes the shape of the argument is the normative reference.** §12.6.4.17 does
not point at Adobe's *JavaScript for Acrobat API Reference*; it points at an ISO standard:

> JavaScript is referred to as ECMAScript throughout this document and is defined by ISO/DIS
> 21757-1.

and the front matter records the change as deliberate: "ISO/DIS 21757-1 replaces several Adobe,
ECMA and ISO publications related to ECMAScript in PDF 2.0". **`ISO/DIS` is what the 2020 text
printed while the document was a Draft International Standard; it was published as ISO 21757-1:2020
in December 2020, first edition** (<https://www.iso.org/standard/71559.html>, read 2026-08-28).

That matters for principle 5 more than for anything else. The host object model on top of the
language — `event`, `Field`, `Doc`, `util` — **is defined, and by a normative reference of the
standard this project implements**: ISO 21757-1:2020's scope, as the front matter of its publicly
available preview states it (read 2026-08-28), is a set of ECMAScript object types for automating
and interacting with PDF documents and their contents.

Its normative references for the language itself are **ISO/IEC 22275:2018** (the ECMAScript
Specification Suite) and ISO/IEC 22537:2006 — so "conformance to test262" is the right instrument
for the language half, and ISO 21757-1 is the source of truth for the host half. There is no point
in this design where another implementation has to be treated as the definition of correct.

**ISO 32000-2 refers to no other document for any of this, and that is checkable**: `grep -cE
'ECMA-262|ECMA 262|JavaScript for Acrobat|32004' doc/md/ISO_32000-2_sponsored_EC3.md` prints 0. Its
one normative reference for the language and the API alike is the ISO 21757-1 line above. So the
division of labour is exact: **ISO 32000-2 specifies the *dispatch*** — where a script hangs, when
it fires, in what order — **and not one API name**; ISO 21757-1 specifies the API. An engine built
from ISO 32000-2 alone would execute correct ECMAScript against an empty host object graph, which is
why where the API is read from had to be settled; the owner's answer and its cost are the
subsections at the end of this section.

Where the words fall says the same thing about where the work is: `grep -o ECMAScript
doc/md/ISO_32000-2_sponsored_EC3.md | wc -l` counts them, and they cluster in §12.6.4.17, Table
246's FDF dictionary, §12.6.3, §12.11.2 and §12.11.5, with clause 13's under the clause 13
exclusion. **The in-scope surface is three clauses and six tables**: 197, 199, 200, 221, 224
and Table 32's name tree.

**And the security model is in the standard too**, which is the part this project should care about
most. ISO 21757-1:2020 clause 9, *Privileged versus non-privileged context*, marks the methods that
carry a security note and lets them run only in a privileged context — the console, a batch, the
application's initialisation — and makes every other event, a page's open and a mouse-up among
them, non-privileged. That is the line this project would draw, at the same place: a script that
runs because a *document* was opened or a *widget* was clicked is non-privileged by the standard's
own definition. Clause 7, *Safe path*, is weaker and says so: it encourages a processor to support a
safe path and requires nothing — a permission-shaped clause, and this project goes further than it
asks.

### The `shall`s are not one clause's — §12.6.3 states four more, per field

§12.6.3 introduces four additional-actions dictionaries by name, and the tables are worth having in
front of the argument because **each of Table 199's four entries is a `shall` naming an ECMAScript
action outright**:

| table | dictionary | entries that name an ECMAScript action |
|---|---|---|
| **197** | an annotation's additional-actions | `/E` `/X` `/D` `/U` `/Fo` `/Bl` `/PO` `/PC` `/PV` `/PI` — actions of any type; all ten are read and raised (§12.6.3's ledger row, ADR 1750) |
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

**So the count of `shall`s the exclusion closed was not one but fourteen or so**, spread over five
tables — nine of which (Table 199's four, Table 200's five) name ECMAScript in the table cell
itself. **Table 200's five are a close, a save and a print**, run at the moments a host marks, a
script's `event.rc` reported and never obeyed (ADR 1614).

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

The exclusion's amendment was a separate question, ratified in A286 (section 8).

### What principle 5 makes of a vendor document a standard adopted

Principle 5's usual answer is the wrong one here, and the reason is worth being able to state.

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

**A `§` this tree does not put a document in front of** means *a clause of ISO 32000-2* — that is
what makes every one of them checkable, and `tools/conformance/src/citation.rs` says so in the doc
comment on its `ForeignCitation`, which exists for the one failure that matters here: a *readable*
citation of something else, which "checks as ISO 32000-2's §5.2, which exists, so it passes in
silence". (**A `§` that names one of this project's own sections is the other exception**:
`` `doc/todo/02` §2 `` and `doc/oracle-and-corpus.md`
§3d are how this project cites *itself*, classified as `citation::ProjectSection` — counted and
printed, checked against nothing, ADR 0987.) Taking a second and a third source therefore has an
instrument cost, priced by running the scanner rather than by reasoning about it (trap 13): each
line below was fed to
`conformance::citation::scan` and its verdict read off, with the clause index asked whether the
number it landed on exists:

| written as | what the checker records | verdict |
|---|---|---|
| a section sign after `ISO 21757-1` | a **foreign citation** of "ISO 21757-1" | **caught** — the gate fails, and the message teaches the spelling: write "ISO 21757-1 section N" |
| a section sign after `ISO 21757-1:2020` | a **foreign citation** of "ISO 21757-1:2020" | **caught** — the guard takes a year joined by a colon as part of a standard's number (ADR 1004) |
| a section sign after `the JavaScript for Acrobat API Reference` | a citation of **ISO 32000-2 §12.5**, which exists | **silent pass, onto the wrong standard** |
| `ISO 21757-1 Table 113` in any comment | a reference to **ISO 32000-2's Table 113** — "Additional entries in Mac OS Roman encoding not in MacRomanEncoding" | **silently the wrong table** |

**The third row is the one to remember, because a prose title is what a round will reach for when
the standard has no acronym.** The guard is `citation::another_document`, which recognises another
document by an acronym followed by a number — `ISO 15076-1`, `ETSI EN 319 122-1` — or by an ITU-T
Recommendation's single word, and a *title* is none of those, so nothing in front of the sign
identifies a document and the number is checked against ISO 32000-2. This file writes "JavaScript
for Acrobat API Reference" constantly, which is exactly the string somebody will copy into a doc
comment.

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
the SDK landing page; so does `js_developer_guide.pdf`. The page tree is served at
<https://opensource.adobe.com/dc-acrobat-sdk-docs/library/jsapiref/index.html>, retitled *Acrobat
JavaScript API Reference* — a URL that answered on 2026-08-28, gave 404 on 2026-09-30 and answers
again on 2026-10-20 — and its source is one page per topic under
`docs/acrobatsdk/html2015/Acro12_MasterBook/JS_API_AcroJS/` in the repository named below, whose
newest commit is `ab3b42a7` (2026-01-22). As read on 2026-08-28 the body was **two pages**:
`JS_API_AcroJS.html`, every object type except `Doc`, and `doc.html`, `Doc`. Objects are
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
this project since ADR 0252. So the owner's "secondary reference" is not a new source: it is the
upstream of an instrument this tree already has.

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

**Every row the exclusion once cost is settled, and the ledger is read rather than this file**
(`grep -nE '^clause = "(12\.6\.4\.17|12\.6\.3|12\.7\.3|7\.7\.4|12\.11\.[125])"' -A2
doc/conformance/ledger.toml`): §12.6.4.17 is `implemented` at the reader's `Scripts` level, `off` by
default (A100's ruling, applied by A286), and §7.7.4, §12.6.3, §12.6.4, §12.7.3 and §12.7.8.3.1
carry no script debt; RFC 0008 section 7 reads each row in both directions.

**The cost this section predicted was the one paid.** §12.11.1, §12.11.2 and §12.11.5 were
satisfied *because* nothing ran, every handler a file could name disabled by construction. An
engine made Table 273's `/RH` an entry to find and act on: the open
sequence now passes over every handler a checked requirement names, and §12.11.5 came back into
scope and is `implemented` (ADR 1923). **A capability that arrives turns settled rows into debts**,
which is `doc/habits/the-ledger-and-claims-about-this-tree.md`'s subject from the other direction,
and an amendment argument has to carry that cost in front of the owner.

### 4.2 The corpus

The census commands at the head of this file are the figures, and RFC 0008 section 3 re-took them
over its own population. Three findings of 2026-08-28 outlive their numbers:

- **On the SafeDocs crawl `/S /JavaScript` was the most-refused action type this program had, by
  document count**, ahead of `/GoToR`, `/Launch`, `/SubmitForm` and `/Rendition`, and on the pdf.js
  corpus by a wider margin: never a refusal nobody meets.
- **A byte `grep` is wrong in both directions for these names, so neither direction bounds the
  other.** It undercounts `/JavaScript`, because a document can put its objects in §7.5.7 object
  streams, and overcounts `/AA`, a substring of arbitrary compressed bytes; `witness_census`'s
  `objects` layer, which walks every object the cross-reference table names and matches a name as a
  token, is the instrument (ADR 0403 paid for the lesson first). Two collisions it cannot separate —
  a `/JavaScript` name-tree key from an action value, an AcroForm `/CO` from §12.5.6.7's
  line-annotation caption offset of the same name — were settled by dumping the object graph
  structurally (`qpdf --json`, as an instrument and nothing else): every `/CO` in the pdf.js corpus
  was a calculation order.
- **This is a forms feature, and `/AA` is not where most scripts hang.** Against the documents with
  an `/AcroForm`, the script action's share was about six times higher in pdf.js than in the crawl:
  pdf.js's corpus is the regression suite of a viewer that implements JavaScript, so it over-samples
  the feature by construction, and the crawl is the honest denominator for robustness — `CLAUDE.md`'s
  two questions. The crawl stated a script action in more documents than an additional-actions
  dictionary, so most scripts reach a reader through `/OpenAction`, an outline item's or a link's
  `/A`, or §7.7.4's name tree rather than a field trigger; RFC 0008 section 3.2 confirmed it.

## 5. What it would mean for functionality

**What a reader gains**, each a thing a person can see and each built (`doc/state-of-play.md`): a
field's formatting — a stored `1234.5` displayed as `$1,234.50` because `/AA /F` runs
`AFNumber_Format` — validation (`/AA /V`), the keystroke mask (`/AA /K`), the calculation chain
(Table 224's `/CO` the order, `/AA /C` the arithmetic), which is what makes a form *work* rather
than merely accept typing, and the document-level library §7.7.4's name tree defines, run once the
first page is presented rather than on the launch path (ADR 1602).

**What it does not gain**: no pixel is drawn by the engine — a script changes a *value*, and the
appearance is still §12.7.4.3's variable text; nothing of clause 13 (ISO 21757-1's 3D API); no XFA
(Annex K, on the permission §K.1 grants); and nothing for a document with no script, which is most
of the world's files (section 4.2's census).

**The obligations it created, each met as built:**

1. **A script's effect is an entry in a log beside the document**, never a change to
   `pdf_syntax::Document`, so `interpret` stays a pure function of the bytes, the viewer state and
   what the user did, which the oracle's comparison rests on: a script's write to a field is an edit
   (ADR 1603). Any design that lets a script mutate the document is wrong on this tree.
2. **A script that opens a URL, writes a file, submits a form or mails a document is a security
   decision, not a feature.** ISO 21757-1 clause 9 classifies these, and `CLAUDE.md`'s *A document's
   restrictions are the reader's to set* gives them the host's four-level policy; RFC 0008 section
   4's tiers say what crosses, and Tier 2 is refused by name.
3. **`app.alert` and `app.response` need a host**, under `doc/ui-boundary.md`'s rules: a question is
   an event out and a command back, held in the worker until answered, the window's thread never
   waiting (ADRs 1627, 1628).
4. **A panic in the engine must not be a crash in the viewer.** RUSTSEC-2024-0444's own workaround
   is `std::panic::catch_unwind`, and Boa's sources carry hundreds of reachable panics (`grep -rhoE
   --include=*.rs '\.unwrap\(\)|\.expect\(|unreachable!\(|panic!\(|assert!\('
   ~/.cargo/registry/src/*/boa_engine-0.22.0/src | sort | uniq -c` counts them); a release build
   aborts on one, so the boundary is the third worker's process (ADRs 1590, 1608) — the fourth
   argument for it in section 6.

## 6. The security argument, which is what the exclusion is actually about

The exclusion's stated reason was one sentence: "a sandboxed script engine is a separate project
with its own security argument." It stopped being a separate project because the tree had, by
then, built the machinery for another reason:

- **`pdf-sandbox`** — a separate binary under seccomp-BPF and Landlock, with `RLIMIT_AS`,
  `RLIMIT_FSIZE` of zero and a descriptor ceiling, and a channel that waits with a wall-clock budget
  and reports `SandboxError::TimedOut`. There is deliberately no in-process fallback.
- **`viewer-confined` + `pdf-view-worker`** — the *whole viewer* confined, with the host holding
  only a window (ADRs 0218, 0223, 0607, 0626, 0633, 0713).
- **an interruptible drawing path** — `pdf_render::Interrupt`, with the policy for stopping decided
  (ADRs 0650, 0657).

So the security argument became a placement question with three candidates, and the machinery for
two of them already existed.

### Where it would sit

**(a) Inside the confined worker (`pdf-view-worker`).** The engine in the process that already
interprets the document under seccomp and Landlock; `RLIMIT_AS` the memory budget Boa does not have,
the channel timeout the wall clock it cannot be interrupted on, a panic a process the host knows how
to lose. *Against*: that worker also draws, so a script that spins takes the page with it — the
budget kills the wrong thing — and on a thread of its own the `RLIMIT_AS` is shared, so a script's
allocation can starve a raster.

**(b) A third process of its own — `pdf-script-worker`.** One more binary on `pdf-sandbox`'s
pattern: a much tighter filter, an `RLIMIT_AS` sized for a script rather than a page, a wall-clock
budget per invocation, and a typed protocol carrying only the operations the host chose to expose. A
runaway script is killed without touching a raster; a compromised engine has reached a process
holding no document bytes it was not handed and no pixels at all.

**(c) Host-side.** Refused outright: the host is the process with the window, the display
connection, the filesystem and whatever the toolkit dragged in, and running a document's script
there inverts the whole of principle 3.

**(b) is right and (a) is the tempting wrong answer**, for the reason `pdf-sandbox` exists at all:
the budget must bound the *hostile* thing without bounding the work the user is waiting for. RFC
0008 section 6.2 agreed, and (b) is built — the narrowest of `pdf_sandbox`'s profiles, started at a
document's first trigger rather than at launch (ADRs 1608, 1609).

### What it would be allowed to reach

What a script reaches is `CLAUDE.md`'s exclusion as amended — this document's field values and
appearance properties and this viewer's state — with RFC 0008 section 4's tiers member by member,
and `pdf_script::surface::REFUSED` naming every refusal with its reason. The design in one sentence,
which the build kept: **the filesystem and the network are nothing, at any level, because the
process has neither; a policy decides whether the *host* performs an operation, never whether the
engine can.**

## 7. The recommendation

**Worth doing, in a strict order, with a first step small enough to be judged** — on four grounds
in the order the evidence supported them: the standard's `shall`s (§12.6.4.17's two and the nine of
§12.6.3's Tables 199 and 200); a memory-safe engine ahead of the C engine everyone embeds, with one
denial-of-service advisory against QuickJS's memory-corruption CVEs (sections 1 and 2); the largest
corpus demand of any refusal (section 4.2); and a security argument already built for another reason
(section 6).

The first step proposed was **formatting only, on a value the user already changed**: `/AA /F` and
`/AA /K` on a single field, in a third worker under a wall-clock budget of a few milliseconds and an
`RLIMIT_AS` in the low megabytes, the object model limited to `event`, `this`, `util` and the
read-only parts of `Field`, no document-level scripts, and everything else refused by name and
reported (trap 5); the calculation chain (`/CO` + `/AA /C`) second, with the harder obligation that
Table 224 makes stating the order the *document's* duty.

### The step re-priced against the settled source — and as written it fails its own test

Read against what Adobe's reference documents, the step is a short, checkable list, every member in
the reference under the section named beside it:

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
not needed by this step and `Doc` only for `getField`, which the step excludes.

**And the step as written cannot pass its own criterion**, a currency field reading `$1,234.50`.
What makes a field read that is `AFNumber_Format`, because a real document's `/AA /F` is almost
always one call to an `AF*` function and nothing else; and `AF*` is in neither source (section 3:
none in Adobe's reference, none in ISO 21757-1's contents, issue #100 declined), Acrobat supplies it
from its own shipped library, and the step excluded the document-level scripts that could have. **A
worker holding `event`, `this`, `util` and a read-only `Field` raises a reference error on
`AFNumber_Format(2, 0, 0, 0, "$", true);` on every document the step is aimed at** — the criterion
was chosen before the source was known.

Three ways out were weighed: (1) implement the `AF*` family in the host, every line of it a
documented choice, since the only published description is an argument table with no algorithm;
(2) narrow the step to what the sources decide — *the script runs, the budget holds, the log stays
beside the document, every refusal is named* — and make `AF*` its own step; (3) `AFNumber_Format` and
`AFNumber_Keystroke` alone, rejected because even one function is mostly choice and a step scoped
by safety would hide that. **RFC 0008 took the first two together**: the `AF*` library as Tier 0,
native and with no engine, each rule a documented choice (ADRs 1578, 1579), and Tier 1's first step
judged on the second's four criteria (RFC 0008 section 11 item 3 (a), ADR 1591).

**It was scoped by safety and not by coverage**: most of the world's script actions hang off
`/OpenAction`, an outline item or the name tree (section 4.2), and the step that reaches them — the
document-level one — was the one that must not be first, since §7.7.4's tree is `shall`-executed on
open and principle 2 protects the launch path hardest; it runs after the first present (ADR 1602).
The other two things not to do stand as RFC 0008 decided them: not rquickjs or the `v8` crate,
which are the C and C++ dependency principle 3 is about (QuickJS is A193's named alternative, ADR
1590), and not an engine written in this tree (RFC 0008 section 5.2 prices it; the owner's choice
of reviewed dependencies over in-tree arithmetic for cryptography is the precedent).

## 8. The amendment argument, for the owner to ratify or refuse

**Ratified.** The owner answered `doc/questions/Q286` (A286), and `CLAUDE.md`'s exclusion reads as
amended: a script runs in scope as far as it reaches this document's field values and appearance
properties and this viewer's state, in a confined worker under a memory and a wall-clock budget, at
the level the reader sets; ADR 1904 names what is built under each half.

The argument rested on four changes — the security argument built in the meantime (section 6), the
host half defined by a normative reference (section 3), a dozen `shall`s across §12.6.3's tables
rather than one, and the largest measured demand of any refusal (section 4.2) — and was written to
be refusable, with what cuts against it in front of the owner. Those costs stand, and RFC 0008
section 8 writes them down: the standard contemplates a processor that cannot run script (§12.6.4.14's
Table 218 makes `/OP` the fallback for one that cannot), §12.11's rows were satisfied because
nothing ran (section 4.1), no engine meets principle 3's `#![forbid(unsafe_code)]`, so a
compiler-enforced rule is weakened to a process-enforced one (ADR 1590 section 2), the dependency is
large and lightly funded (ADR 1590 section 1 counts its packages), and a tail feature must cost
nothing for a document with no script.

## The Tier 1 column's throws, by cause

Read against each document's scripts with the column's `one_document` test; the counts are the
column's to print, and a class is named here with what a reader of this program meets and whether
the producer's own viewer is evidence of meeting something else. Every throw leaves the run's
trigger as though the script had not run — a calculated value not set, a keystroke not filtered, a
format not applied — and puts one sentence per field in the view state's report, at most
`MAX_REPORTS` of them (`crates/pdf-model/src/view/scripts.rs`), so a document whose scripts throw on
every field fills the report with its first fields'.

1. **A function no file defines** (`TFMC` in one document carries nearly all of the column's runs,
   one `/C` on hundreds of fields walked at every commit; `goNext`, `f_insert`, `aaa`; a test file
   that throws on purpose). The producer's viewer meets the same throw unless the author's own
   folder-level scripts are installed; nothing here is owed.
2. **A folder-level library of the author's viewer** (`Matrix2D`, five copies of one form). The same
   reading as 1.
3. **A file's own slip**: a bare `defaultValue` where a field's property was meant, and `getField`
   of a name no field of the document has (a field deleted, a list split on `", "` leaving a space
   before every name but the first), whose `null` then throws a `TypeError`. The same throw in any
   viewer.
4. **A name the document has, spelled with something more**: trailing spaces (`"NUM_013    "` in
   five copies of one form; `"ca8 "` in a calculation Acrobat itself generated from simplified field
   notation), or a trailing period (`"auo."`, `"stpfl.kt.bank."`, in two producers' forms). The
   reference says only that `cName` is the field's name; the generated case is evidence that the
   producer's viewer answers these, and **evidence only**. **Answered by a documented choice**: a
   name that matches no field is read again without the white space at either end and the
   periods after it, and the run's notes say which field was given (ADR 1652).
5. **A root field `/AcroForm /Fields` does not list**, reached only through a page's `/Annots`
   (one PDFBOX form: most of its root fields). **Answered for the whole form**: such a root is
   recovered from the pages its widgets are on and walked as a `/Fields` entry would be, so the
   realm, a host's controls, a submission and a save all hold it (ADR 1653;
   `crates/pdf-model/examples/orphan_field_census.rs` counts the documents).
6. **A legacy engine extension the language standard does not have**: `fn.arguments` read on a
   sloppy function (`checkSSN.arguments.length`, `SYS_CB_HDL_STATE.arguments[0]`, four documents).
   ECMA-262 gives `%Function.prototype%` throwing `caller` and `arguments` accessors, and Boa follows
   it; the producer's engine kept the extension. Carrying it would be an engine change, priced
   against ADR 1590's choice of Boa.
7. **An `AF*` function neither library has**: `AFSpecial_FormatEx`, in neither Tier 0's
   `pdf_model::aform` nor the realm, and in neither source (class 3's reading). Every member the
   column once met unbridged is carried (`surface::NOT_BRIDGED` is empty, ADR 1762): `AFExactMatch`
   and the `style` and `cursor` constants as documented choices (ADR 1652); `getField("myRadio.0")`
   as one widget, a field's state one state per widget across the wire (ADR 1664); `Field.style` a
   check box's or radio button's glyph, Table 192's `/CA` holding Table D.6's code, drawn and saved
   as both of §12.7.5.2.3's states constructed (ADRs 1665, 1676); `setFocus` naming its widget, the
   host turning to the page its `/P` names (ADR 1688); `isBoxChecked` and `checkThisBox` by each
   widget's §12.7.5.2.3 on state (ADR 1689); and `this.getAnnots`, `getAnnot` and `syncAnnotScan`,
   ten properties per `Annotation` from §12.5.2's entries, the reader's edits a save writes by §7.5.6
   and Table 167's filters read as the page is drawn (ADRs 1700, 1720, 1721). `app.fs`, the
   application's full-screen preferences the `cursor` constants are written to, is refused by name
   (ADR 1665). No run of `script_corpus` reaches the annotation members: the static census's
   documents call them from Adobe's attachments shim, which calls `getAnnots` only for a
   `viewerVersion` from 6 to below 7 and so never here, where it is the workspace's release
   (`pdf_script::viewer::version`, ADR 1615), and from annotation events `script_corpus` does not
   raise.

No throw is a Tier 2 member's: every refusal the column meets is caught or carried.

## 9. What a round would do next, and in what order

RFC 0008 section 11's order is run to its end: the owner ruled on section 8 (A286), the RFC is
written and accepted (A193), the census is `javascript_census`, and Tier 0, Tier 1's (a)–(e), the
third worker, the `script` fuzz target and the `script_open` cost line are built. Three obligations
stand in place of buying ISO 21757-1, and they bind every round that writes an API member:

- **The API source is Adobe's and its details are documented choices**, never derivations —
  section 3's *What principle 5 makes of a vendor document a standard adopted* is the argument and
  its limit; a round that writes a member without that framing has quietly promoted a vendor
  document to a standard.
- **The citation shapes in section 3 bind**, because two of the four wrong spellings pass the
  checker in silence and land on real clauses of the wrong standard.
- **The language version comes from the errata, not from either standard.** ISO 21757-1 as
  published pins no ECMAScript version; issue #185 replaces its normative reference with ECMA-262's
  eleventh edition, and any engine-conformance claim is against that.

What is next is chosen from the throws' causes above.

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
