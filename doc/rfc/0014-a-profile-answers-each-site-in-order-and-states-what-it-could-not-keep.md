# RFC 0014 — A profile answers each site in order, and states what it could not keep

Status: **accepted** (A706, all five items of section 9) and **built in part**. ADR 2536 builds the
outcome per object at the embedded-file rows, a text attachment appended as pages under R15, section
3's `try` as two steps per object, and section 4's `append-statement` at the two embedded-file rows;
sections 5 and 6 are ADRs 2538 and 2539's.
Round: 1840, the design slot of batch 125, at the owner's words of 2026-10-27.
Builds on: RFC 0007 (a refusal is a question; its section 3 format, section 4's external-tool API,
section 4.6's per-target remedies, section 5a's profiles), `doc/pdf-a-mitigations.md` sections 14
and 15 (what the format cannot express), `doc/pdfa-remedies.md` (the table of every problem and
remedy, written beside this RFC), `doc/pdfa-blocking-issues.md` (what blocks a real document);
ADRs 0954, 1014, 1099, 1123, 1245, 1285, 1942, 2394, 2490; the owner's answers A54–A58, A681, A694.
Decision record: ADR 2514 (the table's reading). Asks: **Q706** (section 9).

ISO 19005 is cited by part and section and never quoted (`doc/third-party-data.md`); a `§` is a
clause of ISO 32000-2. ISO 19005-3 is not held, by the owner's answer A60, so nothing below says
what part 3 admits.

## 0. Provenance — what this RFC read

ISO 19005-2 sections 6.3.1, 6.6 and 6.8, and ISO 19005-4 sections 6.3.1, 6.9 and Annexes A and B
(`doc/pdfa/`); RFC 0007 sections 3, 4.1, 4.5 and 4.6; `doc/pdf-a-mitigations.md` sections 13.3, 14
and 15; ADR 1014 sections 3 and 5, ADR 2490 whole; the answers A57 and A58; the reader
`crates/pdf-transform/src/archive/{config,toml,remedies}.rs` where it states the grammar; the seven
files of `doc/profiles/`; and `quorra-transform archive --remedy-sites --to <target>` at all six
targets, which is the list of sites a configuration may answer and is the authority for whether a
remedy is built.

## 1. What this proposes, in one paragraph

A profile is a configuration file and nothing else, and today each site takes one answer and one
alternative (`remedy`, `on-failure`, the owner's A57). The owner's words of 2026-10-27 ask for
more: several remedies for one problem, every one choosable, a profile of the operator's own, an
external program where no remedy of this tree reaches, and a profile that accepts a loss but has a
page appended stating what was removed. This RFC proposes four changes to the format and two new
profiles. **(a)** A site's answer becomes an ordered list, `try = [...]`, evaluated per object: the
first step the target admits and that succeeds for that object is taken, and a list that runs out
stops (section 3). **(b)** A new step, `append-statement`: the loss is taken and a page is appended
stating, from the file's own facts, what was removed and why (section 4). **(c)** `derive`
generalised: an external program converts what this tree cannot read into a PDF, this converter
converts that PDF to a part the target admits, and the result is attached or appended (section 5).
**(d)** The operator's own profile becomes a first-class input, checked at every target as a
shipped one is, optionally built on a shipped one by `base` (section 6). The two profiles are
`no-data-loss`, which loses no information that can be written down, and `state-every-loss`, the
profile for the company that must archive every document it can see (section 7).

## 2. Motivation — the owner's words, and what the format cannot say

The owner, 2026-10-27: "I think we need more profiles. Like no-data-loss, which would (contrary to
as-if-printed) document metadata losses,..."; "these profiles are only configuration files, which
choose mitigations for every possible departure / failure / loss. I should be able to create my own
profile with other configurations. Every possible remedy must be choosable in a profile and we
likely should implement multiple different remedies for the same problem. I also forgot, that I
wanted that there was a way to call external tools in case remedies are not possible by use
(example: embedded office documents. An external call might convert these to pdf, which could then
be either embedded or attached)."; and "a profile should be able to for instance say that it
accepts the loss, but wants a page appended stating what had to be removed."

Four things the format as built cannot say, each already found by the catalogue:

- **An order.** `keep-everything.toml` writes `prefer = [...]` at its attachment sites and marks it
  `NOT-YET-IN-FORMAT`; `doc/pdf-a-mitigations.md` section 14 point 6 says the profile is unreadable
  without one, and point 5 found the 3D site wanting three steps (transcode, else attach the
  original, else drop the annotation).
- **A loss stated in the file.** A `discard` is named in the report and in `xmpMM:History`; nothing
  a reader of the archived file sees on a page says that something was removed.
- **A conversion that runs past the first program.** `derive` hands an attachment to a declared
  program and checks only that what comes back opens as a PDF (`remedies.rs`,
  `produced_what_it_promised`); whether that PDF itself conforms is the program's affair.
- **The operator's file checked as the shipped ones are.** `tests/profiles.rs` reads every shipped
  profile at all six targets; a `--config <file>` is read at the one target the run names, so an
  answer another target refuses is found the day a run asks for that target.

## 3. The ordered answer: `try`

```toml
[site."embedded-files/embedded-file-is-itself-pdfa"]
try = ["preserve:attach", "derive:to-pdf:attach", "derive:to-pdf:append", "append-statement"]
```

- **A step is one word of a closed grammar**: `stop`; `discard`; `append-statement` (section 4);
  `preserve:attach`, `preserve:append`; `derive:<tool>:attach`, `derive:<tool>:append` (section 5);
  `supply`; `depart:<name>`. The kind before the first colon is RFC 0007 section 2's vocabulary, so
  a reader still knows what class of thing is agreed to before reading further; the site keys a
  kind already reads (`media-types`, `construct`, `keep`) are read by that kind's step unchanged.
  The list is a single-line array of strings, which the reader's TOML subset already accepts, so
  no new syntax enters `toml.rs`.
- **A step the target does not admit is skipped, and the listing says so.** This is what makes one
  line serve six targets — `preserve:attach` keeps an office file unchanged at 4f and 4e (ISO
  19005-4 Annexes A.2 and B.4 admit an embedded file of any type there) and is passed over at 2b
  and plain 4 — and it replaces RFC 0007 section 4.6's error-naming-both for a list. **A list with
  no step a target admits is still that error**, so an answer that can never act is found when
  the file is read, not when a document arrives. The target qualifier
  (`[site."x".target."4f"]`) stays, for the operator who wants a different order at one target.
- **The list is evaluated per object**, and the report is a list: one document with five
  attachments may keep three unchanged, derive the fourth and stop on the fifth, and the report
  names each (`doc/pdf-a-mitigations.md` section 14 point 10). A step that fails for one object —
  a tool that declines, a page that cannot be composed (`doc/pdfa-blocking-issues.md`'s R14) —
  hands that object to the next step; it never changes the step another object took.
- **A list that runs out stops.** `stop` is implicit at the end, so a list ending in `discard` or
  `append-statement` is the only way a site is told to take a loss, and that is visible in one
  word at the end of one line. ADR 0954's rule that `stop` is every site's default is unchanged:
  a site absent from the file, or present with no `try` and no `remedy`, stops.
- **Every file written for A57 reads unchanged**: `remedy = "x"` with `on-failure = "y"` is read as
  `try = ["x", "y"]`, and a file stating both `try` and `remedy` is an error naming the site.

**Why this reopens A57, and why its own condition is met.** A57 adopted one alternative and said a
chain is added only when a real configuration turns out to want two steps — evidence, not
preference. The evidence is now three configurations: `keep-everything.toml`'s `prefer` at every
attachment site, the 3D site's three steps, and the owner's own sentence of 2026-10-27, which is
two steps (take the loss, then append the statement) at every site a company meets. **Q706 item 1
asks the owner to confirm** rather than this RFC taking A57's condition as its own to judge.

## 4. `append-statement`: the loss taken, and the page that says so

A step that takes the site's loss exactly as `discard` does — the same loss word, authorised by
being named, reported per object and recorded in `xmpMM:History` — and then appends one page per
document (not per object) listing every loss taken that way. What the page carries, per object:

- the site's identifier and the ISO 19005 part and section number it answers to;
- what the object was, in the file's own terms: the page it sat on, an annotation's `/Subtype`, an
  attachment's `/UF` or `/F` name, its decoded length in bytes, its `/Subtype` media type where the
  file states one, and the SHA-256 of its decoded bytes;
- the loss word, and the steps of the list tried before this one with the sentence each failed on.

**Every fact on the page is computed from the source**, as the statement on ADR 1014's signature
page is (signer, time, whether the bytes still hash — what the file's signature told a reader), and
the only words not taken from the file are a closed vocabulary — the site identifiers, the loss
words and their one-line descriptions, the field labels — fixed in this program's source, so the
page can be checked against the input exactly as ADR 1014 section 3 requires of every appended page.

**The current restriction, named.** `CLAUDE.md`'s first amendment to the authoring line admits a
page "composed solely of content the document already holds", and ADR 1014 section 5 adds that an
appended page's text is laid out verbatim "with no sentence added that the source did not
contain". The statement page holds facts about the document's own content, not that content, and
its labels are this program's. `CLAUDE.md`'s own test for the line, *does the operation invent
marks?*, answers no — nothing on the page decides what the document says — but whether the closed
vocabulary is inside the first amendment or needs its own words is the owner's to rule, and is
**Q706 item 2**, with a sentence for `CLAUDE.md` offered there.

What the page is not: a watermark (it is not laid over a producer's page), a summary of the
document, or a place for the operator's text. The profile says whether the page is wanted; it says
nothing about what the page says.

## 5. `derive` generalised: convert, then convert again, then attach or append

Today `derive:<tool>` replaces an attachment with what the program returned. The proposal makes it
a three-stage pipeline, every stage already bounded by RFC 0007 section 4:

1. **The external program** (`[tool.<name>]`, RFC 0007 section 4.1's invocation: no shell, the bytes
   in a file the converter owns, placeholders from the fixed vocabulary, the output size-bounded)
   turns what this tree cannot read — an office file, an image format no reader here decodes, a
   media clip a declared program can make a poster of — into a PDF. A54's shape stays: `apply`
   returns a request and the caller runs it.
2. **This converter converts that PDF**, with the same profile, to a part the target admits for an
   attachment: ISO 19005-2 section 6.8 admits an embedded file conforming to part 1 or part 2, and
   ISO 19005-4 section 6.9 one conforming to part 1, 2 or 4. The derived PDF is an untrusted input
   like any other and is read by the same confined reader; the recursion is one level deep and an
   attachment inside the derived file is the inner run's to answer, never a second program's.
3. **The result is attached** (`derive:<tool>:attach`, the file specification's `/AFRelationship`
   stated `/Alternative` at part 4, the report and `xmpMM:History` saying "this is derived, not
   original" as A55 requires) **or appended** (`derive:<tool>:append`, the derived PDF's pages
   appended under ADR 1014's permission — they are the program's rendering of the document's own
   attachment, which is what A55's term 3 already makes the operator state).

**A correction this reading makes to RFC 0007 section 4.6.** Its table says a derived PDF/A is not
a remedy at PDF/A-2; ISO 19005-2 section 6.8 admits an attachment conforming to part 1 or part 2,
so a derived file *converted to part 2* is one. The narrower rule is about which parts, not about
whether, and stage 2 is what lets the attachment route reach part 2. `doc/pdf-a-mitigations.md`
section 11 already lists "run the verb over the attachment" as unbuilt; this is that remedy, joined
to the program before it.

**The sandbox rule this works under** is A56's, unchanged: the declared program runs in the host
the operator configured, unconfined, and the warning stands where a `[tool.…]` block is written —
*this runs a program you chose, on a document you did not write*. What it returns enters this
program only through the confined reader, which is `CLAUDE.md` principle 3's rule that untrusted
input never reaches unsafe code, applied to the program's output as to any document.

## 6. The operator's own profile

- **`--config <file>` is checked at all six targets** before the first document is read, the check
  `tests/profiles.rs` applies to a shipped profile; `quorra-transform archive --remedy-sites
  --config <file>` with no `--to` prints the six listings and exits non-zero on the first target
  whose reading fails, so an operator validates a file once, without a document.
- **`base = "<shipped profile>"`** in `[profile]`: the file states only the sites it answers
  differently, and every other site is the named profile's. One level only — a base may not
  itself name a base — so what a site does is always found in at most two files. The listing marks
  each site *this file* or *from `<base>`*.
- **A window offers the operator's profiles** from a directory the host names, never one a
  document names, beside the shipped list (ADR 1954's one list, widened by the host's directory).

## 7. The profiles, under the table

The table's loss index sorts every loss word into **information** — something a reader could want
back: metadata, an attachment, an annotation's or a form's content, a script's source, a page's
text, a signature's assertion — and **presentation or behaviour** — a hint, a press statement, an
interactive behaviour, a choice this reader made where the file left one open. The sort is a
proposal of this RFC; it is **Q706 item 3**, because two of the profiles below are defined by it.

| profile | its sentence under this RFC | what changes |
|---|---|---|
| `refuse-any-loss` | every site stops; the lossless rewrites still run | nothing |
| `only-metadata-loss` | metadata may be discarded, nothing else | nothing |
| `as-if-printed` | discard what a print-and-scan would not carry (A694) | `append-statement` never; ADR 2490's remedies as they land |
| `as-shown` | keep what the window showed | as today |
| `keep-everything` | `try` in place of every `prefer`; never `discard` | the `NOT-YET-IN-FORMAT` keys become `try` |
| `derive-attachments`, `factur-x` | single-purpose, as today | `derive` gains stages 2 and 3 |
| **`no-data-loss`** (new) | **no information is lost: it is kept, moved, derived, or written out on an appended page; a presentation loss is discarded and stated** | new |
| **`state-every-loss`** (new) | **convert whatever the window can show: keep what can be kept, and every loss taken is stated on an appended page** | new |

**`no-data-loss`**, the owner's first-named profile. For an information loss its list is
`preserve:attach`, `preserve:append`, `derive:<tool>:attach` (where the operator declared a tool),
and then `stop`: metadata a target refuses is written out on an appended page (the packet's own
text, as `PRESERVABLE_PACKETS`' `original` already does), a script's source likewise, an
attachment kept or derived, and an attachment nothing can keep or derive stops the conversion,
naming the tool that would answer it. For a presentation loss its list is `append-statement`: the
hint or the behaviour is discarded and **stated on the appended page as well as in the report and
`xmpMM:History`** — which is "document metadata losses", taken to every loss the profile takes.
Against its neighbours: `keep-everything` discards nothing, not even a hint, and so stops on more
documents; `only-metadata-loss` discards metadata, which `no-data-loss` writes out; `refuse-any-loss`
takes no loss at all.

**`state-every-loss`**, the owner's ~20:25 sentence made a profile. Its list at every site is
`no-data-loss`'s keeping steps, then `as-if-printed`'s answer with `append-statement` in place of
`discard`, so the document converts wherever the window drew a page and every removal is on the
page an auditor reads. It stops only at `doc/pdfa-blocking-issues.md` section 7's exceptions — the
dynamic XFA form, the document with no page drawn, the unreadable — and at section 8's defects.
Whether it ships under that name, and whether `no-data-loss` and it are two profiles or one with a
key, is **Q706 item 4**.

## 8. Easy and difficult, against this tree

- **Easy**: `try` is a list of strings the reader already parses; `config.rs` keeps its per-kind
  site readers and gains one function that maps a step word to them. `base` is a merge of two
  `Configuration`s before validation. The six-target check is the loop `tests/profiles.rs` runs.
- **Moderate**: per-object evaluation changes `Report`'s outcome from a verdict per site to a list
  per object (section 14 point 10's finding, owed regardless); `append-statement`'s page reuses ADR
  1014's text page and inherits its open cases (R14: a face for every character, `DeviceGray` text,
  a packet that is not UTF-8).
- **Difficult**: stage 2 of section 5, a conversion inside a conversion, needs the budgets of the
  outer run divided rather than doubled, and the inner report folded into the outer one; and the
  Level A targets owe a structure element for every appended page (A58).
- **What it does not touch**: the validator, `pdf_syntax::Document`'s immutability, ADR 0816's rule
  that no producer's content stream is edited, and the sandbox's system-call set.

## 9. Questions for the owner — `doc/questions/Q706`

1. **The ordered list (A57 reopened).** May a site take `try = [...]`, evaluated per object, the
   first admitted step that succeeds taken, a run-out list stopping? *Recommendation: yes; A57's
   own condition — a real configuration wanting two steps — is met three times.*
2. **The statement page.** Is a page carrying only facts computed from the file, under a closed
   vocabulary fixed in this program, inside `CLAUDE.md`'s first amendment, or does it need its own?
   *Recommendation: inside it, with one sentence added to the amendment, offered in Q706.*
3. **The sort of losses** into information and presentation, the table's loss index. *Recommendation:
   adopt it as written, each word's class open to the owner's correction by name.*
4. **The two profiles.** Ship `no-data-loss` and `state-every-loss` as defined in section 7?
   *Recommendation: both, `state-every-loss` as the profile a company is pointed at first.*
5. **The generalised derive and the operator's profile** (sections 5 and 6), including the
   correction to RFC 0007 section 4.6. *Recommendation: yes, built after items 1 and 2.*

## 10. What is built when

Nothing in this round. In the order the table ranks them: `try` with its per-object report (after
item 1); `append-statement` (after item 2); the two profiles (after items 3 and 4); the six-target
check of a `--config` and `base` (no answer needed beyond item 1); stage 2 of `derive` last. The
two reasons the owner named first — an attachment no reader here draws, and multimedia — are batch
126's to build, each a row of the table with its list of steps.
