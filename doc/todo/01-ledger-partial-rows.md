# Read the ledger's `partial` rows against the code

Status: **standing task.** ~176 of the 240 rows have not been re-read. **The seventh sweep, below,
is the first to read the `inapplicable` rows** — 81 of them, never swept before the
three-hundred-and-fifty-ninth session, and its first run corrected five.
Priority: 01 — the population with no gate, and it has paid on every session that touched it
Code: `doc/conformance/ledger.toml`, checked by `cargo test -p conformance`

## Why

All 823 subclauses of the eight technical clauses have been read against this code since the
fifty-sixth session, and so are the 52 of the eight normative annexes since the
three-hundred-and-sixtieth; the statuses are gated: `silent` is **zero** — it was five from the
three-hundred-and-sixtieth to the three-hundred-and-sixty-ninth, when Annex O was built — `REVIEW_OWED` is empty and
fails the build the moment a cited-but-unread clause appears, and `FILE_ONLY_EVIDENCE_CEILING` is
zero and asserted with `==`.

What no gate can watch is a **note that has gone stale**, and the 240 `partial` rows are where
those live. Six failure shapes, in the order they were found:

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

## A sixth sweep, and it is arithmetic rather than a grep: **which parents are behind their children?**

Twenty lines of Python over `ledger.toml` alone, no source and no clause text: build the map of
clause → status, and print every row that is `partial`, `reported` or `unreviewed` while **every
one of its direct children** is `implemented`, `inapplicable`, `out-of-scope` or `writer-side`. A
parent cannot owe less than its children and it can easily owe more, so a hit is not automatically
wrong — but it is always a row nobody has re-read since the last child closed.

**Its first run, in the two-hundred-and-ninety-eighth, produced five and four of them were wrong.**
Three in a shape this file had not named:

- **§7.6.3** was `partial` and its own note opened "[b]oth algorithms are implemented in both
  directions".
- **§9.10** was `partial` and its note said "[a]ll three of §9.10.2's methods are implemented since
  the hundred-and-fifty-sixth session" — a hundred and forty-two sessions earlier.
- **§14.3** was `partial` and its note, corrected four rounds before, said every subclause was
  read.

So the sixth failure shape is **the note was corrected and the status was not**. It is the exact
inverse of shape 1 and it is invisible to every grep in this file, because the note a sweep reads
is the half that is *right*.

The fourth was the ordinary fifth shape with a long fuse: **§9.7.6** said "what is missing is the
predefined `CMap` data §9.7.5.2 owns", which shipped in the hundred-and-fifty-sixth session
(ADR 0140, all 239 of them) — and §9.7.5.2's own row has read `implemented` ever since.

The fifth, **§7.9.2**, was read and kept: its `partial` is the object model carrying one string
type where §7.9.2.1 names three, which is a true statement about this tree.

**And the run found one more thing, which is about this file rather than about the ledger.**
§7.9's parent row still said "[d]ates, text streams, name trees and number trees belong to
features this tree does not have yet" — false of three of the four, with a 1545-string gate over
one of them. `doc/todo/01` records that the two-hundred-and-seventy-eighth session's run *found*
exactly this row, and the row was never changed. **A correction recorded in a todo file is not a
correction**, and the only defence is to make the change in the same commit that finds it.

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

## The seventh run of the fifth shape, and it is the sweep's own subject

**§8.11.2.1 named two Table 96 entries as read by nothing, and the tree read both** — found in the
three-hundred-and-eighteenth session by reading the §8.11 family top to bottom rather than by any
grep, because the row's sentence contains no blocker, no capability and no retired string:

> Two Table 96 entries are read by nothing: /Name, which exists to be shown in a user interface,
> and /Usage, which feeds the automatic state setting of §8.11.4.4.

`/Usage` has been read since the **thirty-fifth** session — §8.11.4.4's usage application
dictionaries fetch it per group and evaluate Table 100's categories against it — and `/Name` since
the **sixty-seventh**, with `viewer_ui::chrome` putting it on a row since the hundred-and-sixty-
seventh. So the entry whose stated purpose is "presentation in an interactive PDF processor's user
interface" was recorded as unread for a hundred and fifty sessions after a panel existed to present
it. §8.11.2.1 is `implemented` and its parent §8.11.2 with it, which is the sixth sweep's shape
arriving one round after the row it depended on was fixed.

**And the same shape one clause along, in the three-hundred-and-nineteenth**: §8.11.3.2 said the
`DP` form was "not implemented", sixty-five sessions after ADR 0178's `groups_referenced_by` covered
it *by construction* — the clause's sentence has one consequence, a reference, and the walk that
answers `/ListMode /VisiblePages` reads the page's `/Properties` rather than interpreting the
stream. Two rows in one family in two rounds, both stale for the same reason: the session that
implements a mechanism does not maintain the rows of the clauses that need it.

**What this adds to the method**: the four greps and the arithmetic all read a row's *reason*, and
this row gave none — it simply listed two keys. The second sweep is the one that should have caught
it (an entry claimed unread) and it did not, because the sentence says "read by nothing" rather
than "Not read:". **Grep the shape, not the wording**: `read by nothing`, `is unread`, `nobody
reads` are the same claim.

## And the fifth shape found in the *code* rather than in a row, in the three-hundred-and-twenty-fourth

`optional_content.rs` explained answering Table 100's `Zoom` category at a magnification of 1.0 by
saying that "a display list has no magnification … the alternative is to thread a scale into
`interpret` and rebuild the display list per zoom, which is a viewer's design question rather than
a clause's". **The tree answered that design question in the two-hundred-and-seventeenth session**:
§12.5.3's `NoZoom` threads exactly such a scale through `ViewState::magnification`, and
`Interpretation::view_dependent` says which pages notice (ADR 0168). So the conclusion was right
and the argument had expired — `doc/todo/01`'s fifth shape, in a doc comment rather than in a
ledger row.

**What replaced it is a measurement**, because the clause has a `shall` behind it (§8.11.4.5:
"[w]henever there is a change to a factor that the usage application dictionaries with event type
View depend on (such as zoom level), the corresponding dictionaries shall be reapplied").
`examples/oc_usage_census` reads every configuration's `/AS` in all 974 documents: 31 state
`/OCProperties`, **six** state a usage application dictionary, and they name `View`, `Print` and
`Export` — **`Zoom`, `User` and `Language` not once**. A path nobody takes is one `CLAUDE.md`
forbids shipping, and now the row says so with a number instead of with an architecture.

## The six sweeps run again in the three-hundred-and-thirty-second, and a clean run is the result

After six rounds that added verbs — §12.5.6.10's markup, `Page`'s identity, §9.10.2's second
method reaching Type 3 fonts — all six were run over `ledger.toml` and over `crates/`:

- **The arithmetic sweep**: one hit, §7.9, which `doc/todo/01` already records as read and kept.
- **Expired blockers**: six hits, every one a row naming a clause it genuinely waits on
  (§11.4.6's knockout groups, §12.10.3's geospatial, §12.6.4.11's hide action).
- **Capability reasons**: 35 hits and every one a true statement about a boundary this tree keeps
  — no clock, no filesystem, no printing path, no comments pane. Two are the *quoted retired
  wording* inside a correction, which is this sweep's oldest false-positive shape.
- **Entries claimed unread**: the same nine §8.9.5.1 and §8.4.5 hits the two-hundred-and-ninth
  session identified as one short key in three clauses.
- **The caller sweep**: 198 `pub fn`s in `pdf-model`, 71 named by neither host. The interesting
  names are all one of three known populations — §14.7/§14.9's structure entries waiting on
  `doc/todo/31`'s host, §7.11.6's collection folders and §12.4.3's beads waiting on
  the panels that now exist (ADRs 0200 and 0202), and functions `pdf-model` calls *itself* (`unresolved_usage` is read by
  `content.rs`, `added_on` by the interpreter), which the sweep cannot see and which are worth
  knowing it cannot.

**A clean run says the population has not drifted**, which is the only way it is watched at all.

## The six run again in the three-hundred-and-forty-second, and the third sweep paid

After three rounds that added verbs — §12.7.5.3's `DoNotScroll`, `LaidOut::overflows`,
`QuorraRasterizer::rasterize_frame` — all six were run over `ledger.toml` and over `crates/`:

- **The arithmetic sweep**: one hit, §7.9, which this file already records as read and kept.
- **Expired blockers**: seven, every one a row naming a clause it genuinely waits on.
- **Entries claimed unread**: fourteen, all of them lists whose entries were checked in the
  two-hundred-and-ninth and three-hundred-and-thirty-second runs, plus §12.7.5.3's own — which the
  round that wrote it had just corrected.
- **Capability reasons**: 24 hits, 23 of them true statements about a boundary this tree keeps.
- **The caller sweep**: 198 `pub fn`s in `pdf-model`, 71 named by neither host — the same three
  known populations.

**The twenty-fourth capability hit was §12.5.6.2 and it had expired thirty sessions earlier.** The
row said `/Subj`, `/RC`, `/IRT`, `/RT` and `/IT` "reach a comments pane this program has no panel
for", and four of the five still do — but Table 172 makes `/RC` "[a] rich text string … that shall
be displayed in the **popup window** when the annotation is opened", and `viewer_ui::chrome` has
drawn that window since the three-hundred-and-twelfth session (ADR 0191). ADR 0199 reads it now.

**What that adds to the method**: a row that lists several entries behind one reason is several
claims, and the sweep reads the reason. §12.5.3's `NoZoom`/`NoRotate` was the same shape in the
two-hundred-and-seventeenth — "**split a refusal into one claim per entry before believing it**" —
and this is that rule applied to a *capability* reason rather than to an architectural one. Five
entries, one sentence, and only one of them named a capability that had arrived.

## A seventh sweep, and it reads the rows the sweeps had never looked at: **the `inapplicable` ones**

Every sweep in this file walks `partial`, `reported` and `unreviewed` rows, because those are the
ones that owe something. **`inapplicable` was never swept**, and it is the status a row goes to
when nobody expects to come back — which is exactly the property that lets a wrong reason live
there. The project owner asked for the re-read in the three-hundred-and-fifty-eighth session, after
§10.5's transfer function turned out to be `inapplicable` on a phrase — "marking device" — the
standard does not contain.

The sweep is twenty lines and mechanical: for each `inapplicable` row, take the capitalised
identifiers and `/Key` names out of its own title and note, and grep `crates/*/src` for each. **A
row claiming the tree does not do a thing, whose own vocabulary the source names, is a row to
read.** It hit 49 of 81; most are noise (`DeviceCMYK`, `XObject`, and the sweep's own English —
`Nothing`, `Whether`), and the signal is a *rare* word: `GoToDp` in three files under a `§14.12`
row, `DPart` in four.

**Its first run corrected five rows and amended two more**, and all five were the same shape — a
`§14` row saying a screen does not do this, beside a `§12` row saying the tree draws it:

| row | said | the clause says |
|---|---|---|
| §14.11.3 printer's marks | "outside what this viewer draws … a screen is not a printer" | "[t]he Print and ReadOnly flags … shall be set and **all others clear**" — `NoView` clear |
| §14.11.6.2 trap networks | "drawing it on a screen would paint the artefact-hiding overlaps *as* artefacts" | the same flags sentence, verbatim |
| §14.12.4, §14.12.4.1 document parts | "[n]either is read, and neither reaches a screen" | Table 409's `/Start` is what §12.6.4.5's `GoToDp` shows |
| §14.9.6 pronunciation | `inapplicable`, "the same reading §10.7.2's flatness permission gets" | §10.7.2 is `implemented`, on `CLAUDE.md`'s own rule |

`PrinterMark` and `TrapNet` are both in `annotation.rs`'s `STANDARD_SUBTYPES` and always have
been, and §12.5.6.20 and §12.5.6.21 said so in their own notes. **So the ledger held both answers at
once, in two families, for a hundred sessions.** The sixth sweep cannot see this: it compares a
parent with its children, and these pairs are cousins.

**What that adds to the method, and it is the sweep's own generalisation**: a mechanism gets one row
per clause that mentions it, and the rows are written in different sessions by different reasoning.
Shape 7 is **two rows about one mechanism, disagreeing** — and the tell is that one of them names a
*capability* ("a screen is not a printer") while the other names *code*. When a row's reason is
about what this program is rather than about what the clause says, find the other row.

**Run over the 87 `out-of-scope` rows in the same sitting, it produced no hit.** 26 of them name
something the source names — `RichMedia` in two files, `ECMAScript` in seven, `Rendition` in one —
and every one is a refusal the row already describes: §12.5.6.25 says in its own note that a
`RichMedia` annotation's "appearance streams are drawn where they exist, like any other
annotation's, because nothing in the placement path switches on subtype", which is exactly the
sentence §14.11.3's row was missing. **A clean run on a population is worth recording**, because it
is the only way this file knows a population has been read at all.

**And one distinction the run had to make rather than blur.** §14.11.2.2's guidelines are
`inapplicable` for a different reason than §14.10's web capture: the first is a **permission this
program declines** ("[i]nteractive PDF processors **may** offer the ability to display guidelines"),
the second is a clause about a thing this program is not. `CLAUDE.md` says a permission read is the
stronger answer, and §10.7.2 is `implemented` for exactly that — but it earns the status by naming
code that reads `i` and discards it. §14.11.2.2 has no code to name, so it keeps `inapplicable` with
its reason stated precisely. **The status vocabulary has one word for two situations**, and until
that is worth a status of its own the defence is that every such note says which it means.

## What is still owed, named

- ~~**§12.8.2.3's `should`**~~ — closed in the hundred-and-ninety-eighth session (ADR 0159).
  Table 258's rights are read, `ViewState::save` rewrites the permissions dictionary without its
  `/UR3` where a save would exceed them, and the condition was *counted* before it was trusted:
  all four corpus documents carrying a `/UR3` grant what this program does, so no file here can
  trip it. What is still owed under §12.8.2.3 is §12.8.2.2.2's comparison of two revisions, which
  needs the digest.
- **~176 `partial` rows** not yet re-read against the code.
- ~~**Annex I.2's version number**~~ — closed in the three-hundred-and-sixty-first session, the
  round after the sweep that found it (ADR 0207). It was worth one line here for exactly one round:
  a `should` nobody had read, two lines from a parser already standing on the number.
- **A dangling `doc/todo/20`** in §8.9.6.2's note — a stencil painted with a tiling pattern, whose
  file no longer exists. Either the work is real and wants a file, or the sentence wants rewriting;
  the three-hundred-and-sixtieth session fixed the other two dangling references it found and left
  this one because the refusal behind it has not been re-read.
- **§14.11.6.2's one reader-side sentence**, found by the seventh sweep and left unread: if the
  page object's `/LastModified` is more recent than the trap network annotation's, "the page's
  trap networks are invalid and shall be regenerated" — and a reader that cannot regenerate them
  is drawing traps the clause has called invalid. No corpus document states a `/TrapNet`, so this
  is a clause to read rather than a defect to fix, and the round that takes it owes the count
  first, the way `doc/todo/13` did.
- **§7.9.3 closed in the three-hundred-and-forty-sixth**, and it is the first `reported` row to close by a capability this tree gave *itself* one round earlier. The row named its own expiry condition — "this closes the day an entry in scope uses the type" — and ADR 0199's reading of Table 172's `/RC` was that day. Six entries in the whole standard are typed `text string or text stream` and `/RC` is the only one in scope, so implementing the clause was implementing it once. **A row that states its own trigger still has to be re-read by somebody**, and this one waited a round.
- **The 29 `reported` rows are worked out** — all read in the hundred-and-twenty-first and
  -second, and none is of the two known failure classes (a true observation about the wrong half
  of a sentence, ADR 0109; a clause with two populations where the row names one, ADR 0110). 17
  are cryptographic validation needing a trust store, 5 need a second file or a network, 3 are
  icon clauses whose own verb is *should*, and the rest name a device or a user control this
  program does not have.
