# 1120 — The fence is about where the marks came from, not about who writes

Status: accepted. Session 1107.
Amends: `CLAUDE.md`'s authoring exclusion, under "What *done* means" — its **fourth** amendment,
on the owner's answer `doc/questions/A65` (2026-09-14).
Scope, not construction: this ADR ratifies that clause-driven relocation is *in scope* and names
the class. The on-page construction and its two refusals are ADR 1123, built in session 1113;
ADR 1099's appended page stays the fallback for both refusals and the whole mechanism for content
that was never on a page.
Context: `doc/questions/Q65` and `A65`; ADR 1014 (the third amendment, whose form this one
follows) and ADR 0816 (the second); ADR 1099 §4, which asked the question and built the fallback;
ADR 1025 (the appended page's mechanism); `doc/questions/A64`, which put the redaction's overlay
on the far side by this fence's own reason; ISO 32000-2 §8.4.2, §8.4.4, §12.5.5. Nothing under
`crates/` changes with this ADR.

## 1. What was asked, and what the owner said

ADR 1099 §4 declined the smaller-looking answer on purpose. An annotation ISO 19005 does not admit
has to go; its normal appearance is a form `XObject` the producer wrote; and the smallest way to
keep those marks is to write `q AA cm /X Do Q` onto the page they came off. That was not taken,
because the third amendment permits *appending a page* and in the same paragraph keeps the far
side closed — "the watermark stays on the far side: it composes new content *over* pages, and
nothing here reaches it" — and writing operators into a page a producer wrote is that operation,
whoever the marks belong to. A round may not widen a ratified amendment by argument alone, so it
went to the owner as `Q65` and the appended page, already inside the permission, is what session
1085 built.

The owner's word, whole:

> Rule provenance.

Everything below is the argument that answer ratifies, written here so that the amendment can be
read against it. `A65`'s `Reading:` section is the round's transcription and keeps the owner's two
words visibly apart from it.

## 2. The sentence was about the marks, and it had been read as being about the writing

The fence sentence has two halves and only one of them is a test. "It composes new content over
pages" — *composes new content* is the test, and *over pages* is the watermark's own geometry. The
third amendment's test is stated in the entry itself and is one question: **does the operation
invent marks?** A watermark invents them; the text or image it lays over every page came from an
operator's command line and from nowhere in the document, and no reader can check the output
against the input because the input never held it.

Reading the sentence as a rule about *writing onto a producer's page* makes the location of the
bytes the test instead of their provenance. That reading has a consequence nobody argued for: it
forbids moving a producer's own marks by one construction and permits the same marks, at the same
coordinates, by another that changes the page count. The appended page is the *larger* act of the
two — it composes a page that was not in the document, and ADR 1025 §4 had to argue ten placement
choices for its text — where the on-page placement composes nothing and chooses nothing.

## 3. The class this opens, and the two-part test that bounds it

**Clause-driven relocation of producer-written appearance streams.** Two conditions, and a site is
in the class only with both:

- **a clause requires the content to move** — here ISO 19005-2 section 6.3.1 and ISO 19005-4
  section 6.3.1 forbid the annotation's subtype and offer nothing to put in its place, so the
  annotation goes and its marks have nowhere to be unless they are moved;
- **and that clause fixes where it goes.** §12.5.5's algorithm computes the matrix `AA` from the
  annotation's own `/Rect`, `/BBox` and `/Matrix` — "A matrix A shall be computed that scales and
  translates the transformed appearance box to align with the edges of the annotation's rectangle
  (specified by the Rect entry)" — so the operators added *are* that placement. Not one number in
  them is this program's.

The class will take in later sites of the same shape, and a flattened form field is the one to
expect: §12.7.4.3 already sanctions the appearance, §12.5.5 already fixes where it sits, and
flattening moves it into the content of the page it is already drawn on. Each such site is
admitted by the two-part test, not by resemblance to this one.

**It stops where the marks become this program's invention.** The watermark stays on the far side:
no clause requires it and no clause says where it goes. So do the redaction's overlay and its fill
— §12.5.6.23's `/RO`, `/OverlayText` and `/IC` — which `A64` had already put there, naming this
fence as the reason: an overlay text a processor sets from `/DA` and `/Q` is partly this program's
drawing, and the clause's own wording makes the removal a redaction without it.

## 4. The two honest costs, which the construction owes rather than has

`Q65` put them in the question because they are what a yes costs, and `A65` answered that each is
to become a refusal with a sentence rather than a difference a reader would find on the page. They
are stated here so ADR 1123 builds them against a written reading; ADR 1123 is where each became a
refusal with a sentence.

**A producer that pops further than it pushes.** §8.4.2 states the rule in one sentence and states
it for exactly the population an on-page construction writes into: "Occurrences of the q and Q
operators shall be balanced within a given content stream (or within the sequence of streams
specified in a page dictionary's Contents array)." Real files break it. A construction that
prepends a `q` and closes it after the producer's operators recovers the initial graphics state
however much the content left *open*; an unmatched `Q` pops that prepended state instead, and every
mark after it draws under a state the conversion introduced. That page is to be refused by name.

**A remaining annotation over the moved marks.** §12.5.5 composites an appearance "with a backdrop
consisting of the page content along with any previously painted annotations", so marks moved into
the content go under every annotation that stays. Where the rectangles do not meet, nothing a
reader sees changes; where they do, the program has decided what a page looks like. It is
computable from the rectangles, because §12.5.5 fits the appearance to the annotation's own
`/Rect` — the rectangle *is* where the moved marks land. **The population is every remaining
annotation whose rectangle meets the moved one**, not those earlier in `/Annots`: the standard
states no painting order for annotations at all — "previously painted annotations" is its only
word on the subject and it names no order — so which of two annotations is over the other is not a
fact this converter can read out of the file, and §12.5.3's Hidden flag is the one exclusion that
is derivable.

**ADR 1099's appended page is the fallback for both.** ADR 1123 built the on-page construction, so
a preserved appearance goes onto an appended page only where one of the two refusals above sends
it. The appended page keeps its own permission whole: it is still the only answer for content that
was never on a page at all,
an XMP packet being the standing case. The report will say which of the two constructions each
preserved thing got, which is ADR 1014 §5's fourth bullet — *a page was appended, carrying this,
from there, placed so* — asked of a placement that appended nothing.

## 5. What this does not permit

It is a permission to *move* marks, not to draw them. Nothing here lets this program construct an
appearance and then relocate it: an appearance `pdf_model::appearance` built is this program's
picture, `doc/questions/A48` forbids keeping one and calling it the producer's, and ADR 1099 §5
already refuses that case by name. Nor will it touch the producer's bytes — §7.8.2 makes a
`/Contents` array one stream, so what a construction writes is appended to the array the producer
wrote, never spliced into a stream of his.
