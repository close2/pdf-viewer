# 940 — Two parts of ISO 19005, read by the program that needs them

Date: 2026-09-07.
ADR: 0920 (§14.7's tree handed out as items, and the artifact filter that could not be applied to
it); 0921 (where ISO 19005-4's words break, and the two repairs that were not it); 0922 (when
veraPDF decides, and when the clause does); 0923 (a validator that reports what it did not check).
Question: `doc/questions/Q16`/`A16` — the owner bought parts 2 and 4 and asked for them to be
prepared, "maybe our own viewer is suitable for extracting text".
Files: `tools/pdf-retrieve/src/lib.rs`, `tools/pdf-retrieve/src/main.rs`,
`tools/pdf-retrieve/tests/retrieval.rs`, `tools/pdfa-text.py`,
`doc/rfc/0006-pdf-a-validation-and-conversion.md`, `doc/pdf-a-conversion-limits.md`,
`doc/adr/0920-*.md`, `doc/adr/0921-*.md`.
Not committed, and not committable: `doc/pdfa/` is ignored, because the two standards are
licensed to one reader.

The owner's guess was right, and it was right for a better reason than proximity. Both files are
AES encrypted, both are tagged, and §14.7's structure tree states where their paragraphs, clause
headings, lists and tables are — so reading them with `quorra-retrieve` gives the document's own
paragraphs rather than lines guessed back into shape, and never sees a running head, a folio or
the rotated line naming the licensee, because §14.8.2.5.1 NOTE 3 keeps an untagged artifact out
of the logical content order. `tools/pdfa-text.py` is 300 lines of formatting over one question.

Asking that question found two things this tree did not know about itself.

## A flag that cut the text it was filtering

`--logical --no-artifacts` had been removing characters from the middle of the answer — the first
62 of ISO 19005-2's page 26, and the same width off the top of every page of ISO 32000-2 — because
the artifact ranges are offsets into the raw readback and the logical reading is those characters
in another order. Silent, and present for as long as both flags have existed. ADR 0920; the fix is
one condition and the clause that makes it one condition is NOTE 3.

## A word break that is a choice, and 1914 of them

ISO 19005-4 reads back with 1914 single letters that are not words, against 63 in part 2, because
its producer sets a fragment at a time with tracking and this reader infers a break from the gap.
§14.8.2.6.2 says whose problem that is — a tagged producer shall state its own word breaks, so
that a processor need not "rely on heuristics based on information such as glyph positioning on
the page" — and part 4 does not. Two repairs were tried and measured: doubling the threshold works
and is forbidden curve-fitting, and taking the space width from `code_for(' ')` looks like the
right fix and makes it three times worse. Both are written down in ADR 0921 so that the hour is
spent once. Nothing was repaired by guesswork; the number is printed in the extracted file's own
header instead.

## And then the first thing the text was used for

The owner's next sentence was that they intend to build the converter, and asked for the
limitations a user has to be told about. `doc/pdf-a-conversion-limits.md` is that list, written
from the two parts rather than from anybody's summary of them, with every entry classified
**refuse / ask / default / mechanical** and the sensible default named where one exists.

Three findings in it could not have been made a week ago, because they needed the normative text:

- **PDF/A-4 states no implementation limits.** Part 2's §6.1.13 caps page boundaries at 14 400
  units, `q`/`Q` nesting at 28 and DeviceN at 32 colourants; part 4's §6.1 runs 6.1.1 to 6.1.12 and
  has no such subclause, which its own contents list confirms. A large-format drawing can be
  PDF/A-4 and cannot be PDF/A-2.
- **PDF/A-4 permits JavaScript** (§6.6.2, user-invoked only) and `/AA` on widgets (§6.6.3), both of
  which part 2 forbids outright — and §6.4.1 states an archival path for form logic as an embedded
  XFDF file, which is itself an embedded file and therefore forces PDF/A-4f. The standard offers a
  workaround whose own target it constrains.
- **The transparency cliff is gone.** Flattening was `doc/rfc/0006` §5.2's hardest case and it
  belongs to PDF/A-1 alone; `A17`'s "part 1 never" removed the occasion for it, which is also what
  `Q19` was asking about.

The user-facing shape of the whole thing is one sentence: the most useful answer a converter can
give is not "this failed" but *this cannot be PDF/A-2, can be PDF/A-4f, and here are the three
reasons* — which is why the validator is built first.

## The correction the owner made, an hour after reading it

The limits file's first version said PDF/A-2a was *validate only, never produce*, and the owner
asked the obvious question: if the input already carries the accessibility information, why can we
not convert it? The answer is that we can, and the file had let "we cannot auto-tag" become "we
cannot produce level A" — two different claims about two different documents.

Reading §6.7 against its modal verbs made the correction larger than the question. **Six
requirements in that subclause are `shall`** — §14.8's tagged-PDF requirements, `/MarkInfo
/Marked true`, a `/StructTreeRoot` hierarchy, a role map terminating at standard types, word
boundaries inside show strings, and a valid `/Lang` *where one is present* — and **everything a
reader would call accessibility is a `should`**: alternate descriptions, replacement text,
expansions, non-textual annotation contents, the default language, artefact marking. So **PDF/A-2a
is a tagged-and-Unicode-mapped level, not an accessible one**, and a complete structure tree with
not one `/Alt` in it conforms. `doc/rfc/0006` §5.7 stated the opposite from a secondary source and
now carries the correction inline.

The question also surfaced a limitation nobody had noticed: **PDF/A-2's base document is
ISO 32000-1:2008, which this tree does not carry.** `doc/md/` holds ISO 32000-2, the ledger is
written against it, and §5.1 of part 2 requires adherence to the earlier edition. Where the two
agree nothing is lost; where they differ a PDF/A-2 verdict would be citing the wrong edition. It
is now `doc/pdf-a-conversion-limits.md` §1.2 and an open question, and it is a second argument for
recommending PDF/A-4 as the default target.

## And a second correction, from the same reading

The owner's next question was the same shape as the first and cut deeper: **why refuse a document
whose font is not embedded, when embedding a substitute is exactly what a viewer shows today?**
The answer is that there is no good reason, and RFC 0006 §5.1 had argued the refusal at length
from a secondary source.

A non-embedded font has no appearance of its own — every viewer substitutes, differently, at
display time — so refusing conserves nothing and leaves the document unarchived. **ISO 19005-2
§6.6.6 NOTE 1 and ISO 19005-4 §6.7.5's NOTE name font substitution as an example of a converter
action to record in `xmpMM:History`**: a standard that names the act and says where to write it
down has not forbidden it. And the "inventing marks" objection does not reach the case, because
the content stream shows the glyph either way and the substitution only decides whose outline
draws it.

Reading §6.2.11.5 turned the correction into an engineering finding. **Substituting is not "pick a
face and embed it".** Glyph advances come from the font dictionary rather than the program —
ISO 32000-2 §9.2.4 says why the width is stored in both places — so the page does not reflow, and
this file's first version was wrong to say line breaks could change. But PDF/A then requires the
dictionary and the embedded program to agree to within 1/1000 unit, so the substitute must be
metric-compatible or must have its advances rewritten to match. Rewriting `/Widths` instead is the
one route that *would* move the text, and it is excluded.

What survives is one refusal and one open question. The refusal is narrow and testable with
machinery the tree already has: where §9.10.2's methods cannot name a code — a symbolic TrueType
addressed by glyph index, an `Identity-H` CID font with no `ToUnicode` — a substitute draws the
wrong *character* rather than a different-looking one, and `naming_gap()` already reports exactly
that. The question is which family a substitute may come from, since most installed fonts are not
licensed for unlimited universal embedding: `Q18`'s ICC problem in a second medium.

## And a third: what "refuse" was hiding

The owner's follow-up — *doesn't this mean we sometimes cannot archive something we can view?* —
found that the word **refuse** had been doing three jobs at once, and the file now separates them
in §2.0.

The answer to the question as asked is **yes, and it is the format's point rather than the tool's
limit**: a viewer must put something on the screen and is not obliged to be right, so where the
file does not determine the answer it guesses, and two viewers guess differently. PDF/A is the
claim that the file determines its own appearance, and at Level U or A its own text. A document
that leaves the answer to the viewer is exactly what the format exists to rule out.

**But the honest response to that divergence is to lower the claim, not to reject the file**, and
almost every entry that said *refuse* turned out to mean one of two milder things:

- **No file at all** — only two cases, and in both the viewer fails as well: an encrypted document
  with no password, and an external stream whose bytes the page actually needs. (A stream nothing
  draws is not one: §6.2.2 exempts an unreferenced named resource by name.)
- **Not this part or level — another works.** Implementation limits refuse PDF/A-2 and PDF/A-4
  accepts; unnameable codes refuse -2u and -2a while **-2b accepts**, because §6.2.11.7.1 confines
  the Unicode requirements to Levels A and U; missing word boundaries refuse Level A only; an
  arbitrary attachment refuses -2 and -4f accepts.

So the font case from the previous section resolves the rest of the way: a document whose codes
nothing can name **is archivable as PDF/A-2b** — appearance frozen by §4.9's substitution, text
declared unverifiable — and what is refused is the *level* whose `shall` would have to be
manufactured. §2.2's `.notdef` prohibition splits the same way: a true refusal where the font is
embedded, and no refusal at all where the converter is building the substitute program, since it
chooses the mapping and can preserve an absent glyph as an empty one rather than filling it in.

## A fourth: the file was written for somebody who may choose the target

The owner's archive requires **PDF/A-4, not 4f and not 4e**, and that made the fourth flaw
visible: every entry that answered "and another part accepts it" is useless to a user whose
deposit rule names one target. The file now carries **§9, the same information from the other
side** — per-target tables saying what a source document forces and what the decision then is,
because for a fixed target every "use another part" answer becomes a decision about what to drop.

For a PDF/A-4-only archive the finding is a cheerful one: the losses are narrow and nearly all of
them are attachments and multimedia. **A PDF attachment can stay** — §6.9 permits it if it
conforms to ISO 19005-1, -2 or -4, so converting it recursively keeps it. JavaScript stays (§6.6.2
permits it) and so does `/AA` on widgets. There are no implementation limits, and `ToUnicode` is a
`should`, so the two things that refuse a PDF/A-2 conversion do not arise. What is actually lost is
a non-PDF attachment, a `FileAttachment` annotation, 3D, sound, and the `/Info` dictionary.

## And the resources question, which had a better answer than expected

Asked whether obtaining fonts and ICC profiles under our licence is difficult. Mostly it is not,
and `doc/third-party-data.md` already said so:

- **The standard 14 are shipped.** `data/standard-fonts/` carries Foxit's ten faces under
  BSD-3-Clause and four Liberation Sans faces under the OFL, both permitting embedding, with
  `/NOTICE` carrying the obligations and `viewer-ui/tests/notices.rs` checking them. §4.9's default
  has something to substitute *with*, today, for the commonest case there is.
- **All 239 Adobe `CMap`s are shipped**, BSD-3-Clause.
- **sRGB is one open decision** (`Q18`), with a second route if the licence reading does not hold:
  an ICC v2 matrix/TRC profile is small enough to generate from published colorimetry.
- **CMYK was going to be the hard one, and reading §10.4.2 softened it.** A CMYK output intent is a
  statement about a specific press and belongs to the archive that owns it — but a document with no
  profile is not blocked: ISO 19005-2 §6.2.4.3 NOTE 2 makes a **DeviceN-based `/DefaultCMYK`**
  device independent, and **ISO 32000-2 §10.4.2.5 states the CMYK→RGB transform outright**, so the
  tint transform is the standard's rather than ours. It is non-destructive — the content stream
  still carries the producer's CMYK numbers — and §10.4.2.1 states its own cost in the same breath:
  these algorithms produce "only crude approximations of the original colours". A documented
  default, with the standard supplying both the method and the warning.

## And then the owner said to build it

`crates/pdf-archive` exists: a reader over `pdf-syntax`, `pdf-model` and `pdf-font` that decides
whether a document conforms to a part and level of ISO 19005 and reports which requirement it
fails, where, and which requirements it did not check. ADR 0923 has the four decisions; this is
what the round did.

**One requirement table covering both owned parts, six targets, levels as an applicability
column** — the owner's question, answered by measuring the overlap rather than by taste
(`tools/pdfa-text.py --overlap`). Eleven agents' worth of tranches: file structure, graphics,
fonts, annotations and actions, metadata, and the document-level clauses, each owning one file so
that none could collide.

**The veraPDF corpus arrived mid-round** and turned out to be CC BY 4.0 — a different repository
and a different licence from the GPL/MPL library — so it is a submodule and
`crates/pdf-archive/tests/corpus.rs` compares the two readings clause by clause. Genuine misses
across the six targets fell from 688 to 64. **False positives stayed at zero throughout**, which
is the number that matters, and the six disagreements behind it are adjudicated with their clause
reasoning rather than tolerated.

**The first use of the corpus produced a finding about the corpus**, twice over. §6.2.11.3.1's
`Supplement` comparison is inverted in veraPDF's own validation profile — description and test
alike — against a clause both parts state identically and whose NOTE explains the direction.
And §6.2.5's halftone rule expects a `TransferFunction` for `Red`, `Green` and `Blue`, which
ISO 32000-2 §10.6.5.6 lists among the *primary* components of `DeviceRGB`.

**Three beliefs about the crate's cost were wrong** and `examples/cost.rs` exists because of it;
the third was mine. A full report on ISO 32000-2's own 1023 pages went from 19 s to about 7 s, and
`Examination` — the document, the target, and shared work computed once — is what did it.

**The front door is `quorra-retrieve archive-check`**, JSON on stdout, with `checked`,
`requirements` and `not_checked` beside `conforms`, because a verdict that did not say what it
covered would be the failure `Q20` exists to prevent.

## What the RFC now says

`doc/rfc/0006` §0 carries the answer to its own question 2: parts 2 and 4 bought and readable,
part 1 never, part 3 still preview-only, and — the sentence that matters most — **every statement
the RFC marks second-hand is still second-hand.** What changed is that they can be checked. The
checking is a round's work and was not this one's.

---

## The round did not end there — 2026-09-07 to 2026-09-09

Everything above was written on the round's first day. What follows is the same round, continued,
and it is appended rather than woven in because the record of when something was known is part of
what a history file is for.

ADR: 0924 (four facts `pdf-font` knew and did not say); 0925 (the JP2 boxes nobody had read);
0926 (PDF/A is on, and part 4 is certified first); 0927 (four permissions and the condition they
share); 0928 (a row that stays unchecked on purpose); 0929 (the previous revision is in the file
and not reachable); 0930 (what the file said and what the reader kept).
Questions: `Q46`–`Q52`, and **the owner answered every open question in the directory** — forty-two
of them — on 2026-09-08 and 2026-09-09.

### The answers, and the one that governs the rest

`A46` turns PDF/A on: the hold `A15` and `A17` placed on it is over, RFC 0006 is ratified, the
validator's shape is confirmed as built, and **PDF/A-4 is to be finished and certified first**.
`A20` confirms the not-checked verdict and adds the prohibition this round then leaned on
repeatedly — an `Unchecked` reason may not be weakened to make a row look better, because the
reason is the report. `A51` declines ISO/IEC 15444-2, which closes the JPX baseline row for good
rather than leaving it a debt. `A18`, `A21`, `A48` and `A50` grant a converter its first four
permissions to write something a producer did not, each conditional on the same thing: **what was
written is reported.**

### Three specifications arrived and were prepared by the program

`tools/spec-md.py` generalises the first day's script: it reads any specification in `doc/` with
`quorra-retrieve` and writes into the output which order it got — the structure tree's where a
document has one, the page's content order where it has not. ISO 32000-1:2008 (756 pages) and
ISO/IEC 15444-1:2000 (240 pages) were prepared that way, and preparing the first at one page per
subprocess took 91 seconds until `quorra-retrieve` grew a `text` question that answers a whole
document in one pass: 0.7 seconds.

**Two files that looked like later editions were not**, and that is recorded in three places so
nobody spends an hour on it: `doc/ISO-IEC-15444-1-2016.pdf` and `-2019.pdf` are iTeh preview
documents, fifteen pages of front matter, with `colr` and `EnumCS` appearing exactly zero times.

### The validator finished its first pass

Genuine corpus misses fell from 64 to seven, with `over` at zero on all six targets throughout.
Five of the six rows that came off `Check::Unchecked` in the final push were blocked by a reader
gap in this tree rather than by a standard — and two of those rows' reasons had simply **gone
stale**, still naming a dependency that had since been added and a `pdf-model` gap that had since
been closed. That is the shape `A20` warns about from the other side: a reason that has stopped
being true overstates a debt exactly as a weakened one understates it.

**Two defects were found with almost no corpus witness for their surface.** ISO 32000-2 §7.3.7
makes an entry whose value is null an absent entry, and §7.3.10 puts a dangling reference in the
same place; seventeen rules across three tranches counted the *key*, eight of them prohibitions,
so each was a latent false positive against a conforming file. And lexing a page's content stream
end to end walks into §8.9.7's inline-image binary and invents hexadecimal strings out of it,
which failed five conforming documents before the new rules learned to skip to `EI`.

### A gate that had been red since the crate landed

`tools/state.sh quick` was exiting 101, and this branch had put it there: `main` contains no
occurrence of `ISO 19005-2 §`, and this branch had 483. A `§` in this tree *means* a clause of
ISO 32000-2 — that is the whole of what makes every one of them checkable — so
`ISO 19005-2 §6.1.4` reads correctly to a person while silently resolving in the wrong document.
The tree already wrote `ISO/TS 32001 section 5.1.4`. All 483 are respelled, `Clauses::citation`
with them.

**It stayed red for two days because it was being checked wrongly**, by grepping its output for
the words "fail" and "error" instead of reading its exit status. That is worth writing down: a
gate checked by the shape of its output is not checked.
