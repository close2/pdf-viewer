# 0893 — A `/Font` entry that is §7.3.10's null object, and why the page stays without its text

Session 926. Status: **accepted**, and the half a round should not decide alone is
[`doc/questions/Q27`](../questions/Q27-a-font-the-file-does-not-carry.md).

## Context

Ranking `corpus-cache/tika-issue-tracker/batch5/pdfminer.six`'s 123 first pages by ink
(`doc/todo/03` §48) put two documents at the head whose cause is neither
[ADR 0892](0892-a-type-3-fonts-encoding-may-be-a-name.md)'s nor anything the corpus had already
argued, and they are the same cause seen from opposite sides:

- **`pdfminer.six-90-0.pdf`** is 27 527 bytes and stops in the middle of object 7. It carries
  objects 1 to 7 and no `trailer`, no `startxref`. Its page names eight fonts `F1` … `F8` and five
  image XObjects, every one of them an object number above 7 — so every one of them is, in
  §7.3.10's words, "a reference to an undefined object … treated as a reference to the null
  object". This tree draws the letterhead, the stamp and the rules and reports eight times; the
  page's whole body of text is missing. `pdftoppm` **refuses the file outright** (*Couldn't read
  xref table*) and produces no raster at all. `mutool draw` repairs it, substitutes a font for each
  of the eight, and draws the letter legibly: ink **8.470** against our 2.636.
- **`pdfminer.six-50-0.pdf`** is a 319 kB engineering drawing whose one text font,
  `/T3romansHorzN0`, is likewise an object the file does not define. Here **`mutool draw` agrees
  with this tree** — 1.028 against our 1.145, the drawing without its labels — and it is `pdftoppm`
  that substitutes, at **3.794**.

## What the two references settle, which is not what they look like they settle

Read as a vote this is one reference against another twice over, in opposite directions. Read as
evidence it is sharper than that, and `doc/todo/00`'s step of *looking at the page* is what makes
it so.

**`pdfminer.six-50-0.pdf` is what a substitution costs when the codes are the producer's.** Its
content stream shows `<0001020304050607010308030009010a0b080c00010703>` through a font selected as
`/T3romansHorzN0` — codes 0x00 to 0x0c, which no encoding in Annex D names. Substituting a face and
showing those codes through it is what `pdftoppm` does, and the result on the page is **blocks of
solid black where the labels belong**, on a drawing whose every other mark is a hairline. That is
trap 1's wrong-but-plausible page produced by a reference: the ink triples, the page looks
*fuller*, and what it gained is not text.

**`pdfminer.six-90-0.pdf` is what a substitution buys when they are not.** Its codes are ASCII
through eight fonts a truncation removed, so `mutool draw`'s substitute renders a readable letter
and this tree's page is a letterhead with nothing under it. Here the guess is right, and it is a
guess: the file states no `/BaseFont`, no `/Subtype`, no `/Encoding`, no `/Widths`, no
`/FontDescriptor`, because it states no font dictionary at all.

**The standard’s own substitution route needs the dictionary.** §9.5 puts it plainly — a font
dictionary carries "information that can be used to provide a substitute **when the font program is
not available**" — and every mechanism the standard offers for choosing and placing that substitute
is an entry *of that dictionary*: §9.8.2's flags, §9.8.1's `/MissingWidth`, Table 109's `/Widths`,
§9.6.5's `/Encoding`. A `/Font` entry that resolves to the null object has none of them. So the
question a substitution here has to answer — *which face, showing which characters, at which
advances* — is one the specification supplies no input to, and the two references' disagreement is
what that silence looks like from outside.

## Decision

**Unchanged: a `/Font` entry that is §7.3.10's null object loads no font, the text it would have
shown is not drawn, and the page reports it by name.** This ADR is written because that was never a
decision — it was the absence of one. ADR 0779 gave the condition its own sentence and said
explicitly that it was changing the wording and "never whether there is one"; what the page *does*
had not been argued, and now it has.

Three reasons, in the order they bind:

1. **There is nothing to substitute from**, per §9.5 above. Every other substitution this tree makes
   is chosen from the descriptor the document states (ADR 0153's coverage rule reads `/Encoding` and
   the flags); this one would be chosen from nothing.
2. **The codes are as likely to be the producer's as ASCII**, and `pdfminer.six-50-0.pdf` is the
   witness on this disk that they are. A rule that draws that page's labels as black bars is worse
   than one that leaves them out, because the second is legible as an absence and the first is not.
   This is the same argument `doc/todo/21` §1 already refuses a per-character chain on for
   `freetext_no_appearance.pdf` — a wrong-but-plausible page is worse than the refusal it replaces.
3. **The report is the product.** Trap 5: the shortfall stays loud, and §7.3.10's sentence is in it,
   so a reader is sent to the clause that decides the case rather than to §7.8.3.

## What is not decided here

Whether a *reader* should nevertheless offer the guess — a legible letter with a substituted face
and invented advances, marked as such — is a product question about what this program shows a
person, not a question the specification answers. It is `Q27`, with a recommendation, and the tree's
behaviour meanwhile is the paragraph above.

## What it is worth, measured

Of the **24 324** documents of `doc/pdf.js/test/pdfs`, `corpus-cache/tika-issue-tracker` and the
four `doc/corpora/` submodules, **40 carry at least one `/Font` entry that is §7.3.10's null** and
56 carry §7.8.3's other condition, a name the resource dictionary does not state at all. So this is
not a single file's oddity, and it is also not most of anything: ADR 0779's split is holding, and
the two populations stay counted apart.
