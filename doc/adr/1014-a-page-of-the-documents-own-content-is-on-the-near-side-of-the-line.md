# 1014 — A page of the document's own content is on the near side of the line

Status: accepted. Session 994.
Amends: `CLAUDE.md`'s authoring exclusion, under "What *done* means" — its third amendment, on the
owner's answer `doc/questions/A58` (2026-09-12). Also records RFC 0007's move from `proposed` to
`accepted` on `A54`–`A60`, the seven answers that finished it.
Context: `doc/questions/Q58` and `A58`; RFC 0007 §4.6.1, §4.6.2 and §5; ADR 0816 (the second
amendment, whose form this one follows); ADR 0954 (what an appended page costs, per target); ADR
1005 §6 and `doc/reviews/984-direction-and-boundaries.md` §Q6 (where the line runs in code);
ISO 19005-2 section 6.7; ISO 32000-2 §12.3 and §12.4.2. Nothing under `crates/` changes.

## 1. What was asked, and what the owner said

`Q58` asked whether a `preserve` remedy that appends a page — an embedded image the target may not
attach, a metadata packet its schemas do not define, the statement a signature made to its reader
— needs its own argued amendment to the authoring exclusion, since that exclusion says in as many
words that the next feature across its line "must be its own argued amendment, not scope creep".
It asked one thing more: whether the structure-tree entries a Level A target owes such a page are
a second amendment, "the nearest this program would come to authoring".

The owner's word, whole:

> Q58 agree with recommendation

The recommendation was `Q58`'s: "Treat it as needing the amendment, and make it. The argument is
that the alternative is losing the content outright, and an exclusion written to stop this program
becoming a layout engine was not written to force that choice — but it is the owner's line and the
record says so." The reading that the two amendments are one — the appended pages *and* the Level
A entries — was the round's summary put to the owner beside the question, and that answer ratified
it; `A58`'s `Reading:` section says so on the round's own account, and
`doc/questions/README.md` names the first draft of that file, which put "one amendment, not two" in
the owner's mouth, as the forgery the convention exists to prevent. Nothing in this record is the
owner's word beyond those four; everything else is the argument the owner agreed to, and it is
written here so that the amendment can be read against it.

## 2. What the exclusion was for

The entry's first sentence is the whole of its purpose: "we do not compose pages: no layout
engine, no text-setting, no chart drawing, no "HTML to PDF". No clause whose subject is deciding
what marks a page should contain falls on this project." The thing excluded is a *category of
program* — one that decides what a page should say. Its history is three widenings of what is
*in*, each leaving that sentence where it was: "we do not create files" gave way to "we do not
*create* PDFs" when §7.5.6's incremental update was read as the user's writing rather than
authoring; that gave way to *assembling documents from existing documents* when RFC 0002 §11.1
was ratified (ADR 0816), with the test that keeps the entry enforceable — **does the operation
invent marks?** — written in beside it, and the watermark named as the first feature past it.

The test is what this amendment is judged by, and it is worth being exact about what it asks. A
watermark stamp invents marks: the text or image it lays over every page came from the operator's
command line, from nowhere in the document. The question `Q58` sharpened is different in kind:
an appended image page is "marks **no clause specifies**, even though the image itself is the
document's own".

## 3. Why an appended page of the document's content is not composition in that sense

**The content is fixed by the file, and the program decides nothing about what the page says.**
The image on an appended page is the document's image, byte for byte. The text on a metadata page
is the packet's text. The statement on a signature page is what the file's signature told a
reader — signer, time, whether the bytes still hash — which ADR 1007 already computes from the
source and prints in the report. In every case the *source* of the marks is verifiable against
the input, which is exactly what a watermark's is not. A layout engine is a program whose output
cannot be checked against its input because the input did not contain it; this program's appended
page can be, and the report is where the check is written down.

What this program chooses is *placement*: where on the page, at what scale, in which face where
the content is text. Those are choices where the standard defines nothing, and `CLAUDE.md`'s "one
honest limit" already says what *done* means there — a documented choice, not a match with
anyone. An appended page is therefore the one content stream this program writes that no clause
specifies; the constructions already on the line (§4) each have one. That is a real difference
and the amendment names it rather than blurring it: it is an exception the entry now *has*,
stated, with the watermark left on the far side by the same test.

**The alternative is losing the content outright**, and under some targets it is the only
alternative. ADR 0954 and RFC 0007 §4.6.1 found that what the six targets differ about is what may
be *attached*, not what may be a page: under PDF/A-2 an embedded file's content survives only as
pages, and under PDF/A-4 an operator may reasonably prefer it visible in the document over an
attachment a reader has to go looking for. An exclusion written to stop this program becoming a
layout engine was not written to force that loss — the owner's agreement is to exactly that
sentence — and reading it so would have been the corpus-going-quiet failure in an exclusion's
clothes: a clause held out of scope because it looked like something it is not.

**Why the ruling is one and not two.** ISO 19005-2 section 6.7 requires a Level A file's logical
structure to describe its content; a page this program appended and did not describe would leave
the output non-conforming, and the converter's standing rule (ADR 0947) is that a file leaves the
verb only if it conforms. So the structure entries are not a second act of authoring layered on
the first: they are what makes the first act finish as a conforming file, and refusing them would
not make the output more honest, only wrong. `Q58` put them as "arguably a second" amendment; the
answer is that composing a page *and the entries that describe it* is one thing this program does
to keep content, and the permission covers the whole of it. What an entry may say is bounded the
same way the page is: it describes the content the page carries, and invents nothing about it.

## 4. Where the line now runs, against where the review found it

ADR 1005 §6 proposed the answer this amendment gives — "appending pages is its own amendment,
because `CLAUDE.md`'s exclusion text says so in as many words" — and the review behind it
(`doc/reviews/984-direction-and-boundaries.md` §Q6) tabled where the line actually runs in code.
Its rows, with the one that moves:

| code | the review's finding | now |
|---|---|---|
| `crates/pdf-font/src/restate.rs` — rewrites `hmtx`/`hhea` and CFF widths of an embedded font | *on* the line: touches a producer's font bytes, invents no mark (§9.2.4, §9.9.1) | unchanged |
| `crates/pdf-transform/src/archive/fonts.rs` — embeds a face the file never carried | past the old line, sanctioned by `A47`, reported per font | unchanged |
| `crates/pdf-model/src/variable_text.rs` — a field appearance's content stream | on the line, sanctioned by §12.7.4.3, named in `CLAUDE.md` | unchanged |
| `crates/pdf-transform/src/archive/prepare.rs` — an `/AP` from a subtype's own entries | on the line, sanctioned by `A21` on condition every appearance is reported | unchanged |
| the `F` → `f` respelling | one byte in a content stream, sanctioned by `A50` on a closed list | unchanged |
| appending pages as a `preserve` remedy (RFC 0007 §4.6.1) | **the far side, not built, `Q58` open** | **the near side, not built, `A58` answered** |

Every sanctioned step onto the line carries one condition in common — **what was written is
reported** — and the appended page inherits it: every page appended is named in the report, with
what it carries and where in the source that came from. The review's remaining ask, that the
principle name all five constructions where it names two, is not taken here: this round's slice
is the amendment, and adding a catalogue to the exclusion is a separate edit with its own
argument.

The watermark stays where ADR 0816 put it, and this amendment makes the reason sharper than
"conventional but excluded": its content originates outside the document, so no report could
verify it against the input. That is the line, stated as a property rather than as a list.

## 5. What an appending remedy owes

Nothing is built here; this section is what the converter round that builds it must answer, so
that the permission is not read as cheaper than it is. ADR 0954 priced most of it:

- **Page labels** (§12.4.2) — the page count changes, and a label tree that stops describing the
  document is wrong even where it conforms; the appended pages are labelled, and the labelling is
  a documented choice.
- **The outline** (§12.3) — likewise not left stale.
- **The structure tree**, under any Level A target (ISO 19005-2 section 6.7) — the entries that
  describe the appended pages, bounded by §3's last sentence; or the Level A claim is not made.
- **The report's sentence**: a page was appended, carrying *this*, from *there*, placed *so*. The
  condition every construction on the line shares, and the one thing that makes the page's
  content checkable against the input.
- **`xmpMM:History`**, where the content the page carries was itself derived (`A55`) — the
  archive carries the fact rather than relying on a report nobody kept.
- **The page's composition as a documented choice**: scale and placement for an image; for text,
  a face — which must be embedded under every target, and which `A47`'s permission and condition
  already govern — and the layout of the packet's or the statement's text, verbatim, with no
  sentence added that the source did not contain.
- **Not the identity conversion**: a source that conforms and needs no page appended is still the
  identity (ADR 1006); a page appended is a rewrite and is reported as one.

## 6. What this does to RFC 0007

The RFC is `accepted` as of 2026-09-12 on `A54`–`A60`, and each answer is folded into the section
it decides: §4.4 takes `A54`'s request-and-one-shared-executor shape, §4.5 takes `A56`'s no
confinement in the first version with the warning at the configuration site, §2.1 takes `A55`,
§4.1's `on-failure` takes `A57`'s single alternative, §4.6.2 takes this amendment, §4.7.2 takes
`A59`'s two switches, §4.7.4 takes `A60`. Its §7 keeps the questions and carries the owner's
words under each; its §8 says what is being built this batch and what later converter rounds owe.
The RFC is not redesigned by any of that — it records what was decided, in the owner's words where
the owner gave them and visibly as the round's reading where the owner agreed to a
recommendation.

This ADR clears `A58`'s `Owes:` line, which the questions convention says is the round's to do
once the work is done and the ADR cites the answer. `A54`'s line loses its clause about this RFC's
ratification ADR, which this is, and keeps the executor.

## 7. What is not decided here

How the page is laid out; whether an image is placed as an image XObject on a page of its own
size or on the document's page size; which face carries text and at what size; how a packet
longer than a page is broken. Each is a choice the standard leaves undefined, each belongs to the
converter round that builds the remedy, and each is written down *as a choice* when it is made —
the "one honest limit" of `CLAUDE.md` applied to the one page this program composes.
