# ADR 0250 — A search that reads one page at a time, and the bar three hosts drew

Status: accepted, 2026-08-09 (session 414).

## Context

Session 413's fifth sweep, run over `viewer-core`'s vocabulary rather than over `pdf-model`'s
functions, found **`Query::Find` named by no program at all**: a text search implemented since the
hundred-and-fortieth session, tested, carried across the confined transport — and with nothing a
person could press, in a tree with five programs and a C ABI. `doc/todo/01` recorded it as owed
rather than fixing it, on the rule that a sweep does not ship features.

Beside it sat a clause. Annex O's `search` was one of four fragment parameters `Parameter::unhonoured`
reported by name, and its reason was the only one of the four that named a *capability*: "this
program has no document-wide search". ISO 32000-2 §O.2.2, Table Annex O.4:

> Open the document and search for one or more words, selecting the first matching word in the
> document.

So the feature and the clause were the same piece of work, and neither could be done without
deciding what `Query::Find`'s *scope* is.

## What the difference costs, measured

`Query::Find` answers for the page being shown, out of a readback `Interpretation::text` already
holds. Annex O wants the document. The gap between them is not a matter of looping: reading a page
that is not on the screen means **interpreting** it.

Measured on ISO 32000-2's own file, 1023 pages, `--profile gates`:

| | |
|---|---|
| interpreting every page, sequentially | **5.84 s** |
| readback of all 1023 pages | 2.66 MB |
| per page, mean | **5.7 ms** |
| first page holding "transparency group" | page 6, **20 ms** in |

That number decides the whole shape. `Viewer::query` takes `&self`, is asked at pointer speed and
caches nothing, so a document-wide search cannot be a `Query`. And `doc/ui-boundary.md`'s rule 4 —
"no threads the core was not handed, and nothing blocking" — with rule 3's "no clock" says the core
may neither block for 5.84 s nor spend a time budget of its own.

## Decision

### `Query::Find` stays the page; `Command::Find` is the document

Two questions, not one with an approximation of the other. A find bar wants both at once: every
match on the screen, from a readback that exists, on every repaint; and *the next one anywhere*,
which interprets pages. Folding them would have made the cheap one cost the expensive one's price.

### A step is one page, and the host decides how many steps a frame gets

`Command::Find(Find::Start { needle, direction })` makes a plan; `Find::Continue` reads **one page**;
`Find::Stop` throws the plan away. `Event::Searched { found, remaining, wrapped }` reports each
step. A host pumps until `remaining` is zero, which is exactly what it already does with
`Event::NeedsRender`: this crate hands out work rather than doing it on somebody's event loop.

One page rather than a budget of *n* pages, because a budget would be a number chosen inside a crate
that cannot see a frame deadline. The three hosts each spend it differently — winit in
`about_to_wait`, GTK through `glib::idle_add_local_once`, Qt through `QTimer::singleShot(0, …)` —
and all three keep painting while a thousand-page document is read.

**Nothing is cached.** A `next` that crosses three pages interprets three pages, and a second search
over the same ground interprets them again. The alternative is a readback cache with a byte budget
and an invalidation rule for every view-state change that alters what a page draws — a layer
switched, a value typed, an appearance following the pointer — and at 5.7 ms a page nobody has
measured a need for it. Written down as a deliberate cost rather than taken silently.

### Annex O's `search` is started when the document opens and finished by the host

`apply_fragment` builds the plan and says so in a note; `Command::Open` answers with
`Event::Searched { found: None, remaining: n }`, which is the signal there is something to pump. It
is **not** run before `Command::Open` returns, and that is `CLAUDE.md`'s startup rule rather than a
convenience: up to 5.84 s of interpretation on the launch path would be the largest single
regression in time-to-first-page this tree could make.

The annex's own words decide two details. "[O]ne or more words" is a *list*, so any needle matching
is a match and the earliest wins; and "the first matching word **in the document**" means the plan
starts at page one and does **not** wrap, where a find bar's does. "[S]electing" is carried out as
the selection — the range `Query::Selection` answers with — because that is the one thing in this
vocabulary that means what the verb means and is what every host already draws a highlight from.

### Three decisions the standard does not make, recorded as choices

The standard describes no find bar and no matching rule. Each of these is ours:

- **Case is folded**, by `char::to_lowercase`, and nothing further. Accent folding, ligature
  equivalence and the collation algorithm's tailorings are decisions about a *language*, and the
  readback is what §9.10.2's three methods produced rather than normalised text. (This one predates
  this session; it is restated because the next two sit on it.)
- **A space in the needle matches whatever separated the words on the page.** This one has a
  derivation rather than a convention behind it. Neither space nor line break is in the file:
  `content.rs`'s `separate_text` *infers* both from where §9.4.4's text rendering matrix put the
  next glyph — "[a] content stream has no notion of words or lines; it has positions" — so matching
  a phrase against a literal `' '` would be matching it against this crate's own line-breaking, and
  "transparency group" broken across a line would go unfound on a page that plainly says it.
- **Whole-word matching is not done**, and the reason is that same sentence from the other side.
  `tests/text_extraction.rs` compares words with all whitespace removed because "[w]ord boundaries
  are deliberately not compared, because a content stream does not record them". Requiring a
  boundary would be requiring one this tree reconstructs by heuristic. So "the" finds the one inside
  "theme" — which is also what every find bar does.

### A match highlight is a selection highlight's neighbour, and crosses the same way

`doc/ui-boundary.md`'s rule is that geometry crosses and the host draws it in its own colours. A
match is exactly that kind of thing, so **no new answer was added**: `Query::Find` already answers
in the same shapes `Query::Selection` does, and the current occurrence is the selection. A host
draws the matches under the selection and the platform decides both colours.

What that produced is the argument's second demonstration after ADR 0246's, and it is sharper for
being about one feature drawn three ways:

| host | the bar | the matches | the current one |
|---|---|---|---|
| `viewer-ui` | drawn by us, `pdf-font`'s Helvetica in a `pdf-render` list | yellow, `Multiply` | the selection's blue |
| `viewer-gtk` | a real `GtkSearchBar` + `GtkSearchEntry` | `gtk_widget_get_color` at 0.12 | the same colour at 0.25 |
| `viewer-qt` | a real `QToolBar` + `QLineEdit` + two `QAction`s | `QPalette::Highlight` at 0.12 | the same at 0.35 |

The two native hosts made the *same* choice — one hue at two alphas — and this host made a different
one, two hues, and the reason is a fact about the platforms rather than about taste: neither GTK nor
Qt hands out a second highlight colour, and a host with no theme to ask may pick both.

## What it cost the consumers

Six of them failed to compile, which is what nothing being `#[non_exhaustive]` is for:
`viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-confined`'s transport, `viewer-ffi`'s two exhaustive
matches, and `viewer-core`'s own headless harness. The C ABI is the one that cannot fail to compile,
and it paid the price this boundary is designed around: **`PDFV_EVENT_KIND_COUNT` moved from 15 to
16**, so an old caller's `pdfv_abi_check` refuses at startup naming the number that moved, rather
than meeting a message it has no arm for. Four entry points came with it — `pdfv_find_start`,
`pdfv_find_continue`, `pdfv_find_stop`, `pdfv_event_searched` — taking the ABI from 39 to 43, and
`c/open_a_page.c` drives the search itself: three steps, an occurrence on page 3 of the application
note, in a loop with a bound because a caller that trusted `remaining` to reach zero would hang on a
library with a bug.

Session 412 demonstrated that adding a `Command` costs a compiled C caller nothing. This is the
other half of the same demonstration, and it is why the count is a number rather than a promise.

## Seeing it

Under `Xvfb` at 1100×1200 with lavapipe, on ISO 32000-2 (1023 pages), `pdf-viewer`:

- `/` opens the bar; "transparency group" typed into it draws in Helvetica with a caret after it.
- Enter → **page 6**, the occurrence highlighted: **818** tinted pixels in a box 87 × 11 at
  (363, 656)–(449, 666), which is "Transparency groups" in the table of contents.
- Enter → **page 8** (803 px at (333, 321)–(417, 331)); Enter → **page 120**; shift-Enter → back to
  **page 8**.
- With the selection on page 8, shortening the needle to "transparency" highlights **two**
  occurrences at once — 1497 tinted pixels in two bands, rows 321–332 and 336–347 — the current one
  under the selection and the other in yellow alone.
- A word the standard does not contain reads all 1024 steps and answers "not in this document".

`pdf-viewer-gtk` and `pdf-viewer-qt`, on the black-point-compensation note: both find
"compensation" on page 1 and say so in the status bar. The pixels show the two layers stacking:
Qt's highlight is `srgb(171, 220, 246)` where the selection alone was `srgb(187, 227, 248)`, which
is `#3daee9` at 0.35 plus a 0.12 match under it; GTK's is `srgb(184, 186, 186)` for the same reason
in the theme's foreground.

## One number that came out of running it

The first full-document miss took **19.25 s** in the window against 5.84 s of interpretation, and
the difference was not the search. `viewer-ui` repainted on every `Event::Searched` — 1024 whole
windows presented through lavapipe to move a progress count by one digit. Repainting once every
sixteen steps, plus the step that answers, is **6.19 s** (median of three: 6.29, 6.19, 6.10) with a
count a person still sees moving. The constant is `SEARCH_REDRAW`, it belongs to this host alone,
and the measurement is beside it in the comment `CLAUDE.md` requires.

**The lesson is the one the tension between principles 2 and 4 keeps producing**: the expensive part
of a feature is often not the part with the clause number on it, and only running the program says
which.

## What is left open

- **`highlight` and `fdf` and `ef` still report by name**, and their reasons are unchanged — a
  missing *concept*, a fetch principle 3 keeps out of the renderer, and a policy `doc/todo/38` owns.
  `doc/todo/39` now names three rather than four.
- **A match count.** "3 of 17" needs the whole document read before the first answer, which is the
  5.84 s this ADR is about; a host that wants it can pump to the end and count, and no message here
  prevents that. Not built, because no host has asked.
- **A cache**, priced above and deliberately absent.
