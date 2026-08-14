# ADR 0346 — The vocabulary a C caller can reach, and the flag that was obeyed backwards

Status: accepted, 2026-08-14 (session 511).

## Context

`doc/todo/30` has three items left and they are *surface* rather than architecture: the C ABI's
entry points are not the whole of `viewer-core`'s vocabulary, Table 229 bit 26's `RadiosInUnison`
"crosses and is not obeyed", and ADR 0245's third decision — the scale a native form host draws the
page at — is unestablished. This round takes all three. Two of them turned out to be smaller than
the file thought and one of them turned out to be a *misreading*, which is the part worth the ADR.

---

## Part one — the ABI's remaining sixty-eight symbols

**43 entry points became 111**, and `doc/todo/30`'s list is closed: the pointer and the selection,
§12.5.1's focus, §12.7's whole form and the four edits, save and extract with a byte accessor
apiece, §8.11.4.3's layers and §7.11.4's files as a second flattened panel, §12.4.4's clock and its
transitions, and the three policy values. Each is a *function*, which is the property ADR 0247's
first shape was chosen for.

Everything below is an application of a decision already taken. What is worth recording is the four
places where the shape had to say something new.

### `PDFV_EVENT_KIND_COUNT` did not move, and that is the finding rather than a footnote

Sixty-eight symbols arrived and a compiled C caller is unaffected by every one of them:
`pdfv_abi_check` answers `abi 1 (header 1), 16 event kind(s) (header 16)` before and after. A
`Command` is a symbol and a `Query` is a symbol; only an `Event` is a number, and no `Event` was
added. That is the third demonstration of the property in three rounds — the
four-hundred-and-twelfth changed a variant's shape under this crate for nothing, the
four-hundred-and-fourteenth moved the count 15 → 16 and made an old caller refuse at startup, and
this one adds two thirds of a vocabulary and moves nothing.

**The round was asked to expect the counts to move and they did not**, which is worth saying plainly
rather than quietly satisfying: the instruction assumed that adding to an ABI costs its callers
something, and the whole of ADR 0247's first shape is the answer that it does not have to.

### Two enumerations are answered with, and they are counted apart from the event kinds

`ControlKind` (eight) and `RowKind` (four) are numbers a caller switches on, so each gets the trio
an event kind has — a `PDFV_*_COUNT`, a `pdfv_*_count()` and a `pdfv_*_name()` answering `"unknown"`
for a number this build does not define. **Neither is added to `pdfv_abi_check`**, and the division
is the reason rather than the convenience:

| | how a caller meets a number it does not know | what it needs |
|---|---|---|
| an **event kind** | it *arrives*, unasked, in a batch a command produced | a check before the first one turns up |
| a **control kind**, a **row kind** | it is the answer to a call the caller wrote | a name to print in the `default:` arm |

And widening `pdfv_abi_check` would change the signature of the one function every compiled caller
already calls in `main`, which is precisely the hazard the four shapes exist to avoid. `kinds.rs`
states the rule at the top: an enumeration this ABI *takes* refuses a number it does not define; one
it *answers with* names it.

### A constant that had been missing since the count last moved

`PDFV_EVENT_SEARCHED` did not exist. `Event::Searched` moved `PDFV_EVENT_KIND_COUNT` 15 → 16 in the
four-hundred-and-fourteenth session and no `#define` was added for the kind itself, so a C caller
switching on kinds had to write `15` by hand for four months of rounds.

**`tests/header_and_library_agree.rs` could not see it**, and the way it could not is the lesson:
that test compares the constants the header *has* against a map of the ones it is *told* to expect,
so a constant absent from both sides agrees with itself. A table of expectations is only as complete
as the person who wrote it, and this one was checked in the same commit as the header it was
checking. The row is there now.

### Sixteen booleans are one word, because a struct by value is the expensive change

`pdfv_field_control` answers a `uint32_t` of `PDFV_FIELD_*` bits rather than sixteen accessors or a
third by-value struct. A bit added later is a bit an old caller does not read; a field added to a
struct passed by value is "a recompilation it has no way of knowing it needs", which is the row of
ADR 0247's cost table that `PDFV_ABI_VERSION` exists for. The header still has exactly two such
structs.

### And one thing crosses as text, deliberately

Table 164's `/S`. Every other enumeration here is a number this ABI invented, and this one is a
*name in the file* — the table's thirteenth case is `Style::Unrecognised`, "[a] name Table 164 does
not define, kept as the file wrote it". A number would have had to lose that one, and ADR 0230's
whole point is that a processor which cannot animate a style should be able to say which style it
could not animate.

### What it cost, and what it did not

- **`viewer-host` gained its fourth consumer and wanted no change.** `viewer_host::ControlKind` is
  what a C caller's `PDFV_CONTROL_*` is, unaltered: ADR 0246's third decision said a native host on
  this boundary is mostly not toolkit code, and a C host taking the *decision* — one variant per
  control a toolkit has for the job, rather than one per §12.7.5 type — is that tested rather than
  repeated.
- **The `unsafe` position is unchanged in kind**: one lint lift, 111 `#[unsafe(no_mangle)]`, 103
  signatures, and **no `unsafe` block anywhere in the crate**. Two crates in the tree lift
  `deny(unsafe_code)` and every crate touching PDF bytes still forbids it.
- **`c/open_a_page.c` drives the new surface**, including the form — which needed a document with a
  form, and the note it is run on has none. A corpus document was refused: `doc/pdf.js` is optional
  in a checkout and a gate that skipped when it is absent would be a gate that quietly stops
  running. The test writes eleven objects of hand-written PDF beside the test binary instead.
- **One assumption the C program made was wrong and the run said so**: a document stating no
  optional content answers an **empty list** rather than `PDFV_NO_ANSWER`. That is `viewer-core`'s
  existing choice and it is right — a document with no layers has answered the question — and it is
  pinned now, because "no layers" and "no document" are different sentences in a status bar.

---

## Part two — Table 229 bit 26, obeyed backwards

**This is the round's finding and it is a reading rather than a defect report.**

`doc/todo/30`, ADR 0235 and two ledger rows all said the same thing: `RadiosInUnison` "crosses and
is not obeyed", because "turning on every button of a set that shares an on state is a decision for
whatever handles the press". Reading the clause against the code found that sentence wrong in both
directions at once.

### The half everybody expected was already obeyed, by code that had never read the flag

A widget is on when its `/AP /N` states a stream under the name `/V` holds — §12.7.5.2.3's own
mechanism, implemented in `Field::replacement_state` since ADR 0235. So two widgets of a field that
share an on-state name **go on together by construction**, which is exactly §12.7.5.2.4's NOTE:

> An exception occurs when multiple radio buttons in a field have the same on state and the
> RadiosInUnison flag is set. In that case, turning on one of the buttons turns on all of them.

The flag being *set* needed no code at all, and never had.

### The half that needed code is the flag being **clear**, and it is stated in the wrong subclause

§12.7.5.2.3, three paragraphs after the `/Opt` entry's two stated purposes:

> For radio buttons, the same behaviour shall occur only if the RadiosInUnison flag is set. If it is
> not set, at most one radio button in a field shall be set at a time.

and Table 229's own row: "[i]f clear, the buttons are mutually exclusive (the same behaviour as HTML
radio buttons)." §12.7.5.2.1 makes both binding on a reader rather than on a producer — "[a]n
interactive PDF processor **shall follow the intended behaviour**", of bits 15, 16, 17 *and 26*.

**This tree turned them all on regardless of the flag.** The requirement it failed is the one nobody
had looked for, and the reason nobody had is worth keeping: the sentence is in the **check box**
subclause. A round reading §12.7.5.2.4 for a radio button's flag finds a NOTE describing the
exception and no requirement at all; the requirement is two subclauses up, attached to `/Opt`.
*Read the clause the entry is defined in, not only the clause the feature is named in.*

### Which button stays on is a documented choice, and the standard says why it has to be

The clause states none, and the *file* cannot: `/V` is a name, so a producer that gave two buttons
the same appearance-state name has written a document whose own value cannot distinguish them.
Table 230 is the standard's own instrument for a producer that wants them distinguishable — the
`/AP` names "may use numerical position (starting with 0) of the annotation in the Kids array …
This allows distinguishing between the annotations even if two or more of them have the same value
in the Opt array."

So the choice is the **first kid answering to the name**, which is the field's own order and the
order `/Opt` is indexed by. Recorded as a choice, in the code and in the ledger, rather than
presented as derived.

### It binds a value this reader replaced, and not the file's own `/AS`

`Field::an_earlier_button_answers_to` is consulted only in `replacement_state`, which runs only when
something has displaced Table 226's `/V` — a person, §12.7.6.3's reset, §12.7.8's import. A file
that states `/AS /Yes` on two widgets has said which of *its own* buttons are on, per widget, and
§12.7.5.2.3 gives that entry precedence over `/V`; bit 26 is a rule about what happens when a button
is turned **on**, and the party turning one on is this program. Correcting the document instead
would be inventing a repair nobody asked for, which is trap 5 pointing the other way.

One consequence is free and is the reason the fix went where it did: `Field::is_on` and
`appearance::appearance_state` both reach `replacement_state`, so the description a host reads and
the picture the page draws cannot disagree about which button went on. ADR 0235's finding was that
those two paths can be wrong *differently*.

### The corpus cannot see any of this, measured rather than assumed

`cargo run --release -p pdf-model --example field_flag_census` over all **1293** documents this tree
can reach — 964 openable of pdf.js's 974, and 329 of the four corpora's 337:

| | widgets | documents |
|---|---|---|
| bit 26 on a `Btn` (`RadiosInUnison`) | **0** | 0 |
| bit 26 on a `Tx` (`RichText`) | **0** | 0 |
| a radio field whose widgets share an `/AP /N` on state, flag set | **0** fields | — |
| the same, flag clear | **0** fields | — |

The census could not answer the first two rows before this round: bit 26 was counted as one row
called `RadiosInUnison/RichText` because Table 226's `/FT` was prose beside the count rather than a
filter. It is a filter now, which makes every type-specific row in that table accurate and not only
this one. The last two rows are the population that *actually* matters and no flag count would ever
have found it — a document can exercise the clause without setting the bit, because the bit's
clear case is a requirement too.

So the instrument is trap 8's: a pair of fixtures differing in one bit, in
`pdf-model/tests/radios_in_unison.rs`. Both new assertions were checked by deleting the rule; both
fail, and the three that describe the pre-existing behaviour keep passing, which is the evidence
that the flag-set half really was already right.

---

## Part three — the scale a form host draws at, answered with the messages that exist

ADR 0245 left this as a third decision: a platform control has a theme-decided minimum size, so a
control placed over a widget's `/Rect` can be larger than the rectangle and cover the page. ADR 0244
measured it on GTK and ADR 0246 on Qt, and *every* control is taller than its rectangle on some
forms — a property of platform controls rather than of a theme. `doc/todo/30` recorded the open
question as whether choosing the magnification needs a message the vocabulary has not got.

**It does not, and the argument is four steps of which only one did not exist:**

1. `Query::Fields` already answers with every widget's `/Rect` in device pixels of the viewport;
2. the control's minimum is the *toolkit's* answer and no clause's — each host asks its own;
3. a control's minimum does not change with the page's magnification and the rectangle's size does,
   in proportion, so the magnification at which everything fits is the current one times the worst
   ratio of minimum to asked. **This is the piece that did not exist**, and it is
   `viewer_host::ControlFit` — toolkit-free, for `panel.rs`'s reason: two hosts measuring the same
   thing must not be able to compute two different answers from it;
4. `Command::Zoom { zoom: Zoom::Scale(that) }` applies it.

`viewer-gtk`'s private `ControlFit` moved into `viewer-host` and gained the arithmetic; `w` sends the
answer. Driven under `Xvfb` on `160F-2019.pdf`:

```
11 of 76 control(s) wider than their /Rect (worst +85 on 120 px), 76 taller (worst +22 on 12 px);
  every control fits at 3.278, which `w` sends
note: fitting §12.7's controls at 3.278
 0 of 76 control(s) wider than their /Rect (worst +0 on 0 px), 0 taller (worst +0 on 0 px);
  every control fits at this magnification
```

The counts before the key press are ADR 0245's own numbers to the pixel, which is what says the
refactor measured the same thing; the line after it is the answer. **No message was added.**

**What is deliberately not decided is *when*.** A viewer that magnified a page by itself because a
form is on it would be answering a question nobody asked, and which gesture asks for it — a key, a
menu item, a preference — is chrome and therefore a host's (rule 5). The crate answers what the
number is.

**Qt does not use it and the reason is honest rather than good**: its measurement is in
`cpp/window.cpp`, on the C++ side of the bridge, so feeding `ControlFit` would mean carrying the
pairs across `cxx`. That is a bridge change and it is left in `doc/todo/30` as one, with its cost
named.

## Consequences

- **`doc/todo/30`'s entry-point list is closed.** 43 → 111, and `PDFV_EVENT_KIND_COUNT` is 16 before
  and after.
- **A `#define` had been missing since the last time that count moved**, and the test that exists to
  stop the header drifting could not see it because it compares against a hand-written map.
- **Two ledger rows and one todo file said `RadiosInUnison` is not obeyed, and the true statement was
  that half of it was obeyed by accident and the other half was violated.** The requirement is in
  §12.7.5.2.3, attached to `/Opt`, and not in the radio-button subclause the feature is named after.
- **`field_flag_census` counts bit 26 by field type**, so "does any document set `RadiosInUnison`"
  is a command rather than an inference, and it counts §12.7.5.2.4's own population beside it —
  which is the thing a flag count cannot see.
- **ADR 0245's third decision is closed for GTK and open for Qt**, and closing it needed no message:
  eleven messages in twelve rounds of hosts, and the twelfth added none.
