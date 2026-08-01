# ADR 0120 — An edit is a log beside the document

Status: accepted, 2026-08-01.

## What this decides

A person can put a value into a form field, undo it and redo it. `Command::Edit(Edit::SetField
{ field, value })`, `Command::Undo`, `Command::Redo`, `Event::Dirty`, `Query::FieldAt` and
`Query::Dirty` — and behind them, in `pdf-model`, a fourth statement about a field's value beside
the three that were already there.

**`pdf_syntax::Document` is untouched, and that is the whole architecture.** `CLAUDE.md` rule 1
makes it immutable; an edit is an entry in a log beside it, and interpretation stays a pure
function of the file's bytes and the viewer's state. Without that, the oracle's comparison of
1665 pages would mean nothing, because "what this program draws for this file" would no longer be
a function of the file.

## The decisions

**A field is named, not pointed at.** §12.7.4.2 makes a field's identity its fully qualified
name and §12.7.4.1 lets one field own several widgets, so typing into one changes all of them —
which is what "a field's value" means. `ViewState::set_field` therefore takes a name and applies
to a set, through the same §12.7.4.2 name table §12.6.4.11's hide action, §12.7.6.3's reset and
§12.7.8's import already walk. Four operations, one idea of what a field is called.
`Query::FieldAt` is how a host turns a click into that name.

**Undo replays; it does not invert.** An inverse would have to remember what each edit replaced,
and for a field with no `/V` that is "no value at all" — a state distinct from every value — and
it would drift the moment two edits touched one field. The log has a cursor; undo moves it and
the surviving prefix is re-applied to a state with none of it. One pass over what a person did in
one sitting, and nothing to keep in step.

**A cleared field is not an untouched field.** `None` shows nothing; never having typed shows
Table 226's `/V`. Two different things, and a `BTreeMap<ObjectId, Option<String>>` says so.

**What a person typed outranks everything.** The file's `/V`, §12.7.6.3's `/DV`, §12.7.8's
imported value and this are four statements about one thing, and the last one made stands. The
three existing ones already worked this way; this joins them.

**Table 227's `ReadOnly` refuses a person and nothing else.** "[A]n interactive PDF processor
shall not allow a user to change the value of the field" — and §12.7.6.3's reset and §12.7.8's
import are the *document* changing its own value, so both still apply. The flag's own sentence
is what draws that line, and the test asserts both halves of it.

## The defect it uncovered, which was six sessions old in the ledger's terms

**A replaced value was not drawn at all where the file stored an appearance stream.**

`regenerates()` — the function that decides whether §12.7.4.3 rewrites a widget's appearance —
asked one question: does Table 224's `/NeedAppearances` say so? For a widget with a stored `/AP`
in a document without that flag, an imported value (§12.7.8, session 100) or a reset (§12.7.6.3,
session 97) changed the field's value and the page went on drawing the *old* one. Silently.

Not caught, because both features' fixtures state no `/AP` at all — and a widget with no
appearance stream takes the other path, where the appearance is *constructed* from the value and
so cannot be stale. The one shape that could fail was the one no fixture had.

The clause settles it. §12.7.2: "[i]f such an object defines an appearance stream, the appearance
shall be consistent with the object's current value as a field." That is an obligation on
whoever wrote the file, and the file kept it — the stream matches the `/V` the file states. It
stops being kept the moment *this program* replaces the value, and at that point the stored
stream shows a value the field no longer has. §12.7.4.3 states the only construction available,
for exactly the two field types whose text is "not known until viewing time"; one line further on
the clause states the strong form of the rule for a `RichText` field: "the entire annotation
appearance shall be regenerated each time the value is changed."

So `regenerates` now asks the flag *only* when the value is the file's own.
`a_replaced_value_is_drawn_even_where_the_file_stores_an_appearance` is the fixture that was
missing, and it fails against the old condition.

**The gates cannot see this**, before or after: every widget in every corpus page has
`FieldValue::Stored`, because nothing clicks. It is the same shape as ADR 0118's upside-down
click — a path only a person reaches.

## A second one, from the same test

**A render in flight was not invalidated by the page changing under it.** The scheduler decided a
request was already outstanding by comparing the page and the target resolution. An edit rebuilds
the display list *at the same page and the same resolution* — so does a layer switch (§8.11), and
so does a pointer moving over a rollover appearance (§12.5.5) — and the scheduler would have
called the outstanding request good and left the old picture on the screen.

`Open::revision` counts interpretations and rides along in `Pending` and `shown`. The test asserts
the shape directly: after an edit, the request that comes back has the same page and the same
target, and a different token.

## Consequences

Tests 919 → 925. Two are `pdf-model`'s — the fixture with a stored appearance, and Table 227's
`ReadOnly` — and two are `pdf-syntax`'s, for the encoder.
`pdf_syntax::text_string::encode_text_string` is new — §7.9.2.2's write side, `PDFDocEncoding`
where the table covers the string and UTF-16BE with the clause's prefix otherwise, checked by a
round trip and by which of the two it chooses. `pdf_model::view::field_at` is new, and
`widgets_by_field_name` became public with it.

The four gates are unmoved.

## What is not here

**Saving.** `ViewState::edits` hands out what was changed, and turning that into §7.5.6's
incremental update — the one form of writing `CLAUDE.md` permits — is the next session's whole
content. Until then an edit lives as long as the program does, which `Event::Dirty` says out loud.

No caret, no keyboard entry into a field, no text editing in the window: the host sends whole
values, and a text editor with a caret is a UI rather than a clause. `viewer-ui` shows the dirty
mark in its title and nothing else.
