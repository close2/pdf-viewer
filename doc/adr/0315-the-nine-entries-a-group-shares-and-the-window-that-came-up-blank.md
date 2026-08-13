# 0315 — The nine entries a group shares, and the window that came up blank

**Status.** Accepted.
**Context.** `doc/todo/01`'s fifteenth sweep — a ledger row that retires its refusal by naming a
capability that arrived, with nobody asking whether the clause's own *entries* were wired to it —
was built in the four-hundred-and-sixtieth session, run once, and left a reading list. This round
re-ran it on the current tree and took the strongest thing outside §14.8 and outside clauses 8
to 11.

## What the sweep said

Population **182 rows of 875**; **24** of them state an entry no file the row itself lists in
`code = [...]` names; **43 entries**, 8 named nowhere in `crates/`, `tools/` or `fuzz/` and 35
named only elsewhere. Most are what the first run found: refusals `CLAUDE.md` already closes by
decision, and rows whose `code` array has simply gone stale beside a crate that grew a second
file. The hit that was work is **§12.5.6.2's `/IRT` and `/RT`**, and the row disposes of them in a
clause of a sentence:

> Nothing else in the clause marks the page by itself: /T, /Subj, /RC, /IRT, /RT, /IT and /ExData
> are a title, a subject, formatted text, a reply relationship, an intent and external data, all
> of which reach a comments pane rather than a raster

That is the sixth refusal shape exactly. It is a *reason*, it is true of most of what it lists,
and it is what stopped anyone reading the paragraph the two entries point at.

## What the clause actually says

Table 172's `/IRT` and `/RT` are not only a relationship. Four paragraphs below the table:

> In PDF 1.6, a set of annotations may be grouped so that they function as a single unit when a
> user interacts with them. The group consists of a primary annotation, which shall not have an
> IRT entry, and one or more subordinate annotations, which shall have an IRT entry that refers to
> the primary annotation and an RT entry whose value is Group .

> Some entries in the primary annotation are treated as "group attributes" that shall apply to the
> group as a whole; the corresponding entries in the subordinate annotations shall be ignored.
> These entries are Contents (or RC and DS ), M , C , T , Popup , CreationDate , Subj , and Open .

Nine attributes, ten keys, and **the row's own justification fails on two of them**: `/C` is ink —
the colour of a synthesised square, polygon, line, ink scribble, text-markup wash or icon
background — and `/Contents` is what §12.5.6.6 lays out on the page for a free text annotation.
The remaining seven decide what a person sees in §12.5.6.14's window: its title, its body, its
date, its colour, whether it opens with the page, and *which window it is*.

Verified against the PDF with `pdftotext -layout` as well as `doc/md/`, because the sweep's
standing warning is that the conversion is the first suspect. Here it is faithful; the only
difference is the spaces `doc/md/` puts around an italicised key name, which is why the quotations
above read `Group .` and `Open .`

## The decision

**`crate::markup::group_source` is the sentence, and it is a function that answers *which
dictionary an entry's value is in*** — not one that replaces the annotation.

That distinction is the whole design. The clause moves nine entries and no others: a subordinate
is still drawn where its own `/Rect` says, in the shape its own `/QuadPoints` describe, under its
own `/F` and its own `/AP`. A reader that swapped the dictionary would draw the group's strike-out
at the caret's insertion point. So the callers read exactly the keys on the list from the result
and everything else from the annotation, and the two sit side by side where a function reads both
— `square_or_circle` takes `/C` from the group and `/IC` from the annotation, in adjacent lines,
because that is what the list means.

Three consequences were decided rather than fallen into:

- **One hop.** The primary "shall not have an IRT entry", so a group is flat and the walk is a
  single resolution. That is also why no cycle is reachable from here: a file whose primary names
  a primary of its own has already left the clause, and following further would invent a hierarchy
  the standard does not describe.
- **`/RT` is checked, not assumed.** Its default is `R`, which is a *reply* and shares nothing. A
  reader that treated every `/IRT` as a group would take the primary's text for 1752 of ISO
  32000-2's replies and show the same comment 1752 times. The fixture pair differs only in `/RT`.
- **A subtype is not a group attribute.** `popup::opens_with_the_page` asks the *parent* what
  subtype it is — Table 175's `/Open` is a text annotation's — and asks the *group* what that
  entry says. Two dictionaries, two questions, and the clause's list is what separates them.

**Table 172's `/Popup` is on the list too, and that is the interaction half.** `popup_of` is
§12.5.1's activation, so a click on any member of a group now exhibits the group's one window —
the primary's — which is what "function as a single unit when a user interacts with them" means
for the one interaction this program has.

## Counted before believed, and the corpus could not rank it

`crates/pdf-model/examples/annotation_group_census.rs`, over the 964 openable documents of the
974: **one** `/IRT` in 34 835 annotations. It is `issue13447.pdf` — a strike-out grouped with the
caret that replaces it — and its primary states the same `/C`, so no corpus page moves and no
corpus popup exists to move. The other three populations state none at all: 273 documents across
`format-corpus`, `pdf20examples`, `pdf-differences` and `pdfbox`, zero.

So the rule is pinned by a **pair of fixtures differing only in `/RT`** (trap 8's fourth shape),
and each half was watched fail with the rule removed — four tests, all four red.

**The witness is ISO 32000-2's own PDF**, which is the second time in twenty sessions that the
standard's own file has been the only document exercising a clause of the standard:

| | |
|---|---|
| annotations | 11 462 |
| stating `/IRT` | 2074 |
| of those, `/RT /Group` | 322 — every one naming a primary on the same page |
| subordinates stating a group attribute | 322, **323 entries disagreeing with the primary** |
| by key | `/Popup` 213, `/RC` 109, `/M` 1 |
| replies (`/RT /R` or absent) | 1752, **every one naming a popup of its own** |

`examples/spec_annotation_census` measures the effect end to end: **1535 of its 2552 windows
carried text before this change and 1748 after.** Exactly the 213. Each of those is an editorial
change to the standard where the caret carries the replacement text and the strike-out — the
annotation a reader clicks — carries an `/RC` whose body is `<p></p>`. A reader of ISO 32000-2 in
this viewer clicked a struck-out passage and got an empty comment.

## Run in the real window, because no gate here sees a host

A popup is drawn by `viewer_ui::chrome::popup_windows` and nothing in §2's sequence looks at a
window. So: `Xvfb`, the real event loop, the real vello surface, and a fixture that is a caret
carrying the words grouped with a strike-out that states its own `/T (nobody)`, its own
`/C [0 1 0]`, no text at all, and an open popup. The window that came up carried **the editor** in
its title bar, **the primary's words** in its body, and a **blue** title bar — and the strike-out on
the page is drawn blue rather than green, which is the `/C` half of the same list showing up as ink.
Four of the nine entries visible in one photograph.

## What is not done, and why it is not a refusal

The other `/RT`:

> Interactive PDF processors shall not display replies to an annotation individually but together
> in the form of threaded comments.

ISO 32000-2 has 1752 replies and this program hands a host 1752 separate windows. Threading them
is a *panel* — the same half round 460 left named for §12.5.6.15's attachments — and it is not
something `pdf-model` can decide alone: a thread is one window per root annotation with an order
and an authorship, drawn by a host that has none of that vocabulary yet. It stays in §12.5.6.2's
ledger row as what keeps it `partial`, quoted, with the count beside it. It is not made a report,
because a report is what this project says about *input it cannot draw*, and this is a shortfall
in what we display.

## Cost

One `Dictionary::get` on the common path — the unresolved `/IRT`, absent from 34 834 of the
corpus's 34 835 annotations — before anything is resolved or cloned. The clone of a primary's
dictionary happens only for an annotation that is a group's subordinate. Every gate ran and none
moved: the corpus gate, the oracle, both text gates, dates, XMP, JPEG 2000 and the quorra corpus
gate print what they printed, which is what a round that changes a window and no raster should
produce.
