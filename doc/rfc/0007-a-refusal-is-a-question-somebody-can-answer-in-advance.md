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

## 4.7 Departures — going against the standard, on purpose and by name

Asked for by the owner on 2026-09-11:

> I would also like the option, to go against the spec and for instance accept xml (and only xml)
> attachments when targeting pdf/a 2.

**This is not a remedy and must not be modelled as one.** Every remedy in §2 produces a file that
conforms to the target; a departure produces one that does not. Putting them in the same table
would make the configuration's most important distinction invisible.

### 4.7.1 Why it is a real request rather than a shortcut

`doc/pdf-a-conversion-limits.md` §3.1 says of a non-PDF attachment that there are "exactly two
honest answers — retarget to PDF/A-4f, or drop the attachment. There is no third one." A third
exists in the standard — **PDF/A-3** — and this project cannot target it because part 3 was never
bought (`A17`).

**But retargeting is not the same request, and the owner's correction on 2026-09-11 is why.** Part
3 is understood to relax the embedding rule for **any** file, as PDF/A-4f does; neither is a
narrowing. *(Understood rather than read: part 3 is not held here, so this RFC records it as the
owner's reading and the common account rather than as a clause this project has checked.)*

So an operator who wants *PDF/A-2's discipline plus exactly XML* cannot get it from any target.
PDF/A-3 and PDF/A-4f would both also admit a spreadsheet, an executable, or a video — and an
archive that asked for PDF/A-2 has a policy about what may be in its files that "any attachment"
does not honour. **A departure can be narrower than any target**, and that is the strongest
argument for building it: retargeting trades one rule for a weaker one, where a departure keeps
every rule but the one it names, for only the class of file it names.

The concrete case is current. ZUGFeRD and Factur-X — the same specification since 2020, with a
German mandate arriving in 2026 — embed a machine-readable invoice as `factur-x.xml` in a
**PDF/A-3** with `/AFRelationship /Alternative`. An operator whose archive mandates PDF/A-2 and
whose invoices carry that XML has a genuine conflict that no remedy resolves: dropping the XML
loses the invoice's machine-readable half, and retargeting is not theirs to decide. *(That is
evidence about demand and convention, which the RFC conventions admit as its own register; it says
nothing about what any clause requires.)*

### 4.7.2 The three properties that make a departure honest

**It is named, per requirement, and never blanket.** There is no "ignore errors" and no severity
threshold. A configuration departs from `attachments/embedded-file-is-itself-pdfa` or it does not,
and a requirement it does not name is enforced exactly as today.

**It carries a narrowing predicate, and the owner's "(and only xml)" is precisely that.** A
departure is not permission to embed anything; it is permission to embed what the operator named.
The predicate's shape is departure-specific, like a remedy's keys:

```toml
[depart."attachments/embedded-file-is-itself-pdfa"]
media-type = ["application/xml", "text/xml"]
relationship = ["Alternative"]           # the ZUGFeRD shape, narrowed further
reason = "Factur-X invoices; our archive accepts them"
```

`reason` is required and is copied into the report. A departure nobody wrote a reason for is one
nobody will be able to explain in two years.

**And the output does not claim what it has not earned.** This is the load-bearing property. A
file that states `pdfaid:part 2` while carrying a forbidden attachment is *asserting something
false about itself* — and this converter's existing discipline is that no file is written "wearing
a claim it has not earned". So by default a departed conversion **omits the PDF/A identification
schema**, and the report says so in those words: the output is a PDF that meets PDF/A-2 in every
respect but the ones listed, and it does not claim to be PDF/A-2.

Whether an operator may demand the claim anyway is `doc/questions/Q59`, and it is the sharpest
question in this proposal — because a validator downstream will fail the file either way, and the
difference is only whether the file *lied* before it failed.

### 4.7.3 What does not change

Stage three's net still runs, and is not switched off. It re-opens the output, holds it to the
target, and reports every requirement it fails — the departed ones **by name, as departures**, and
anything else as the failure it is. A departure narrows what counts as success; it does not stop
the converter checking.

Every departure is reported per document and recorded in the file's own `xmpMM:History`, so the
archive carries the fact rather than relying on a report nobody kept.

And the invocation has to say so. A configuration file can be inherited, copied between teams or
written by somebody who has left; **the verb refuses a departing configuration unless the command
line also carries `--depart-from-the-standard`**, whose only job is to put the operator's intent at
the call site rather than only in a file. That is a deliberate exception to §3's rule that the
configuration is the whole answer, and it is worth the exception: every other entry in that file
makes a conforming file, and this one does not.

### 4.7.4 The question underneath it

**Part 3 would answer the ZUGFeRD case and not the general one**, and the difference is worth
keeping straight. A Factur-X invoice is a PDF with one XML attached, so a PDF/A-3 target holds it
and says so — no departure, no omitted identification, a file that conforms. That is the better
outcome whenever it is available, and `doc/questions/Q60` asks whether to obtain part 3 for it.

It does not answer the owner's request, because *and only xml* is the request. Part 3 and PDF/A-4f
both admit any embedded file; neither expresses "these and nothing else". So the two are
complementary rather than alternatives: part 3 is the right answer for an operator who wants the
invoice case supported, and departures are the only route for one whose archive mandates PDF/A-2
itself, or who wants a permission narrower than any part grants.

### 4.7.5 One departure was discussed; every requirement needs the same consideration

Stated by the owner on 2026-09-11:

> note, that we have just discussed this single possible exclusion of the spec. There are probably
> a lot others where different ways of "ignoring" the spec make sense. We need to think in every
> case, what could make sense.

The XML attachment is one instance and it is not special. **Every requirement a target binds is a
candidate for a departure, and each needs its own consideration of what departing would mean** —
exactly as §5 says every refusal needs its own consideration of remedies.

That was not a tractable sentence a week ago. It is now, because session 955 built
`crates/pdf-transform/src/archive/census.rs`: for the first time there is a **complete enumeration**
of what every target is held to, 151 to 167 requirements each, with none reaching a catch-all. A
departure catalogue can be built against that list rather than against whatever a corpus happened
to raise.

#### The method, and one third of it the census can compute

Sorting a requirement by **what departing from it costs** gives three kinds, and the first is
objective rather than a judgement:

**A — another part of ISO 19005 already relaxes it.** The census knows which requirements bind
which targets, so it can answer this mechanically: ISO 19005-2 section 6.1.13's ten implementation
limits bind part 2 and **part 4 states none of them**; part 4 removes `/Info`; part 2 and part 4
differ on `/DefaultCMYK`, on associated files, on what an embedded file may be. A departure here
has precedent inside the standard itself — the committee has already judged, somewhere, that a
conforming file need not have this. That is the strongest ground a departure can stand on, and it
is free to compute.

**B — no part relaxes it, and departing still leaves an archive.** A judgement, argued per
requirement. The XML attachment is here: no part of ISO 19005 expresses "PDF/A-2 plus exactly XML",
but a file that is PDF/A-2 in every other respect and carries one XML attachment is plainly still
an archival document. Most of the interesting cases will be of this kind and each one is a small
argument rather than a lookup.

**C — departing defeats what the format is for.** PDF/A exists so a file renders the same in
decades, and some requirements are load-bearing for exactly that. **Font embedding** is the
clearest: a file whose fonts are not embedded may not render at all in twenty years, which is the
whole thing being prevented. **Encryption** is another — an archive nobody can decrypt is not an
archive. **A stream whose data is in an external file** is a third. A departure here does not
produce a slightly different archive; it produces something that is not one.

This project should say so rather than offering the switch and letting an operator discover it.
The catalogue's entry for a kind-C requirement is *no departure, and here is why* — which is the
same shape as `REFUSED_BY_NAME`'s fence rows, and for the same reason: a refusal with an argument
is finished work.

#### And the second axis: can it be narrowed?

Cutting across those three is whether a departure admits a **predicate**. The owner's "(and only
xml)" is the model, and it is what makes a departure safer than retargeting — but not every
requirement has a natural narrowing. "The file shall have a conforming header" is all or nothing.
A requirement that cannot be narrowed is a blunter instrument and its catalogue entry should say
so, because an operator choosing between two departures should be able to see which one is
narrower.

#### What this means for the work

The catalogue is the bulk of this feature, not the mechanism. There are 107 requirements the census
newly enumerated plus the 40-odd already answered, and each needs a sentence about departure even
where the sentence is "no". That is several rounds of reading, and it is the same reading the
remedy table needs — which argues for doing them together, requirement by requirement, rather than
as two passes over the same clauses.

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

## 5a. Shipped configurations, which are the feature most operators will actually use

Proposed by the owner on 2026-09-11:

> I also think that we will provide different configurations for different use cases:
> `as-if-printed.conf` (which just archives as if the user printed it and ignores any loss which
> would have been lost, if the user printed the file), `only-meta-info-loss.conf` …

**These need no new mechanism**, which is the strongest thing about the idea. A profile is a
configuration file this project ships; §3's format is already the whole of it. What they add is
that almost nobody wants to answer a hundred and fifty questions, and almost everybody can say
which of half a dozen sentences describes their archive.

### 5a.1 `as-if-printed` is derivable, not a taste

Most policies would be somebody's opinion about what matters. This one is not, and that is what
makes it the best of the proposed profiles: **the standard states what a printed page shows**, so
the profile's content can be read out of clauses rather than argued.

- §12.5.3's Table 167, bit 3: "If set, print the annotation when the page is printed unless the
  Hidden flag is also set. If clear, never print the annotation, regardless of whether it is
  rendered on the screen." So an annotation the file marks unprinted is, under this profile,
  something the user already accepted losing.
- The same row's next sentence bounds it: "If the annotation does not contain any appearance
  streams this flag shall be ignored."
- §8.11.4.4's usage application dictionary takes an `Event` of `View`, `Print` or `Export`, and a
  group's `Usage` may carry a `Print` dictionary whose `PrintState` "shall be either ON or OFF,
  indicating that the group shall be set to that state when the document is printed". So optional
  content that does not print is likewise already-accepted loss.
- And everything that reaches no printed page at all — attachments, JavaScript, multimedia
  streams, the metadata packet, the document's own restrictions — is loss the user accepted the
  moment they pressed print.

The profile's sentence is therefore short and checkable: **discard what printing would not have
carried; touch nothing that it would.**

### 5a.2 And it interacts with the target, which is the part to get right

`as-if-printed` is *incoherent* with two of the six targets, and saying so is more useful than
shipping a file that quietly under-delivers:

- **PDF/A-2a** is Level A: it requires the logical structure that describes the content. Printing
  carries none of it. A profile that discards what printing loses would discard the very thing
  that target is for.
- **PDF/A-2u** requires every text-showing operation to map to Unicode. A printed page carries the
  glyphs and not the mapping.

So a shipped profile declares the targets it is coherent with, and using it against another is an
error naming both — the same rule §4.6 already sets for a remedy the target does not admit. This
is the second time that pair has turned out to be the unit rather than the site.

### 5a.3 The profiles worth shipping, and what each says in one sentence

| profile | its sentence | coherent with |
|---|---|---|
| `refuse-any-loss` | every site `stop`; today's behaviour, named so an operator can state it deliberately | all six |
| `as-if-printed` | discard what printing would not have carried; touch nothing it would | 2b, 4, 4f, 4e |
| `only-metadata-loss` | nothing may be lost but metadata; everything else `stop` | all six |
| `keep-everything` | prefer `preserve` wherever it exists, then `derive`, never `discard`; `stop` rather than lose | all six |

`keep-everything` is the one that most needs §4.6.1's finding — appending is available under every
target, so "keep it somewhere" is nearly always possible — and it is the profile an operator
reaches for when the archive matters more than its tidiness.

### 5a.4 Shipping conversion programs beside them

> we could even provide conversion program, which for instance convert embedded files

Worth doing and worth keeping separate from the profiles, because it is a **packaging and licence**
question rather than a design one. The lesson is a week old and cost a round: shipping
`data/icc/sRGB2014.icc` needed a first-hand reading of the ICC's terms, a provenance file with a
hash, and a `/NOTICE` section — and the obvious download turned out to be the wrong file.

Two shapes, and the second is much cheaper:

- **ship a program** — a converter binary or script this project distributes. Every dependency it
  has becomes ours to license, notice and keep working;
- **ship a tool *declaration*** — the `[tool.…]` block for a program the operator already has, so
  `soffice`, `ffmpeg` or `pandoc` can be configured correctly without the operator working out the
  argument order. That is a text file, carries no third-party bytes, and is where the real
  friction is anyway.

The second should come first, and the profiles should reference tools by name so that a declaration
can be dropped in beside them.

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
