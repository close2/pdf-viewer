# 0613 — A window two hosts drew nothing of, a cursor two hosts did not change, and a place three accessors reach

Status: accepted (session 726, the ninth UI round)
Supersedes nothing. Extends ADRs 0191, 0576, 0603, 0604.

## What this round was handed

ADR 0603 gave `tools/state.sh windows` a **reading** — one line per unreached variant, marked *debt*
or *not a debt*, checked in both directions — and left five debts standing. This round took three of
them, and the choice is ADR 0509's criterion in order: *what a reader can do with a document and
cannot do here*, then *what costs no new message*, then *what makes the level-hosts decision
checkable*.

- **§12.5.6.14's popup windows.** A comment on a page was invisible in two windows of three. The
  corpus has witnesses. First on the criterion.
- **§12.5.6.5's link cursor.** A reader could not see that a link was there until after clicking it,
  in two windows of three. Cheap, and the same shape one clause over.
- **`AccessibilityNode::lines` across the C ABI.** Named by ADR 0576 and priced by `doc/todo/30` at
  *"[t]wo accessors and no new decision"* — a claim about cost that nobody had checked.

The two left standing are §12.3.5's collection and §14.7's tree on the native hosts' own
accessibility interfaces, the second with §9.10.2's readback beside it. Both are named in
`tools/state.sh windows` with their reasons; the second is `doc/todo/31`'s and is the largest thing
on that list.

**No message was added**, which is the thirteenth time since the six-hundred-and-seventh: everything
these three needed was already on the boundary. `Query::Popups` has existed since ADR 0191,
`Query::LinkAt` since the vocabulary was frozen, and `Answer::Accessibility` has carried the lines
since ADR 0394. What was missing was consumers.

## 1. The popup window, and what belongs in `viewer-host`

§12.5.6.14 is the one annotation subtype whose picture is not the page's: *"It shall have no
appearance stream or associated actions of its own."* So `pdf-model` reads it, `viewer-core` places
it in device pixels, and a **host** draws the window — which is why the answer is a rectangle and
three strings rather than pixels, and why two hosts drawing nothing of it was a debt rather than a
tier difference.

**`viewer_host::popup` is the deliverable**, not the two toolkits' widget code. What it decides:

- the title bar's two texts — §12.5.6.2's `/T` and Table 166's `/M` through `viewer_host::stamp`;
- Table 166's `/Contents`;
- the upright box, from `viewer_host::bounds` of the quadrilateral;
- and the one refusal: **a window with no area is not placed.** Table 166 makes `/Rect` required and
  the clause gives the popup no appearance stream to fall back on, so a rectangle whose corners
  coincide describes a window a person could not see and a widget a toolkit would put in its layout
  anyway.

`viewer-ui` adopted it and lost its own derivations doing so, which is this crate's own test applied
to itself: a third copy of a reading is where two hosts stop agreeing. It also gained a correction it
had not asked for — it took the box from `quad[0]`, `quad[2]` and `quad[5]` directly, which is the
axis-aligned bound only for a page §7.7.3.3 does not turn.

**`viewer_host::bounds` came with it and was overdue.** `viewer-gtk` and `viewer-qt` each carried a
private `bounds` with the same body and nearly the same doc comment; the popup would have been its
third and fourth call site.

**What is *not* shared is the look**, and the split is where this crate's "no widget and no pixel
format" line already falls: a `GtkFrame` with a `GtkOverlay` under its title bar (GTK4 has no
per-widget background short of a style sheet, and a document's `/C` reaching into this program's CSS
is not a thing to build), a `QFrame` with a palette on the bar, and `viewer_ui::chrome::draw_popup`'s
own paper. Each wraps the note with its own text engine, which is most of what makes placing a real
widget worth doing.

**Neither window takes a press.** The clause gives a popup *"no … associated actions of its own"*, so
`set_can_target(false)` and `Qt::WA_TransparentForMouseEvents` are the clause rather than a
convenience: a widget over the page that swallowed a click would take the selection, the link and the
form control underneath it away from the reader.

### The defect the screen found, which is trap 19

A `GtkFixed` measures the union of its children. `issue14438.pdf` states six open windows *beside*
its page — `/Rect` x 598 to 785 on a 612-unit page — so placing them in the layer the page is in made
the document decide how wide the window was: `--trace` shows the page area walking 509 → 653 → 838 →
1035 → 1134 → … → 1229 device pixels in nine frames, a geometric series that converged only because
each step halved. Nothing looked wrong; the windows appeared and the page drew.

The popups are a `GtkOverlay` child now — not measured unless `set_measure_overlay` says so, and
allocated the overlay's own size, so a window outside the viewport is clipped instead of moving it.
Qt's `PageArea` has no layout at all and was right by accident; it is now right on purpose, with the
reason written beside it.

**And the first version of the report would not have caught it.** The trace line fired on the count
of windows *placed*, so a document whose windows all had zero area printed the same silence as a
document with none. It fires on what the answer held — trap 11 pointed at the one line that would
otherwise say nothing about a refusal.

### What the screen showed

Driven under `Xvfb` on `pr7352.pdf` and photographed in all three: Table 166's `/C` red title bar,
§12.5.6.2's `/T` *Popup annotation* at the left, `2016-05-25 12:40` at the right — one format, from
`viewer_host::stamp`, in all three — and `/Contents` wrapped underneath. `issue14438.pdf` reports
`6 of 6 §12.5.6.14 popup window(s) placed` in both native hosts.

## 2. The link cursor, and a convention said out loud

§12.5.6.5 states **no cursor at all**. It describes what a link *is* and what activating one does,
and Table 176's `/H` is about the *highlight on a press* rather than about a hover. So what a pointer
looks like over the activation region is a convention this program chose in ADR 0166 and kept in one
host of three — which is exactly the asymmetry `doc/todo/30`'s level-hosts decision exists to
remove, and which is recorded here as a choice rather than as a requirement met.

`Query::LinkAt` is asked on every pointer move, which is what makes it a query rather than a command,
and each host sets the cursor only when the answer **changes**: `gdk_surface_set_cursor` and
`QWidget::setCursor` reach the display server, and a pointer sweeping a page of links would reach it
on every motion event. GTK sets it on the layer the pages are in rather than on the toplevel, because
GTK takes the cursor from the widget the pointer is *picked* on — the chrome layer is
`set_can_target(false)` and is therefore never picked, and the toplevel would override the text
cursor a `GtkEntry` over a §12.7 widget sets for itself. Qt cannot be called from Rust at all
(`crate::bridge`), so it is a `QtUpdate` flag with `over_link` beside it — the shape ADR 0470 and
ADR 0519 already use.

**The cursor cannot be photographed on this machine.** `xwd` does not capture the pointer and there
is no compositor, so what was driven is the *loop*: `viewer_host::trace::Topic::Pointer` is a new
topic, and sweeping the pointer down page 5 of ISO 32000-2 makes both native hosts report crossing
into and out of the activation region. Saying that plainly is better than a code check dressed as a
measurement.

## 3. `AccessibilityNode::lines`, and what "two accessors" cost

`doc/todo/30` priced this at *"[t]wo accessors and no new decision"*. **It is three**, and the reason
is the ABI's own convention rather than an oversight: a count is asked before an indexed accessor,
and a line has two counts — how many lines an element drew, and how many character codes a line
holds. `pdfv_structure_lines`, `pdfv_structure_line` (text *and* code count in one call) and
`pdfv_structure_character` (byte count and box).

The text and the code count come back together deliberately: the invariant a text interface rests on
is that the character byte counts sum to the line's length, so an offset into the string and an index
into the characters convert into each other without either side guessing. A caller that had to ask
twice could observe a state where they disagreed.

**They are not §14.9's substitutions**, and that is the decision worth recording rather than the
symbols. `PDFV_ELEMENT_NAME` applies `/Alt` and `/E`; these do not, because a caret moves over what is
on the page and `GetCharacterExtents` asks where the *glyph* is — a phrase substituted for an
element's content has no glyphs. So an element stating one has zero lines here, which is the same
thing `pdfv_structure_node`'s `substituted` already says from the other side.

`PDFV_ABI_VERSION` did **not** move: not one of the three takes or returns a struct by value, which
is the only kind of change that constant exists to catch.

## 4. Two instruments corrected, both of them by their own checks

**`tools/state.sh windows` printed `SPENT` for `LinkAt` and `Popups`** the moment the hosts reached
them, which is what ADR 0603 built the second direction for. Both reasons are deleted.

**And the `Popups` reason was wrong about its own population.** It said *"[s]even of the corpus's
documents state an open one"*; the measurement is **seven open popups on two documents**
(`issue14438.pdf` with six, `pr7352.pdf` with one), which is what `pdf-model`'s `popup` module and
`viewer-core`'s own headless test both say. The row is gone with the debt, so the durable fix is in
the instrument: `examples/open_annotation_census` counted popups stating Table 186's `/Open true`
into its totals and **named no document holding one**, so a round wanting to look at an open window
had a number and no file. It names them now.

## 5. Two documentation defects that cost this round its time

**`doc/todo/02` §5 told a round to install from a literal `/home/AI/cargo-target/pdf-viewer/`.** That
is the *main* tree's build directory. A worktree round has its own, so the install copies a
**neighbour's** binary over this round's — and this round rebuilt the GTK host three times,
installed it three times, ran a feature that was working and saw nothing, three times. It is trap
15's own subject reached through an instruction; §5 derives the directory now, as `tools/round.sh`
always did, and `tools/state.sh disk` does too.

The tell was the one trap 15 already names: *nothing moves when you re-run after an edit.*

## Consequences

- All three windows draw §12.5.6.14's popup and change the cursor over §12.5.6.5's region. Two of
  ADR 0603's five debts are spent; the reading's `SPENT` check said so rather than a round asserting
  it.
- `viewer-host` gains `popup` and `geometry`; `viewer-ui` loses three private derivations and gains
  a rotation correction.
- The C ABI reaches every part of `Answer::Accessibility` a screen reader needs. 169 → 172 entry
  points, `PDFV_ABI_VERSION` unchanged.
- Trap 19 is new: a widget the *document* placed can decide how big the window is. Trap 15 gains the
  instruction that reproduced it.
