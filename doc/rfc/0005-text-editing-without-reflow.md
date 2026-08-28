# RFC 0005 — Basic text editing, without reflow

Status: **draft**
Round: 786, commissioned by the owner
Companions: RFC 0002 (the transform layer — the write path this proposal saves through),
RFC 0003 (file-system faces), RFC 0004 (print). Numbering may be reconciled by the merge
round; references here are by title as well as number for that reason.

**The owner's framing, verbatim in force**: this RFC is not limited by the project's current
rules. Where a standing rule is relevant it is named as a *current restriction with its
rationale*, and the unconstrained design is proposed beside it. The rules bind
implementation until the owner amends them; this document's job is to show the owner what
the unconstrained design looks like so the amendment can be decided by argument.

## 1. Motivation

The owner's words: "we could start without reflowing." That one concession is what makes
text editing tractable at all, and this RFC is the design of the honest version of it.

Users ask viewers for small textual corrections constantly — a typo in a name, a wrong
date, a figure in an invoice — and the market's answer is either a full editor (Acrobat,
whose "Edit PDF" reflows paragraphs and often re-typesets them detectably) or nothing.
RFC 0001's survey found the demand; what no mainstream tool offers is the *modest* middle:
change these glyphs, in this line, in place, and change nothing else — not the layout, not
the neighbouring runs, not the producer's bytes outside the one stream that carries the
edited string. That modest middle is exactly what this tree is best placed to build,
because it already has the three hard parts: a content-stream interpreter that knows every
glyph's position, a font layer that knows what each embedded font can spell, and a save
path that appends §7.5.6 incremental updates without rewriting the producer's file.

What "without reflow" buys, stated as the contract this RFC proposes:

- **No line is ever re-broken.** An edit lives inside one line's box. Text never moves
  between lines, pages never change their break points, and nothing after the edited line
  shifts by a single unit.
- **No re-typesetting.** The font, the size, the rendering mode, the colour and the
  spacing parameters of the edited run stay exactly the producer's. What changes is which
  glyphs are shown and, within the line, where the following glyphs of *the same line*
  sit (an overwrite of equal advance moves nothing at all).
- **No pretence.** Where the document cannot support the edit — the font cannot spell the
  character, the line's box cannot hold the width — the program says so, by name, rather
  than approximating.

## 2. Current restrictions, named

Three standing rules are relevant, and under the owner's directive they are named with
their rationale rather than obeyed silently:

1. **The authoring exclusion** (`CLAUDE.md`, "What *done* means"): we do not create
   documents from nothing, and no generator-side clause is in scope. Its amended form
   already permits what a *user* does to an open document, written back by §7.5.6 —
   "changes shall be appended to the end of the file, leaving its original contents
   intact" — and the tree already saves filled fields, added markups and retyped free-text
   annotations that way (ADRs 0100, 0196, 0304). Editing page text is the same shape one
   step further: the producer's bytes stay in the file, the replacement is appended.
   *This RFC needs no relaxation of the exclusion's rewritten form* — it needs the owner
   to confirm that "what a user does to an open document" covers the page's own content
   stream, which is a bigger step than an annotation but the same kind of step.

2. **`pdf_syntax::Document` stays immutable** (`CLAUDE.md`): an edit is a log beside the
   document, never a change to it, so `interpret` remains a pure function of the bytes,
   the view state and the user's edits. The rationale is the oracle: the whole
   cross-renderer comparison rests on interpretation being a function of the bytes alone.
   **The unconstrained design keeps this rule anyway** — see §5.1. This is a case where
   the current rule and the best design coincide: `ViewState` already carries field
   values, added annotations and retyped free text as a log, and `ViewState::set_free_text`
   (ADR 0304) is precisely the mechanism this RFC extends to page text. A mutable document
   would not make editing easier; it would only make the oracle blind. The RFC therefore
   proposes no change here and records that the choice was re-derived, not inherited.

3. **A document's restrictions are the reader's to set** (`CLAUDE.md` principle 3):
   Table 22's `/P` bit 4 — "Modify the contents of the document by operations other than
   those controlled by bits 6, 9, and 11." — and §12.8.2.2's `/DocMDP` speak directly to
   content editing. The tree already routes such assertions through
   `restriction::asserted` as a policy a host answers (ADR 0604), because a refusal wired
   into the operation could never become the *ask* level. Text editing consults the same
   policy at the same place. No new mechanism; one more operation named in it.

The JavaScript exclusion is not brushed: nothing here executes a script, and editing a
page's text raises no field-recalculation question (that is form *behaviour*, excluded).

## 3. Prior art

- **Acrobat "Edit Text & Images"** — full reflow within a paragraph box it infers.
  Regularly re-typesets visibly (substituted fonts, changed kerning) — the standing
  demonstration that reflow is where fidelity goes to die. Evidence of demand and of the
  wall, not of a design to copy.
- **Foxit / PDF-XChange** — same paragraph-box model, same substitution behaviour, same
  user complaints when the font is a subset (both fall back to a system font and the
  edited run visibly changes face).
- **Okular / Evince / Preview** — no content text editing at all. Annotations only. This
  is the other honest answer, and it is what the market's viewer tier converged on
  because reflow-grade editing is an editor-sized project.
- **The classic wall, everywhere**: subsetted embedded fonts. §9.9.2 requires a subset's
  name to carry a six-letter tag, and different subsets "shall have different tags"; its
  NOTE recommends treating subsets as "completely independent entities". A subset carries
  only the glyphs the producer used; the character the user wants to type has, in
  general, no outline in the file and the full font exists nowhere this program may look.
  Every product that edits text either extends subsets from a system font it happens to
  have (fidelity lost, and this machine may not have it), or substitutes (fidelity lost,
  visibly), or refuses. We refuse, per glyph, out loud — see §5.3.

## 4. What v1 is, precisely

**Editing text runs in place, where the glyph layout allows it.** One line at a time; the
line is the unit the user edits and the box the edit must fit.

Supported operations:

- **Overwrite** a character span with new characters in the same font and size.
- **Extend** — insert characters within or at the end of a line's text.
- **Delete** characters from a line.

In all three, glyphs after the caret *within the same line* shift by the difference in
advance widths; glyphs before it do not move; no other line is touched. A deletion leaves
trailing white space where the line got shorter; that is the design working, not a defect.

Not in v1, stated so the owner can see the boundary rather than discover it:

- No reflow (the premise). No paragraph model at all.
- No font, size, colour or style change — the edited run inherits everything from the
  glyphs it replaces (Tf, Tz, Tc, Tw, Ts, render mode, fill state).
- No editing of text drawn as paths or Type 3 glyph programs used as artwork; no editing
  inside tiling-pattern or annotation appearance streams (annotation text has its own
  route, ADR 0304).
- No editing of text a structure tree marks as an `Artifact`? — open question 5; the
  mechanics are identical, the semantics (page numbers, headers) may deserve a warning.

### 4.1 The boundary of the line's box — the owner chooses

When an insertion makes the line wider than its box, three candidate behaviours:

| behaviour | what the user sees | what it costs |
|---|---|---|
| **refuse** (recommended) | typing stops at the edge; the rejected keystroke flashes the box edge; the status line says why ("the line is full") | a user who wants two more words cannot have them — honest and predictable |
| **clip** | glyphs past the edge are not shown (or are clipped at the box) | silent data loss on screen; the text *is* in the stream but invisible — the worst of the three |
| **overflow** | glyphs run past the box toward the margin / into the neighbour column | preserves the text, ruins the layout the premise promised to preserve; unbounded in tables |

"The line's box" needs a definition, and the honest one is: from the line's first glyph
origin to the earliest of (a) the page's crop edge, (b) the nearest same-baseline glyph
run that is not part of this line (a second column, a table's next cell). The interpreter
already has every glyph's device-space quad (it is what selection and the caret are built
from), so (b) is computable from the readback, not from a paragraph model we do not have.

**Recommendation: refuse**, with the refusal visible at the keystroke. Clipping hides
what the user typed; overflow breaks the only promise this feature makes. Offer overflow
later as an explicit per-edit override if users ask ("let it run into the margin"), never
as the default. The owner decides.

### 4.2 Which documents are editable — detection and refusal

Editability is decided **per glyph to be typed**, not per document, and the answer is
computed from what the file contains:

1. **Embedded font (FontFile/FontFile2/FontFile3), simple or CID.** The new character must
   survive the whole chain *backwards*: Unicode → a character code under the font's
   encoding (§9.6.5's Differences / base encoding, or the CMap's code for the CID whose
   ToUnicode entry matches) → a glyph the embedded program actually defines (a `glyf`
   entry that is not empty-and-unmapped, a CFF charstring) → a width (§9.7.4.3's /W or
   §9.6.2's /Widths). `pdf-font` walks every link of that chain today to *draw*; editing
   walks it in reverse. Any missing link is a named refusal: **"EOODIA+Poetica cannot
   spell 'z': the subset has no glyph for it."** The per-glyph grain matters: overwriting
   "2024" with "2025" needs only a '5', which a subset that ever printed a '5' has, so
   most small numeric and date corrections succeed even in heavily subsetted files.
2. **The standard 14** (non-embedded, base name in §9.6.2.2's list): editable. The file
   already delegates the artwork to the reader's font programs, ours are compiled in
   (ADR 0526), and the widths are the standard's AFMs. The edited run renders exactly as
   the un-edited text around it renders today.
3. **Non-embedded, non-standard-14** (the substitution population, `doc/todo/21`): the
   screen shows our substitute; another reader shows its own. An edit is *typographically
   safe* (widths come from the descriptor we'd reuse) but *visually unverifiable here*.
   Recommendation: allow behind a warning — "this font is not embedded; other programs
   may show this edit differently" — or refuse in v1 and add the warning path later.
   Owner's call (open question 2).
4. **Type 3 fonts**: the glyphs are content streams in the file; a character the font's
   /CharProcs defines is typeable, one it does not is refused like case 1. Rare enough
   to be v1.1 without cost.

The detection runs when the caret enters a run (so the UI can say *up front* "this line
is read-only: FontX is a subset missing most of the alphabet" — shown as a lock glyph in
the edit chrome) and again per keystroke (the per-glyph answer).

### 4.3 The interaction design

Mode entry: an explicit **edit mode** — `viewer_host::keys` gains one entry (say `e`, and
the menu/toolbar equivalent in the native hosts), and within edit mode a click places the
caret in a line. Explicit mode rather than double-click-anywhere because every host
already routes clicks through selection/annotation/field activation, and a mode keeps
"click means select" true everywhere else. (The caret, its placement from a click and its
per-keystroke advance already exist for form fields — ADR 0211 — and are reused.)

    +-----------------------------------------------------------------+
    | pdf-viewer — invoice.pdf                              [edit] e  |
    +-----------------------------------------------------------------+
    |                                                                 |
    |   Invoice date:  .-------------------------.                    |
    |                  | 12 March 2̲025           |   <- line box,     |
    |                  '-------------------------'      caret after 2 |
    |                                                                 |
    |   Amount:        1.240,00 EUR      (other lines: normal draw,   |
    |                                     hover shows a faint box)    |
    |                                                                 |
    |  [!] DejaVuSans-subset cannot spell "@" — keystroke refused     |
    +-----------------------------------------------------------------+

- Hovering in edit mode outlines the line under the cursor faintly; a lock glyph marks
  lines whose font refuses editing (§4.2), with the reason on the status line.
- The active line gets a solid box (the theme's colour in the native hosts, as selection
  already does). Typing, Backspace, Delete, Left/Right, Home/End behave as in a field.
- A refused keystroke flashes the box edge and states the reason in the status line —
  same surface the hosts already use for reports.
- Escape leaves the line (keeping the edit in the log); `e` leaves edit mode; edits ride
  the existing undo/redo stack (`Command::Undo` already crosses the boundary).
- Save is the same save that exists: the file it was opened from, incremental update
  appended. Nothing is written until the user saves, exactly as with fields today.

Boundary vocabulary (sketch, in `viewer-core`'s idiom — final shape is the implementing
round's): `Command::Edit(Edit::SetLineText { page, line, text })` extending the existing
`Edit` family, with the caret/geometry questions answered by the existing readback
queries; a `Query::Editability { page, line }` → per-run answer with the refusal reason,
so every host (GTK, Qt, winit, confined, C ABI) shares one decision — the
`viewer_host::form::Clicked` precedent (ADR 0630). The confined window gets editing for
free: `Command`s already cross the wire, and the worker computes editability inside the
confinement.

## 5. How it is built

### 5.1 The edit log (unchanged architecture, by choice)

`ViewState` gains the text-edit log: per page, per content line, the replacement text
with its anchor (the text-showing operator's identity — stream index and byte range of
the operand, plus the run's graphics/text state snapshot). `interpret` consumes the log
exactly as it consumes retyped free text today: pure function of (bytes, view state,
edits). The oracle keeps its meaning: rendering a document with an empty edit log is
byte-for-byte today's rendering, which is the regression gate for the whole feature.

On screen, an edited line is drawn from the log (the interpreter substitutes the edited
operand during interpretation — same glyph pipeline, same rasterisers, nothing new below
the display list).

### 5.2 The save path — through the transform layer (RFC 0002)

Saving goes through **the transform layer** (RFC 0002's seam; this RFC deliberately names
the seam and not its API — the merge round reconciles). What text editing asks of it:

1. **Replace a content stream.** The edited page's stream is re-encoded as: the decoded
   original bytes, with *only the edited operand's byte range spliced* — one string (or
   TJ array) replaced, every other byte of the decoded stream preserved exactly. The new
   stream object and the page object naming it are appended as §7.5.6's update; where
   /Contents is an array, only the member stream that carries the operand is replaced.
   Byte-splicing (rather than re-serialising the parsed stream) is the fidelity argument:
   there is no round-trip of the producer's operators to get subtly wrong, and the diff
   of decoded streams *is* the edit.
2. **Widths bookkeeping.** Overwrite/extend uses glyphs the font already has (per §4.2),
   so /Widths, /W, FontFile are untouched in the supported cases. Nothing else in the
   file moves.
3. **Font subset extension — priced, not proposed.** If v-next ever wants to type a glyph
   the subset lacks *and the full font is available somewhere licit*, the transform layer
   would need: a font *writer* (sfnt table assembly, CFF charstring merge — a new
   capability class for this tree, weeks not days, plus its own fuzz surface), a new
   FontFile stream, a rewritten /Widths or /W, CIDSet/CharSet updates, and a new subset
   *tag* per §9.9.2 (two subsets differing in glyph complement "shall have different
   tags"). All appendable — the mechanism is still §7.5.6 — but the font writer is the
   cost, and no source for the missing outline exists inside the file by construction.
   v1 refuses instead (§4.2), and the refusal text is where the user learns why.

Restrictions (Table 22 bit 4, `/DocMDP`) are asked once, at the operation, through
`restriction::asserted` — the four-level shape (`off`/`on`/ask/warn) drops in when its UI
arrives, per `CLAUDE.md`.

### 5.3 What can go wrong, named now

- **A run edited twice, and overlapping edits**: the log keys on the operand's identity;
  a second edit of the same operand replaces the first in the log (last-writer-wins
  within a session; undo steps back through both).
- **The same stream object shared by two pages** (rare but legal): replacing the stream
  edits both pages. Detect (the xref knows the object's users… no — object *use* is not
  indexed; the page tree walk is) — v1: detect by walking the other pages' /Contents at
  save, and duplicate the stream for the edited page (new object, edited page points at
  it) so the edit stays on the page the user made it on.
- **Text shown through multiple operators per visual line** (a word per Tj, kerned TJ
  fragments): the line the user sees is many operands. The edit log therefore anchors on
  operands but the *UI* unit is the readback line (which the selection machinery already
  assembles); an edit spanning operands splices several ranges in one save. Mechanical,
  but it is where most of the implementation's care lives.
- **Encrypted documents**: already handled — `incremental_update` re-encrypts per object
  through the document's own key (write.rs does this today for fields).
- **Signed documents**: an incremental update after a signature is exactly what
  `Signature::integrity` reports as "changed after signing" — because it *is* that. The
  edit UI must say so before the first keystroke on a signed document, through the same
  restriction-policy surface (warn level).

## 6. Difficulty

| piece | grade | why |
|---|---|---|
| caret/selection plumbing into content lines | **easy** | readback quads, caret and keys all exist; new wiring, no new machinery |
| per-glyph editability (encoding→glyph→width, reversed) | **moderate** | every table is already parsed in `pdf-font`; the reverse walk and its refusal names are new code with many small cases (Differences, CMaps, ToUnicode gaps) |
| in-place overwrite/extend rendering via the edit log | **moderate** | same shape as retyped free text (ADR 0304) but inside the page's own stream; interpreter substitutes operands during interpretation |
| byte-splice save through the transform layer | **moderate** | splice itself is easy; operand-identity bookkeeping across TJ fragments and /Contents arrays is the careful part |
| line-box computation and the refuse-at-the-edge UX | **moderate** | box from readback geometry; per-keystroke advance check is arithmetic on widths we have |
| shared-stream and multi-operand edge cases | **moderate** | enumerable, testable, no unknowns — just work |
| font subset extension | **hard** | a font *writer* (sfnt/CFF assembly) — a new capability class with its own security surface; and the outline usually does not exist in the file at all. Priced in §5.2.3; explicitly out of v1 |
| reflow | **out** | the premise |

Overall: v1 is a realistic medium-sized arc — nothing in it is research, the hard item is
excluded by design, and every piece lands on machinery this tree already trusts.

## 7. Open questions for the owner

1. **Scope confirmation**: does "what a user does to an open document" extend to the
   page's own content stream? This is the one place the amended authoring exclusion needs
   the owner's word rather than a round's reading.
2. **Non-embedded, non-standard-14 fonts** (§4.2 case 3): allow behind a warning, or
   refuse in v1?
3. **The line-box boundary** (§4.1): refuse (recommended), clip, or overflow — and if
   refuse, is a later explicit per-edit overflow override wanted?
4. **Signed documents**: warn-and-allow (recommended — the reader's document, and
   integrity reporting already tells the truth afterwards) or refuse at `on` level until
   the restriction-levels UI exists?
5. **Artifact text** (page numbers, running heads): editable like any line, or warned?
6. **Undo grain**: per keystroke (field precedent) or per line-visit? Recommend per
   keystroke for consistency with fields.

## 8. Recommendation

Build v1 as specified: edit mode + per-glyph editability with named refusals + in-place
overwrite/extend/delete within the line's box, refuse at the edge, save as a byte-spliced
replacement stream through the transform layer. Keep the edit-as-log architecture — it is
not a constraint on this feature, it is the correct design for it, and it keeps the
oracle's meaning intact. Refuse subset-missing glyphs by name and do not build a font
writer until demand proves it. The result is the only text editor in the viewer market
whose promise — *nothing moves but what you typed* — is kept to the pixel.
