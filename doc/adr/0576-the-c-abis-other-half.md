# ADR 0576 — The C ABI's other half, and the four shapes eleven questions turned out to be

Status: accepted, 2026-08-24. Session 709, the **sixth** round on the project owner's *"even though
low priority, I think we should start investing time into the UI (and its API for the native
versions)"*, taking **item 5** of the ordering ADR 0509 wrote. 683 took item 1, 687 items 2 and 6,
695 item 3, 704 item 4.

Every one of `viewer_core::Query`'s thirty-one variants now reaches a symbol. `PDFV_ABI_VERSION`
does not move.

## 1. What was wrong, and why nothing said so

`tools/state.sh hosts` reported **`Query` reaching 20 of 31**, and named the eleven a C caller could
not ask for. Two of them are the sharpest kind of half-feature:

- a C caller could start Annex O's document-wide search with `pdfv_find_start`, learn from
  `Event::Searched` which page a match is on, and **had no way to draw it** — `Query::Find`'s
  per-page geometry reached no symbol;
- both native hosts obey Table 29's `/PageMode` and `/PageLayout` on open, and a C caller could
  *set* the arrangement with `pdfv_layout` while having no way to ask what the catalogue said.

The reason eleven accumulated is structural and is the thing worth fixing rather than the eleven.
This ABI's protection against `viewer-core` growing is `PDFV_EVENT_KIND_COUNT`: the header states
the number of event kinds a caller was compiled against, the library answers with what it has, and
`pdfv_abi_check` refuses at startup. **That is exactly right for a message that arrives unasked, and
it is no instrument at all for a question.** A `Query` added after the last sweep leaves a C caller
with no symbol and no signal — nothing fails, nothing warns, and the gap is invisible until somebody
counts.

## 2. The deliverable is the instrument, and the eleven come with it

`crates/viewer-ffi/tests/every_query_reaches_the_abi.rs` is what stops the list opening again, and
its shape is the one the rest of this tree already relies on. Every other host is protected by
`viewer-core`'s enums being exhaustive — *"a new `Event` should fail to compile in every consumer"* —
and a C caller cannot fail to compile. So the compiler is made to fail **here** instead:
`entry_points(&Query) -> &'static [&'static str]` matches exhaustively over `Query`, so a variant
added to the boundary breaks this file, and whoever fixes it has to name the symbol that answers it.

Three assertions, each against a different way the list reopens, and **all three were run against
injected defects before being believed** (trap 13):

| assertion | injected | reported |
|---|---|---|
| every variant names an entry point | `Query::Opening => &[]` | *Opening names no entry point* |
| the samples cover the enumeration | a sample deleted | *declares a variant this file has no sample for* |
| every symbol exists both sides | a renamed symbol | *Dirty names pdfv_was_never_written, which the library does not export* |

**The enumeration's size is counted out of `viewer-core`'s own source rather than written down**,
which is `CLAUDE.md`'s rule about derived facts applied to a test. A hand-written count is the thing
that has gone stale in this project at least four times — including in this very crate, where
`PDFV_EVENT_SEARCHED` was missing from the header *and* from the map that checks the header, so the
constant agreed with itself for ninety-seven sessions.

**This is why all eleven had to land in one round.** An instrument with an allow-list is the
mechanism that failed; a test that permits "this variant reaches nothing" is the drift it was built
against, wearing a comment.

## 3. Four shapes, and the rule that produced them

A C entry point cannot change shape once a caller has compiled against it, so the question for each
of the eleven was *what should this look like in C*, not *how do I expose it*. A caller reads a
structure by **index into a handle**, never by following a pointer this library owns, and that
constrains every answer into one of four:

| shape | which answers | why |
|---|---|---|
| a **flat list** | §12.5.6.14's popups | one answer, no page on it — the rule `Query::Fields` follows, because a quadrilateral already in the viewport's own pixels needs no page |
| a **list of lists**, two indices | `Query::Find`, §14.7's structure | an occurrence is several quadrilaterals; a screen is several pages' trees |
| a **tree flattened depth first with a depth per row** | §12.3.5.2's folders | what §12.3.3's outline already takes, because a tree is the one shape a C ABI cannot hand over as itself |
| a **buffer the caller copies out of** | §12.3.4's miniature | the tier-1 copy this project prices everywhere else |

Plus two things that are neither, and each is a decision:

**Table 147 is a *keyed accessor*, not a struct and not nineteen symbols.** `pdfv_preference(viewer,
PDFV_PREF_…, &value)` answers a boolean as 0 or 1, an enumerated name as its own `PDFV_…` number and
a count as itself. The argument is `viewer-ffi`'s own, transposed from a command to a table: a struct
passed by value would put Table 147's *size* in the ABI, so an entry added by a later part of ISO
32000 would change a type every caller has already compiled — the one hazard `PDFV_ABI_VERSION`
exists for, and this header has exactly two instances of it. A symbol apiece would be nineteen
exports for one table. A key is a number, so an entry added later is a new constant beside a function
every compiled caller already links.

**Two of the eleven cost one `pub` between them.** §12.4.3's article threads and §14.3.3's
properties are `viewer_host::article_rows` and `viewer_host::property_rows` already — the two native
hosts wanted them as rows in the seven-hundred-and-fourth session — so they cross as the panel handle
this ABI has had since ADR 0346. That is ADR 0246 decision 3 holding for a third kind of host: *a
native host on this boundary is mostly not toolkit code, and a C host is a native host.*

Four decisions are worth naming because a later round would otherwise re-take them:

- **§12.3.4 gets no list-valued call, deliberately.** There is no `pdfv_thumbnails_read` for a host
  to reach for, because the seven-hundred-and-fourth session found `viewer-ui` building the whole
  list at tab-open while `/PageMode /UseThumbs` opens that tab as the document opens. `pdfv_page_label`
  is a *separate* call for the same reason: a page list needs a name per row and a picture only for
  the rows it is showing.
- **`PDFV_PREF_PRINT_PAGE_RANGE` exists and refuses.** It is the one entry of Table 147 that is a
  list, and it answers `PDFV_WRONG_KIND` rather than being absent — a key that simply did not exist
  would look to a caller like a table this build had not read, and this one says *ask the other
  function*. Trap 5 in the small.
- **`pdfv_collection_folder_of` takes no viewer**, because §12.3.5.2's key grammar is a fact about a
  string. It is the one piece of that clause a caller could not compute for itself: holding a folder
  tree and a file list is no use without the rule that puts one inside the other.
- **A popup with no `/C` answers `PDFV_NO_ANSWER` rather than black**, and a thumbnail's two
  producer-side constraints cross as *bits* rather than as a refusal. Both are the same reading:
  a file breaking §12.3.4's constraints is wrong and its picture is still what the file says.

What is deliberately **not** carried is Table 153's `/Sort`, `/Navigator`, `/Colors` and `/Split`:
each describes how a particular layout would *look*, and this boundary's standing rule is that a look
belongs to the platform — the same rule that keeps a selection's colour out of `pdfv_quads`.

## 4. What it cost, and what it did not

**117 → 169 entry points, and `PDFV_ABI_VERSION` is unchanged.** Not one of the fifty-two takes or
returns a struct by value, which is the whole reason the shapes above are handles, keyed accessors
and out-parameters. A caller compiled against the old header keeps working, loses nothing, and gains
nothing until it recompiles — which is what "an old program does not use a new feature" should cost.

`viewer-core` was not touched. No message was added, no variant changed shape: the eleventh
consecutive round since the six-hundred-and-seventh in which this boundary needed nothing, and the
first in which the *whole vocabulary* is reachable from C.

## 5. Measured with a real C program, outside the tree

Compiled with `-Wall -Wextra -Werror` against `target/libviewer_ffi.so` and `include/pdf_viewer.h` —
ADR 0519's standard — after `doc/todo/02` §5's release build.

**§12.3.4 is the number that matters**, because the briefing for this round asked that a `Thumbnail`
query must not become in another host what it was in `viewer-ui`. On a 233-page corpus document
carrying 231 miniatures (`format-corpus/govdocs1-error-pdfs/error_set_2/210260.pdf`):

| | miniatures | bytes | wall clock |
|---|---|---|---|
| the eight rows a panel is showing | 7 | 210 672 | **0.81 ms** |
| every page, which the ABI offers no call for | 231 | 6 952 176 | **21.6 ms** |

**27 times the work and 33 times the memory**, for rows nobody is looking at — this clause's own NOTE
priced. And 233 page labels cost **8 µs** on the same document, which is what the split between the
two calls buys.

The in-tree C gate (`c/open_a_page.c`, which the test suite compiles and runs) exercises all eleven
on `PDF20_AN001-BPC.pdf` and reports: `/PageMode UseOutlines`, `/PageLayout 1`; 14 of Table 147's 18
entries answered; 14 property rows; page one is called *Cover*; its `/Thumb` is 74×105 and 31 080
bytes with a conformant flag word; 11 occurrences of *the* in 22 shapes; 19 structure nodes on the
page shown with `Document` at the root; no collection and no open popup; and `<7>report.pdf` resolved
to *report.pdf* in folder 7.

## 6. What this round did not do

The structure tree crosses without `AccessibilityNode::lines` — the per-character byte counts and
boxes AT-SPI's `Text` interface wants. An element's own text is its `PDFV_ELEMENT_NAME`, so a caller
building a screen reader has the tree and the extents and not the character offsets. It is two
accessors and no new decision, and it is written into `doc/todo/30` rather than left silent.
