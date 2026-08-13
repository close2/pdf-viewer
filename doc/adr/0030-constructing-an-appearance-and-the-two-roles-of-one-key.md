# ADR 0030 — Constructing an annotation's appearance, and the two roles of one key

Status: accepted, 2026-07-30.

## Context

An annotation with no `/AP` was reported and drawn as nothing. That was the largest gap of any
kind on the demand list once §9.7's composite fonts landed: **63 corpus documents**, 26 of them
carrying a `Widget` and 18 a `Link`, and the corpus gate's annotation row was by one document the
largest row it had.

Table 166 says how unusual that is *supposed* to be:

> A PDF writer shall include an appearance dictionary when writing or updating the PDF file
> except for the two cases listed below. Every annotation (including those whose Subtype value is
> Widget , as used for form fields), except for the two cases listed below, shall have at least
> one appearance dictionary.

The two cases are an annotation whose `/Rect` has no area, and one whose subtype is `Popup`,
`Projection` or `Link`. So a writer that omits an appearance for a `Square` is writing an invalid
file, and 63 documents do it anyway — which is the robustness question rather than the coverage
one, and the corpus is the only instrument for it.

What a processor should then draw is not left to invention. §12.7.4.3 names the operation — "the
PDF processor shall construct an appearance stream dynamically at rendering time" — and each
subtype's clause states what goes in it, out of the entries §12.5.2 calls *appearance
characteristics*: `/C`, `/IC`, `/Border`, `/BS`, `/BE`, and for a widget the `/MK` dictionary of
Table 192.

Reading the whole of §12.5 as a family — thirty-one ledger rows, all `unreviewed` before this
session — is what decided the shape below, and it produced one finding that has nothing to do
with construction.

## Decision

### The construction writes a content stream, because that is what the clause calls it

`pdf-model/src/appearance.rs` builds a **content stream** in the page's own default user space
and hands it back as bytes; `annotation.rs` places it and `content.rs` runs it through the same
path a stored appearance takes. Nothing downstream knows one from the other, which is the same
decision `inline_image.rs` took for §8.9.7 (ADR 0019).

The alternative was to emit display-list commands directly. Writing the stream is better for two
reasons beyond taste. It is *the standard's own model* of the operation, so the code reads like
the clause. And it inherits every rule the interpreter already implements — colour spaces, dash
phases, fill rules, the `/BBox` clip — instead of restating them beside a second painter.

Because the stream is written in default user space, its `/BBox` **is** the annotation's `/Rect`,
so §12.5.5's placement algorithm reduces to the identity and this path needs no second placement
rule. The `/BBox` still clips, which is §8.10.2 doing what §12.5.5 relies on: an appearance is "a
self-contained content stream that shall be rendered inside the annotation rectangle".

### Which subtypes are constructed, and which are refused

The line is not how hard the drawing is. It is whether the clause states a *shape*:

| Constructed | From |
|---|---|
| `Link` | §12.5.4's rounded rectangle, in Table 166's `/C`, at `/Border` or `/BS`'s width |
| `Square`, `Circle` | §12.5.6.8's rectangle or ellipse inscribed in `/Rect` less `/RD`, `/IC` inside, `/C` around |
| `Polygon`, `PolyLine` | §12.5.6.9's `/Vertices` or PDF 2.0 `/Path`, closed for the first, `/IC` filling only the first |
| `Ink` | §12.5.6.13's `/InkList`, one stroked subpath per entry |
| `Line` | §12.5.6.7's `/L`, stroked |
| `Widget` | §12.5.6.19 Table 192's `/BG` background and `/BC` border |

Refused and reported, each because the clause names an appearance without stating it:

- **`Text`, `Stamp`, `FileAttachment`, `Sound`** display an *icon*. §12.5.6.4 requires a processor
  to "provide predefined icon appearances" for seven names and describes none of them. The artwork
  would be ours, not the document's.
- **`Highlight`, `Underline`, `StrikeOut`, `Squiggly`** (§12.5.6.10) state their `/QuadPoints`,
  and the edge the text is oriented against, and *no mark*: not an underline's thickness or
  offset, not where a strikeout crosses, not a squiggle's period, not how a highlight leaves the
  text under it legible. Table 182 does not even admit a `/BS` to take a width from. The corpus
  confirms that the guesses differ — on `annotation-highlight-without-appearance.pdf` the three
  reference renderers draw three different pictures.
- **`FreeText`** (§12.5.6.6) is text, so it needs §12.7.4.3's variable text.
- **`Caret`, `Redact`, `Screen`, `Movie`, `PrinterMark`, `TrapNet`, `Watermark`** state no
  geometry of their own.

Within a constructed subtype the same rule applies to individual entries: Table 179's nine line
endings are named shapes with no dimension between them, Table 169's cloudy `/BE` is "a series of
convex curved line segments" with no curve, and Table 168's beveled and inset borders are
"simulated" three-dimensional effects with no highlight or shadow colour. Each is refused, and a
`Line` whose `/LL` is present is refused **entirely** rather than drawn wrongly, because there
`/L` "represent[s] the endpoints of the leader lines rather than the endpoints of the line
itself".

### A widget that states nothing draws nothing, and that is worth as much as the drawing

The commonest appearance-less annotation in the corpus is an empty text field with no `/MK` at
all: 36 of them in one document, 87 in another. Table 192 is where a widget's background and
border come from, so a widget without one, holding no value, **states no appearance** — and
drawing nothing for it is correct rather than incomplete. Reporting it named 23 corpus documents
for a gap that is not one.

A widget that *does* hold a value draws its frame and reports the value: §12.7.4.3's variable
text is a real gap, and the frame is a real statement in the file. Two true statements rather
than one, which is the pairing `/NeedAppearances` (§12.7.4.3) and `/Matte` (§11.6.5.2) already
use.

### The same keys have two roles, and which one they have depends on `/AP`

This is the finding, and it changed behaviour on 12 documents' worth of annotations — none of
which the corpus can see.

§12.5.2 closes with:

> A PDF reader shall render the appearance dictionary without regard to any other keys and values
> in the annotation dictionary and shall ignore the values of the C, IC, Border, BS, BE, BM, CA,
> ca, H, DA, Q, DS, LE, LL, LLE, and Sy keys.

Table 166 says the same of the two opacities from the other side. Each is defined as the value
used "when regenerating the annotation's appearance stream", and then: "The specified value shall
not be used if the annotation has an appearance stream ... in that case, the appearance stream
shall specify any transparency."

**§12.5.5 states the opposite**, in one sentence about compositing the appearance's transparency
group: "using the values of the BM, ca and CA entries in the annotation dictionary … and a soft
mask of None". This tree followed that sentence, applying `/CA` as the initial alpha of the
appearance's graphics state.

Two statements against one, and the two explain the risk the one creates. `highlight.pdf` writes
`/CA 0.8` on the annotation **and** `/R0 gs` with `ca 0.8` inside its appearance stream: a reader
that honours both applies 0.64 where the producer specified 0.8. That is exactly what the 2020
edition's sentence prevents, and it is why "the appearance stream shall specify any transparency"
is in it. So the decision is:

- **A stored appearance ignores** `/C`, `/IC`, `/Border`, `/BS`, `/BE`, `/BM`, `/CA` and `/ca`.
- **A constructed appearance is built from them**, with `/ca` for nonstroking and `/CA` for
  stroking, and `/CA` standing in for `/ca` where there is none, exactly as Table 166 states.

§12.5.5's group requirement then costs nothing and is satisfied by derivation rather than by
code: an appearance with no `/Group` "shall be treated as a non-isolated, non-knockout
transparency group", and with the annotation's compositing parameters ignored that group has
alpha 1, the Normal blend mode and no soft mask — which §11.6.7's NOTE 1 makes identical to
painting the elements directly, which is what the interpreter does.

## Consequences

**42 documents left the corpus gate's incomplete list**, 189 → **147**, the largest single
movement this project has had; the annotation row fell from 67 to 13. **The oracle's judged set
grew by 46 pages and 36 of them agree with the reference consensus outright.** Two pages that had
been contradicted are now drawn correctly — `annotation-square-circle-without-appearance.pdf` and
`issue20062.pdf` — and every one of the pdf.js corpus's purpose-built `annotation-*-without-appearance`
files agrees with two independent renderers, which is the strongest evidence available that the
geometry above is right.

**Four pages became contradicted, all for drawing a link's border**, and neither silent reference
is reading the clause differently: `mupdf` constructs no appearance for a link at all — its
`pdf_write_appearance` switches over eighteen subtypes and throws for the rest — and
`ghostscript` implements it but renders for a printer, where Table 167's Print flag, clear on
these files, means "never print the annotation". Adding `/F 4` to `file_url_link.pdf` makes `gs`
draw the border, which was checked rather than assumed. `oracle.rs`'s
`CONTRADICTED_LINK_BORDER` carries the whole argument.

**The `/CA` reading changes no pixel in the corpus**, and that was measured rather than assumed:
all twelve documents that carry a `/CA` beside an appearance stream also set their own alpha
inside it, so all 1794 oracle verdicts are identical either way. The only thing in the tree
defending the rule is `annotations.rs::a_stored_appearance_ignores_the_annotations_opacity` —
trap 8 in its most literal form, and the second consecutive session to find a load-bearing rule
the corpus cannot exercise.

**Interpretation costs +0.34%** — 1.9331 G instructions to 1.9398 G by callgrind on
`examples/callgrind_interpret`, with the baseline measured on this machine at the previous commit
rather than taken from a note. The page it interprets carries 15 annotations, so the changed path
runs; the corpus gate is unchanged at 1.6 s.

**One departure is recorded rather than hidden.** §12.7.4.1 says "An interactive PDF processor
shall not limit the range of inheritance for field dictionaries", and the `/Parent` walk that
looks for a field's `/V` is bounded at 32 anyway, because that chain can be a cycle in a hostile
file. Reaching the bound is reported, so the departure cannot be silent — which is the rule the
operand cap taught this project in the fourth session.

**What is left of the demand item is one job, not eleven.** §12.7.4.3's variable text is what
`FreeText`, a widget's value, a check box's tick and a redaction's overlay all need, and it is a
text layout routine this crate has never had. §12.5.6.10's text markup is the only remaining
group whose refusal is about the *standard* being silent rather than about work owed.
