# Read the ledger's `partial` rows against the code

Status: **standing task.** ~180 of the 236 rows have not been re-read.
Priority: 01 — the population with no gate, and it has paid on every session that touched it
Code: `doc/conformance/ledger.toml`, checked by `cargo test -p conformance`

## Why

All 823 subclauses of the eight technical clauses have been read against this code since the
fifty-sixth session, and the statuses are gated: `silent` is **zero**, `REVIEW_OWED` is empty and
fails the build the moment a cited-but-unread clause appears, and `FILE_ONLY_EVIDENCE_CEILING` is
zero and asserted with `==`.

What no gate can watch is a **note that has gone stale**, and the 238 `partial` rows are where
those live. Five failure shapes, in the order they were found:

1. A note that *understates* what the code does (five in session 115).
2. A note whose **reason** has expired — "while §X does not exist", "needs §Y" (117, 118).
3. A note claiming an entry is *unread* where the tree reads it (three in 122, five more in 159).
4. A note whose "what IS done" half is wrong — **the class that resists a grep**, because the
   name being present is what a grep looks for.
5. A note that is *stale about its neighbour*: §7.7.2 listed eighteen catalog entries as unread
   that were read, most of them by the session that built their clause. **A family's parent row
   is not maintained by the sessions that implement its members**, because the clauses do not
   cite each other. Four instances so far (§12.3's parent, §14.8.5.1's, §7.7.2's, and §7.9's in
   the two-hundred-and-seventy-eighth — it called dates, name trees and number trees "features
   this tree does not have yet" while all three of its own child rows read `implemented`, one of
   them with a gate over 1545 corpus date strings).
6. **A note that contradicts itself**, found in the two-hundred-and-seventy-eighth. §12.5.6.5 said
   "`/H`'s highlighting mode is still a response to a mouse this program does not draw" and, four
   sentences later in the same note, that Table 176's `/H` "is honoured since the
   hundred-and-thirty-eighth session … (ADR 0123)". The row was corrected by *appending* the new
   sentence and nobody re-read the paragraph above it. This is the cheapest shape to find and the
   only one whose evidence is entirely inside the row: read a corrected note **whole**, not from
   the correction onwards.

## The three sweeps

Twenty lines of Python apiece, each of which paid on its first run. Run all three after any round
that adds a verb.

| sweep | looks for | first catch |
|---|---|---|
| expired blocker | `while §X does not exist`, `needs §Y`, `until §Z` | session 118; found §9.7.5.2's "a licensing decision" 150 sessions after the decision |
| entry claimed unread | every `/Key` in a "Not read:" list, grepped against the tree | six of ten lists had a live entry; §7.7.3.3's had eleven of eighteen |
| capability | `this program has no ___`, `no panel`, `which this is not` | §12.6.3's "this crate has no events", 41 sessions after `Command::Pointer` |
| **retired claim** | the *string* a correction retired, grepped over every other row | §8.9.6.1 still said "reported rather than applied on 28 corpus documents" fourteen sessions after §11.6.4.3 retired that exact sentence |

**And a fifth: run all four over the *source tree*, not only over the ledger.** The
two-hundred-and-twentieth session found `icon.rs`'s module comment blocked on a flag that had
arrived three sessions earlier, by accident. Run deliberately in the two-hundred-and-twenty-first
it produced four more, all of them false for between forty and two hundred sessions:

- `pdf-model`'s own **crate documentation** opened "[t]ext and images are not yet drawn", true of
  the sixth session; `content.rs`'s module comment said the same. A crate's front door is where a
  reader learns what it does, and it outlives every ledger row that says otherwise.
- `set_dash`'s doc comment said "only the 'solid line' case is honoured for now" — the sentence
  from *before* ADR 0018, on the function the handover calls the archetype of a feature switched
  off in one place.
- `requirements::unmet` named three of §12.11's requirements as unmet whose capability had
  arrived: `OCInteract` (a layer panel, session 167), `AcroFormInteract` (a field a person types
  into, 135) and `Attachment` (`Command::Extract`, 167). **Its own doc comment had predicted it**
  — "a session that builds a layer panel has to come back and change `OCInteract`" — which is the
  strongest form of this failure: a warning written where the work is does not fire either.

**The ledger has a gate and the source does not**, which is why these lasted longer. One `grep`
apiece.

**Run again in the two-hundred-and-sixty-ninth over `crates/`**, after ten rounds that added
panels and verbs: 89 matches, 87 of them true statements about what a *crate* deliberately does
not own — no clock, no filesystem, no toolkit — which is the shape this sweep produces most and
which is worth knowing it produces. The two that were false had both expired in the ten rounds
themselves: `outline.rs` opened "[a]n outline is a *panel* in a viewer that has none", false since
session 166, and `pdf-viewer.rs` said `/PageMode` had "[t]hree of the six it can now obey", false
since the session before. **A comment about a sibling crate's capability decays at that crate's
pace and not at its own.**

## A fifth sweep, from the other direction: **who calls it?**

The three above ask what a *row* claims. The two-hundred-and-fifty-third and -fourth sessions
found two clauses neither of them could see, and both were the same shape from opposite ends: a
capability arrived, and nobody maintains the *callers* of the code it unblocks, because the code
and the caller do not cite each other either.

- **§12.5.6.19's `/H`** was `implemented`, argued in ADR 0123 and tested with pixels — and
  `viewer-core` took the pressed annotation from `link_at`, so no host could press a widget for a
  hundred and fifteen sessions (ADR 0177).
- **§8.11.4.3's `/ListMode`** was read into `OptionalContent::list_mode` and asked by nothing,
  with a layer panel on the screen (ADR 0178).

**Run again in the two-hundred-and-seventy-eighth, and it produced a clause on its second run
too**: 175 functions now, 69 that neither host names, and `Signature::must_cover_whole_file` among
them. Table 255 makes the byte range's coverage a `shall` for two of §12.8.1's sub-filters and a
`should` for the rest; `viewer_core::notes` worded an uncovered tail identically for all of them,
so a file breaking that `shall` read as §12.8.1's NOTE 1 ordinary incremental update. The model
had the distinction, tested; the only host ignored it. **Nothing in the corpus exercises it** —
all six of the 974's signatures are `adbe.pkcs7.*` — which is why it could not have been found
from the demand side at all (trap 8).

The sweep is twenty lines: every `pub fn` in `pdf-model`, grepped against `viewer-core` and
`viewer-ui`. 174 functions, 72 that neither names. Most are internal helpers that happen to be
`pub`; read the ones whose name is a *clause's noun*. What it produced beside `list_mode`, unread
so far: `logical_order` and `logical_text` (§14.8.2.5, which `doc/todo/33` owes), `beads_on_page`
(§12.4.3's articles), `all_folders` and `folder_of` (§7.11.6's collection folders, with an
attachments panel already drawn), `document_language`, `alternate_description` and `actual_text`
(§14.9, waiting on `doc/todo/31`'s host), `print_field` and `user_properties` (§14.8.5's
attributes), `widgets_by_field_name` and `clear_field` (§12.7).

**The fourth sweep paid again in the two-hundred-and-thirty-eighth, on its own subject.** Run
after five rounds that corrected rows, over the *mechanisms* those corrections named rather than
over their exact words, it produced two:

- **§12.5.5** still ended "Not implemented: the NoZoom and NoRotate scaling this clause defers
  to §12.5.3" — twenty sessions after ADR 0168 applied it. The clause that *defers* is where
  the stale sentence lives, which is the shape exactly: correcting §12.5.3 left its neighbour
  lying.
- **§12.5.6.22**'s `/FixedPrint` was still explained by "a resolution-independent display list
  cannot express a size that depends on the view" — the *refusal* ADR 0168 dismantled. The claim
  it supports is still true (`/FixedPrint` waits on printing) and its reason was not, which is a
  fifth way for a row to be wrong: **right conclusion, expired argument.**

So the sweep is worth running over a *mechanism* and not only over a quoted string: grep for
`NoZoom`, `uncoloured`, `ColorTransform` — the noun the correction was about — and read every
row that holds it.

**And it paid a fourth time in the two-hundred-and-ninety-fifth, on the noun the round before had
just given the tree**: `XMP`. Four rows and four source comments still said this program reads
none — §7.7.3.3's page attributes, §8.9.5.1's image dictionary, §14.3's parent, `metadata.rs`'s
own crate-level paragraph, `has_metadata_stream`'s doc comment and `chrome.rs`'s properties panel
— and three of the six had been written *by the round that retired them*, one file away.
**Running this sweep in the same round as the correction is the cheapest it will ever be**, and it
is now what `02-every-round.md` step 4 means by "after a round that adds a verb".

And it produced a fifth row nobody was looking for. **§14.3.4 was `inapplicable`** on the reason
"a question for a program that *writes* or *displays* metadata, and this one does neither" —
false twice, since the hundred-and-thirty-sixth session and the hundred-and-seventy-third
respectively. Reading the clause then found one rule that binds and is met by construction
(§7.5.6's update leaves both metadata sources byte for byte), one excluded by `CLAUDE.md`, one
`may` declined with a reason, and **one `shall` that staying out of the way of is a decision**:
writing a `/ModDate` on save would oblige this program to write `xmp:ModifyDate` too, so the cost
of a date nobody asked for is an XMP *writer*.

**And it paid a third time in the two-hundred-and-ninetieth**, on the nouns four rounds had just
corrected — `notdef`, `Differences`, `SigFlags`. §12.7.4.3 still said "4 now draw their fields and
2 keep the blank because Helvetica has no glyph for their characters", six sessions after §9.6.5.1
gained the `/Differences` that draws one of the two and one session after this project established
that the reason was never a glyph. **Two rows, one mechanism, and the one that was corrected is not
the one that was wrong** — which is this sweep's whole subject, and the third time it has been the
*font* rows.

The same run over `crates/` was clean: 64 capability matches, every one a true statement about a
boundary a crate deliberately keeps — no clock, no filesystem, no toolkit, no window — down from 89
in the two-hundred-and-sixty-ninth because rounds since have retired the false ones. A clean run is
a result: it says the population has not drifted, which is the only way it is watched at all.

**The fourth sweep is new in the two-hundred-and-sixteenth session and it is the cheapest of the
four.** Whenever a row is corrected, the note says so in the row that was corrected — "this
sentence said X" — and X is a string. Grep the *whole* ledger for X's distinctive words: two
clauses describing one mechanism is the commonest shape in this file, and correcting one leaves
the other lying. It found §8.9.6.1 on its first run, and a second understating row beside it:
§8.9.5.1 said `/Mask` was "read only to report it", which stopped being true in the *fourteenth*
session (ADR 0023) and is the **third** entry in that one list to be recorded as unread while the
tree read it — a list that has been wrong three times about itself is a list to check rather than
to read.

Three false-positive shapes on the second, all seen: a note *quoting* its own retired wording
(§9.6), a key named in a sentence about something else (§12.7.5.3), and a key that is a string in
an unrelated list (`/Metadata` in `thumbnail.rs`). Read the hit before believing it.

**A fourth, and it is the most common: one short key, three clauses.** The two-hundred-and-ninth
session's run of the second sweep produced five hits and *all five* were this — §8.4.5's `/BG`
and `/TR` are Table 57's device transfer and black generation, while `appearance.rs`'s `"BG"` is
Table 232's widget background and `soft_mask.rs`'s `"TR"` is Table 145's soft-mask transfer
function. Three clauses, two names, nothing stale. **A clean run of a sweep is a result**: it says
the population it watches has not drifted since the last one, which is the only way that
population is ever watched at all.

## The shape the sweeps found last, and it is a new one: the blocker was the *interface*

The two-hundred-and-fourteenth session ran the capability sweep after a round that added a verb.
**§14.9.3** said Table 226's `/TU` "names a field in a user interface this program does not have"
— the familiar shape, and false since the hundred-and-thirty-second session put a window on this
program. But the window was never what blocked it. The clause is a `shall`:

> An alternative name may be specified for an interactive form field (see 12.7, "Forms") which, if
> present, shall be used in place of the actual field name when an interactive PDF processor
> identifies the field in a user-interface.

and `Query::FieldAt` answered with **one string**, which cannot be both the identity
`Edit::SetField` addresses and the label a person is shown. So the row would have gone on being
true-looking however many windows arrived: what had to change was the *answer's shape*. ADR 0167.

**The lesson for the sweep**: when a row's reason names a capability, ask what the program would
have to *say* to obey the clause, not only what it would have to have. A row can survive the
arrival of the very thing it names.

## The shape the sweeps found before that, and it is the longest-lived

The two-hundred-and-first session ran the capability sweep again. **§12.3.2.1** said a
destination's other two items — "[t]he location of the document window on that page" and "[t]he
magnification (zoom) factor" — are "properties of a window with scrolling and zoom, which this
program does not have". `Command::Zoom` and `Command::Scroll` had been in the vocabulary since
the **hundred-and-thirty-second** session: sixty-nine of them, the longest any of these has run.

The tell is the same every time: the row explains itself by naming something the *program* lacks
rather than something the *standard* leaves open. `viewer_core::Open::apply_view` answers all
eight of Table 149's forms now, and the row is `implemented`. ADR 0162.

## The shape the sweeps found before, and it is the strongest

The hundred-and-ninety-first session ran all three. §12.8.6 said a usage-rights signature grants
"features of a PDF processor that are not available by default" and that **"this program has no
feature behind such a gate"**; §12.8.2.3 said the same. Both were true when written and both
stopped being true in the hundred-and-thirty-fifth and -sixth sessions, when this program learned
to fill in a field and save a file — which are exactly the rights Table 258 grants and exactly
the changes Table 257's `/P` restricts.

And the requirement was not new. §12.8.2.2.1 has always carried, in a parenthesis:

> (These changes to the document shall also be prevented if the signature dictionary is referred
> from the DocMDP entry in the permissions dictionary.)

A `shall`, addressed to a processor that modifies, unread for fifty-six sessions after this one
became one. `ViewState::set_field` obeys it now.

**So: after a round that gives the program a verb, re-read the rows whose reason is about what
the program *is*, not only the ones about what a clause needs.** The same shape as §7.6.3.2's
random initialisation vector, which sat in an `implemented` row for a hundred and twenty sessions
because a reader only ever *reads* one (ADR 0129).

## What is still owed, named

- ~~**§12.8.2.3's `should`**~~ — closed in the hundred-and-ninety-eighth session (ADR 0159).
  Table 258's rights are read, `ViewState::save` rewrites the permissions dictionary without its
  `/UR3` where a save would exceed them, and the condition was *counted* before it was trusted:
  all four corpus documents carrying a `/UR3` grant what this program does, so no file here can
  trip it. What is still owed under §12.8.2.3 is §12.8.2.2.2's comparison of two revisions, which
  needs the digest.
- **~180 `partial` rows** not yet re-read against the code.
- **The 30 `reported` rows are worked out** — all read in the hundred-and-twenty-first and
  -second, and none is of the two known failure classes (a true observation about the wrong half
  of a sentence, ADR 0109; a clause with two populations where the row names one, ADR 0110). 17
  are cryptographic validation needing a trust store, 5 need a second file or a network, 3 are
  icon clauses whose own verb is *should*, and the rest name a device or a user control this
  program does not have.
