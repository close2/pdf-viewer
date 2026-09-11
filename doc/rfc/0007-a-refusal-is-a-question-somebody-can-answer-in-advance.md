# RFC 0007 — A refusal is a question somebody can answer in advance

Status: **proposed**
Round: 954 — commissioned by the owner on 2026-09-11: *"This tool will probably be used in
automatic environments, where a refusal would mean that a human has to intervene. I think we should
allow a configuration file, where every possible refusal reason can be configured."*
Companions: RFC 0002 (the transform suite, whose §9 determinism claim this proposal strains and
must therefore answer), RFC 0006 (PDF/A validation and conversion, ratified by `A46`).
Amends: `doc/adr/0947` §the-middle-stage, `doc/adr/0951` §`REFUSED_BY_NAME`, `doc/adr/0952`.

---

## 0. What is wrong today, in one paragraph

`quorra-transform archive` answers every failed requirement with one of five decisions, and one of
them — `Refused(Because)` — is **terminal**. The verb writes nothing and names the requirement. For
a person converting one document that is the right behaviour and the project has defended it
carefully: a file that leaves the verb conforms, and a refusal with a sentence is finished work
rather than a gap.

**For a queue it is not a behaviour at all.** An automated archiving pipeline that meets a refusal
has no next move except to stop and wait for a person, and the whole point of the pipeline is that
there is no person. The owner's sentence is the motivation and it is worth keeping verbatim: *a
refusal would mean that a human has to intervene.*

## 1. The reframe

**A refusal is not a verdict. It is a question the converter is asking, and today the only place to
answer it is a human at a terminal.** The proposal is to let the answer be given in advance, in a
file, per question.

Three consequences fall out immediately, and they are why this is a reframe rather than a flag:

1. **Every refusal needs a stable name.** Today a refusal is identified by a requirement
   identifier plus a `Because` kind, and the sentence is a `&'static str` written at the site.
   A configuration can only speak about things that have names.
2. **`stop` becomes one answer among several, not the absence of an answer.** It stays the
   default, because a converter that silently did something else to a document nobody asked it
   about would be the worse failure.
3. **The answers are not all "lose it".** This is the owner's substantive point and the part this
   RFC exists to work out: *losing information is the easiest remedy, not the only one.*

## 2. A taxonomy of remedies, because "lose it or stop" is a false pair

Sorting the possible answers by **what happens to the document's content** gives four kinds, and
the sort is load-bearing — it is what tells a user what they are agreeing to.

| kind | what it does | example |
|---|---|---|
| **`stop`** | nothing; the conversion refuses, as today | the default for every site |
| **`discard`** | the information is lost | drop a metadata property its own schema does not define |
| **`preserve`** | the information survives, in a form the target admits | append an embedded image as a page; render a metadata packet as a prefixed page |
| **`derive`** | a *new representation* is made from content the document already has | an office attachment converted to PDF; a movie's poster frame; an audio transcript |

`discard` exists today as `Decision::Authorised`/`Unauthorised` and §3 of
`doc/pdf-a-conversion-limits.md`. **`preserve` and `derive` are new**, and they are the owner's
examples: *for embedded images we could provide a remedy, that we append the images to the
document*; *instead of losing metadata it could be appended or prefixed as an extra page*; *an
external program could generate screenshots of the movie, or an audio transcription*.

### 2.1 Why `preserve` and `derive` are different, and must stay different

A `preserve` remedy moves information that is already in the document into a place the target
admits. Nothing is invented and nothing is lost; what changes is *where* the information is and
what a reader has to do to get at it. An embedded PNG appended as a page is the same image.

A `derive` remedy makes something that was not in the document before. A transcript of an audio
track is not the audio track. **This is the boundary `A48` drew** — *state an interpretation the
standard defines; never fill in an absence* — and a `derive` remedy stands on the far side of it.

That does not make it wrong. It makes it a different promise, and the honest position is:

- a `derive` remedy is **never** a default and never reachable without the configuration naming it;
- the report says, per document, what was derived, from what, and by which tool;
- `xmpMM:History` records it in the file itself, so the *archive* carries the fact that part of it
  is derived rather than original;
- and the archived document is no longer a faithful copy of the original in that respect, which
  the report must say in those words rather than leaving a user to infer it.

A PDF/A file whose movie has become three screenshots is a *different document* from the one that
went in. The converter may produce it; it may not pretend otherwise.

## 3. The configuration

One file, named on the command line (`--remedies <file>`) or found by a documented search, in a
format this tree already parses. **TOML**, because `doc/conformance/ledger.toml` and
`doc/checks/*.toml` already are and the workspace holds a parser.

```toml
# Every key is a refusal site's identifier. Absent means `stop`.
[site."attachments/embedded-file-is-itself-pdfa"]
remedy = "derive"
tool   = "office-to-pdf"
on-failure = "discard"          # or "stop"

[site."metadata/properties-use-known-schemas"]
remedy = "discard"

[site."annotations/multimedia-is-not-permitted"]
remedy   = "derive"
tool     = "movie-poster"
placement = "append"            # remedy-specific, and the site documents its own keys

[tool.office-to-pdf]
program = "/usr/bin/soffice"
args    = ["--headless", "--convert-to", "pdf", "--outdir", "{out}", "{in}"]
expects = "application/pdf"
timeout = "60s"
output-limit = "256MiB"
```

Three properties of that shape are deliberate:

- **A site absent from the file behaves exactly as today.** Adding the file changes nothing until
  it names a site, which means the feature cannot alter an existing pipeline by being installed.
- **`remedy` is from the closed vocabulary of §2**, so a reader of a configuration knows what class
  of thing is being agreed to before reading the site-specific keys.
- **Tools are declared once and referenced by name**, so the same converter can be described for a
  fleet without repeating a path, and so a reviewer can see every external program a configuration
  can run by reading one section.

### 3.1 Every site is enumerable, and that is a gate

`quorra-transform archive --remedy-sites` prints every site, its default, and the remedies it
supports. The list is generated from the same table the converter decides from, so a site cannot
exist without being documented, and a configuration naming a site that does not exist is an error
rather than a silently ignored line. Session 954's coverage census is the instrument that makes
this enumerable at all.

## 4. The external-tool API, which is the part that needs care

The owner asked for this explicitly: *we probably want to take some time and think about a good API
for calling external tools, to keep it "mostly" consistent, but every refusal needs its own
consideration of remedies.* Consistent skeleton, site-specific flesh.

### 4.1 The invocation

- **No shell.** `program` plus an `args` array, executed directly. A shell would make every
  document-derived string an injection site.
- **Document-derived bytes go in on stdin, or in a file the converter creates in a directory it
  owns.** Never in `args`.
- **`args` may contain placeholders from a fixed vocabulary**, substituted by the converter and
  never taken from the document: `{in}`, `{out}`, `{media-type}`, `{target}`, `{site}`. A
  placeholder the site does not define is a configuration error.
- **The result comes back on stdout, or in `{out}`** — the site says which, because a tool that
  writes one file and a tool that writes a directory of frames are different shapes.
- **stderr is captured and put in the report verbatim**, truncated with the truncation stated. A
  tool's own explanation of why it declined is the most useful thing a report can carry.
- **Exit status**: `0` means it produced a result; a documented code means *I decline, take the
  fallback*; anything else is a failure. `on-failure` decides between `stop` and the site's other
  remedies, and defaults to `stop`.

### 4.2 What comes back is not trusted

**`expects` is declared in the configuration and checked by the converter**, because a tool
returning something other than what it promised would otherwise put arbitrary bytes into an
archive. A declared `application/pdf` that does not open as a PDF is a tool failure, not a
document.

And whatever comes back goes through **the net that is already there**: ADR 0947's third stage
re-opens the assembled output and holds it to the same target. A derived PDF that is not itself
conforming is caught there, by machinery this converter already has and already trusts.

### 4.3 Bounds, because an external program is unbounded by nature

`timeout` and `output-limit` are required of every tool declaration — not optional with a default,
because the right value is a property of the tool and a wrong default is worse than an absent one.
They join `Budget`, which RFC 0002 §5 already threads through `apply`.

### 4.4 Determinism, which this proposal genuinely strains

RFC 0002 §9 claims `apply` is a pure function of `(sources, plan, policy, budget)` — *no
filesystem, no clock, no environment* — and calls that claim "a test rather than a demo". **An
external program breaks it, and this RFC will not pretend otherwise.**

The proposal:

- the claim is **narrowed, in writing, to a conversion that invokes no tool**, which is every
  conversion today and every conversion where the configuration names none;
- a conversion that invokes a tool records, in the report, each tool's name, its resolved program
  path, and a digest of what it returned — so that a re-run can be *checked* even though it cannot
  be *guaranteed*;
- the determinism test keeps its current subject and gains a second case asserting that a
  tool-invoking conversion is deterministic **given a recorded tool output**, which is the strongest
  true statement available.

### 4.5 Trust and confinement

The program is named by the operator's own configuration, so the operator trusts it. The *input* is
an untrusted document. `CLAUDE.md` principle 3 confines this project's own renderer for exactly
that reason, and the honest position is that we cannot confine somebody else's program to the same
standard — LibreOffice will not run under our seccomp profile.

What is proposed instead, and what it is worth saying plainly:

- the converter creates the working directory, passes only paths inside it, and removes it after;
- nothing from the document reaches `args`;
- the tool's output is size-bounded before it is read;
- and **the documentation says, in the place an operator configures a tool, that they are choosing
  to run that program on untrusted input.** An operator running `soffice` over a queue of documents
  from the public has made a security decision, and the configuration file is where they should be
  told so.

Whether the converter should *additionally* offer to run a tool under the project's own sandbox for
the cases where that is possible is an open question (§7).

## 4.6 A remedy belongs to a *site and a target*, not to a site

Stated by the owner on 2026-09-11, and it changes the shape of the table rather than adding a row
to it:

> the provided remedies will also differ (slightly) based on the selected target. As we could
> insert an embedded pdfa for variant pdfa/4 but not for pdfa/2 (and pdfa/4f would allow any file
> as embedded file to avoid losing information)

The clauses bear that out exactly. ISO 19005-4 section 6.9 requires every embedded file to conform
to ISO 19005-1, -2 or -4, so a **derived PDF/A** may be attached under PDF/A-4 and the information
survives. ISO 19005-2 section 6.8 is narrower about what may be embedded at all, so the same
derived file is not a remedy there. And PDF/A-4**f** exists precisely to carry arbitrary embedded
files — under that target the original office document can be attached **unchanged**, and nothing
is lost or derived.

So one site has three different menus depending on what was asked for, and the best available
answer is different in kind at each:

| target | the embedded-file site's best remedy | what happens to the information |
|---|---|---|
| PDF/A-4f | attach the original, unchanged | **nothing is lost and nothing is derived** |
| PDF/A-4 | derive a PDF/A and attach that | the *content* survives, in another format |
| PDF/A-2 | derive a PDF if it is not one, and **append its pages** | the content survives, in the document's body |

**The last row was wrong in this RFC's first draft, which said PDF/A-2 had neither remedy
available.** The owner corrected it on 2026-09-11: *appending pages for target 2 could also be an
option.* It is, and the reason is worth stating because it generalises — **what the targets differ
about is what may be *attached*, not what may be a page.** Every one of the six admits as many
pages as a document likes. So the restriction that closes the attachment route at PDF/A-2 does not
touch the appending route at all.

### 4.6.1 Appending is available everywhere, which makes the target dimension about *mechanism*

Once that is seen, the table above is not "which targets have a remedy" but "which mechanism each
target's best remedy uses", and appending is a *fourth* column available under all six:

| mechanism | available | what it costs |
|---|---|---|
| attach unchanged | 4f only | nothing |
| attach a derived PDF/A | 4, 4f | the original format |
| **append as pages** | **all six** | the content becomes body rather than an attachment |

Appending is therefore an operator's choice rather than a fallback: somebody archiving to PDF/A-4
may still prefer the content *visible in the document* over an attachment a reader has to go
looking for, and the configuration should let them say so.

### 4.6.2 What appending costs, which is not nothing and differs by target

A page is cheap to add and expensive to add *consistently*, and the costs are per target:

- **the page count changes**, so §12.4.2's page labels and §12.3's outline no longer describe the
  document unless they are extended. Neither is a conformance requirement, and both are
  user-visible, so silently leaving them stale is the wrong answer even where it conforms;
- **PDF/A-2a and any Level A target need the structure tree to cover the new pages.** ISO 19005-2
  section 6.7 requires the logical structure to describe the content, and appended pages with no
  structure elements are content the tree does not describe. So appending at 2a is not the same
  operation as appending at 2b — it costs structure-tree work, or it costs the Level A claim;
- **an appended page is marks no clause specifies**, which is `Q58`'s whole subject. The content is
  the document's own, which is why this is `preserve` and not `derive`, but the page it sits on is
  composed by this program.

None of that argues against appending. It argues that the remedy's entry in the per-site table has
to say *which target* and *what else it then owes* — which is what makes the site-and-target pair
the unit, rather than the site.

Three things follow from all of this.

**`--remedy-sites` takes a target**, because the list it prints is a property of the pair. A
configuration written against one target and used with another must not silently do less than it
says: a remedy the chosen target does not admit is a **configuration error naming both**, not a
quiet fall-through to `stop`.

**The configuration may say so per target**, and the obvious shape is that the site table takes an
optional target qualifier, with the unqualified entry as the default:

```toml
[site."attachments/embedded-file-is-itself-pdfa"]
remedy = "discard"                      # wherever nothing better is available

[site."attachments/embedded-file-is-itself-pdfa".target."4f"]
remedy = "preserve"                     # 4f can hold the original

[site."attachments/embedded-file-is-itself-pdfa".target."4"]
remedy = "derive"
tool   = "office-to-pdf"
```

**And the report should say when another target would have kept what this one loses.** This is the
most useful thing in the whole proposal for an operator, and it costs almost nothing: the converter
already knows every target's requirement table, so it can answer "the target you asked for cannot
hold this; PDF/A-4f can" without doing any extra work.

It must **not** switch targets to get it. `doc/pdf-a-conversion-limits.md` §9 exists because the
owner said so directly — *switching levels is not a way to avoid problems; usually we have to
target a specific level and we need to document the limitations* — and an archive that requires
PDF/A-4 is not served by being handed a 4f file. But an operator who learns, once, that their queue
would lose nothing at 4f can change their own policy, and that is a decision only they can make.

## 5. Per-site remedies, first pass

**Read every cell below as "for the targets that admit it"** — §4.6 is why, and the embedded-file
row is the clearest case rather than the only one.


Every site needs its own consideration; this is the first pass over the ones that exist today, and
the list is the work rather than the shape.

| site | `discard` | `preserve` | `derive` |
|---|---|---|---|
| embedded file not itself PDF/A (§3.1) | drop the attachment | **4f: attach unchanged. All six: append as pages** (§4.6.1) | **4: derive a PDF/A and attach.** 2: derive a PDF and append it |
| multimedia and 3D annotations (§3.2) | drop the annotation | keep the poster image the annotation already carries | poster frame, key frames, or a transcript, appended as pages |
| JavaScript and behavioural actions (§3.3) | drop the action | — | — (a script's *text* as a page is information nobody asked to archive) |
| encryption (§3.5) | decrypt and drop the permissions | — | — (no tool helps; the information is the restriction itself) |
| digital signatures (§3.6) | drop the signature | **append the signer, time and validity as a page**, which is what a signature *told* a reader | — |
| metadata property outside its schema (§3.9) | drop the property | **prefix or append the packet as a page**, the owner's own example | — |
| a font nothing can substitute (§2.1) | — | — | — (`stop` is the only honest answer; a wrong glyph is not a remedy) |
| an image codec the target forbids | — | transcode to a permitted codec, which is `preserve` | — |

Two entries in that table are worth their own sentence. **A signature's remedy is `preserve`, not
`discard`**, because what a signature conveys to a reader is a statement — who signed, when, and
whether it verified — and that statement can be written on a page even though the cryptography
cannot survive. And **a font has no remedy**: §2.1 already argues that substituting a face whose
glyphs are wrong is not a repair, and no external program changes that.

## 6. Easy and difficult, against this tree

**Easy**, because the architecture already has the seams:

- the decision table is already keyed by requirement identifier and already carries a sentence per
  refusal — naming the sites is mostly mechanical;
- `Decision` already has five variants and one more class is not a redesign;
- `Authorisations` already models "the user agreed to a loss", one field per loss, so that adding
  one is a compile error everywhere it must be answered — the configuration is that idea widened;
- stage three already re-validates whatever the rewrites produced, which is exactly what a derived
  artefact needs;
- `Budget` and `Policy` already thread through `apply`.

**Difficult**, and each of these is a decision rather than a task:

- **`apply` currently opens no path and runs no process**, by RFC 0002's second rule. A tool
  invocation breaks both. The clean shape is that `apply` does not run the tool either — it returns
  a *request*, and the caller runs it — which keeps the library pure and puts the process boundary
  in the binary. That is more work and a better boundary, and §7 asks the owner.
- **`derive` changes what the archive is**, and a project whose whole discipline is "state an
  interpretation the standard defines; never fill in an absence" should not add a content-creating
  mode without the owner ruling on it directly.
- **The remedy vocabulary has to be closed and small**, or a configuration becomes a program.

## 7. Open questions for the owner

1. **Does `apply` run the tool, or return a request for the caller to run?** The second keeps RFC
   0002 §5's purity — no filesystem, no process — and makes the CLI the only thing that spawns.
   It costs a round trip per invocation and complicates streaming. This RFC recommends the second
   and does not assume it.
2. **Is `derive` acceptable at all for an archival format?** A movie replaced by three screenshots
   is not the document that went in. The proposal is that it is acceptable *only* when configured,
   reported, and recorded in the file's own history — but the owner should say whether this
   project offers that mode at all.
3. **Should a tool run under this project's sandbox where it can?** Some tools would; most will
   not; offering it for some invites the belief that it is offered for all.
4. **What is the fallback chain's shape?** `on-failure` as proposed picks one alternative. A list
   would be more expressive and much harder to reason about from a configuration file.
5. **Does a `preserve` remedy that appends pages need the owner's ratification separately?**
   `CLAUDE.md`'s authoring exclusion was amended twice by argument; appending a page of derived
   text is close to its line, and the amendment record says such a move is "its own argued
   amendment, not scope creep".
