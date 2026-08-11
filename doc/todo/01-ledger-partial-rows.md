# Read the ledger's `partial` rows against the code

Status: **standing task.** ~80 of the **248** rows have not been re-read — 29 went in the
three-hundred-and-seventy-fifth, 16 in the three-hundred-and-eighty-seventh, 11 in the
three-hundred-and-ninety-fourth, 9 in the four-hundred-and-second, 14 in the
four-hundred-and-thirteenth, 8 in the four-hundred-and-twenty-ninth and **30 in the
four-hundred-and-thirty-seventh**, against a population that grew by 12 in between and has now
fallen by 4. **Fourteen sweeps**, twelve run every round, a thirteenth run once and declined
(ADR 0265), and **the fourteenth built in the four-hundred-and-thirty-seventh** — a `partial` row
whose note names nothing owed, which breaks the ledger's own definition of the status. Its first
run printed 16 rows and seven were defects, two of them statuses: §9.3, `partial` on an expired
reason for **365 sessions**, which is the longest this file has recorded. **The ninth sweep is the first to check that a citation names the *right* table**,
and its first run corrected nine ledger rows and nine source comments — a whole block of §12.5.6's
annotation tables and a whole block of §14.8.5's attribute tables, every one of them ISO 32000-1's
number for something else. **Its second run, seven rounds later, found two more and one of them was
in the round before's own work; its third, eight rounds after that, found three and all three were
in the *source*, beside ledger rows that had been corrected without them.** **The tenth sweep is new in the four-hundred-and-second** — it compares a parent row's stated
*count* of its children with what the children say — and it paid twice on its first run and again on
its second. **The eleventh is new in the four-hundred-and-thirteenth**: it reads the ledger's
*quotation marks*, which no gate in this project has ever done, and its first run found six
misquotations of the standard (ADR 0249).
Priority: 01 — the population with no gate, and it has paid on every session that touched it
Code: `doc/conformance/ledger.toml`, checked by `cargo test -p conformance`

## Why

All 823 subclauses of the eight technical clauses have been read against this code since the
fifty-sixth session, and so are the 52 of the eight normative annexes since the
three-hundred-and-sixtieth; the statuses are gated: `silent` is **zero** — it was five from the
three-hundred-and-sixtieth to the three-hundred-and-sixty-ninth, when Annex O was built — `REVIEW_OWED` is empty and
fails the build the moment a cited-but-unread clause appears, and `FILE_ONLY_EVIDENCE_CEILING` is
zero and asserted with `==`.

What no gate can watch is a **note that has gone stale**, and the 249 `partial` rows are where
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
| **retired claim** | the *string* a correction retired, grepped over every other row **and over `doc/adr/`** | §8.9.6.1 still said "reported rather than applied on 28 corpus documents" fourteen sessions after §11.6.4.3 retired that exact sentence |

**`doc/adr/` joined the fourth sweep's targets in the four-hundred-and-twenty-ninth**, and it is the
only change ADR 0265 makes to this file. An ADR is a dated record whose prose nobody maintains, so
the general sweeps over it are 64 hits to 2 defects and are declined — but a claim a *later* round
disproves and leaves standing is a defect wherever it lives, and the round that disproves it amends
the ADR that made it, in the same commit. ADR 0261 said `confined_wire` reaches `pdf_model::interpret`
and that `cargo-fuzz` is not installed here; the four-hundred-and-twenty-eighth disproved both with
`nm` and with `ls ~/.cargo/bin`, and wrote the correction into its own commit message rather than
into the ADR.

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

## All seven run again in the three-hundred-and-seventy-fifth, after five rounds that added verbs

`Query::Caret`, `pdf_model::restriction`, `pdf_render::repeat`, `ImageSource` and
`pdf_model::fragment` had landed in five consecutive rounds and none of them had re-swept, which is
the condition these exist for. Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 which this file already records as read and kept, and
  §O — whose own note already answers it, written by the session that built the annex.
- **Expired blockers**: four, and three are the *quoted retired wording* inside a correction
  (§11.3.7.2, §11.6.4.3, §11.7.4.4 all say "this row used to say"). The live one is §12.10.2's
  wait on §12.10.3, which is real.
- **Capability reasons**: 21 over the ledger, 69 over `crates/`, and every source hit was a true
  statement about a boundary a crate keeps — no clock, no filesystem, no toolkit, no trust store.
- **Entries claimed unread**: 12, eleven of them the known one-short-key-three-clauses population
  (`/Name`, `/Metadata`, `/ID` against `thumbnail.rs`'s key list). **The twelfth was §12.5.2 and
  two of its entries were live**: `/RC`, read since ADR 0199 thirty-three sessions earlier — the
  *same clause* §12.5.6.2's row was corrected for, and this row was never read against it — and
  `/NM`, which `fragment::annotation_named` resolves Annex O's `comment` against since the round
  before last.
- **Caller sweep**: 209 `pub fn`s in `pdf-model`, 75 named by neither host. The new name is
  `restriction::withheld`, and it is the known "functions `pdf-model` calls itself" population —
  `asserted` calls it, and its own doc comment says why it is separate.
- **Retired claim**, run over the nouns rather than the strings, and it paid twice on one phrase.
  **"Marking device"** is what ADR 0204 retired from `CLAUDE.md` in the three-hundred-and-fifty-
  seventh session, and eighteen sessions later it was still in six places: the ledger's own
  *definition of the `inapplicable` status* and the same sentence in `tools/conformance` twice,
  §8.4's parent row, §11.7.5's parent row, §11.7.5.2 (which contradicted itself four sentences
  later), and `requirements::unmet`'s `SeparationSimulation` arm. **A phrase inside a status's
  definition is the worst place for it**: §10.5 spent three hundred and fifty-seven sessions
  `inapplicable` partly because the word the status was explained with named a device the standard
  does not have.
- **The eighth sweep**, below, which is new and which found the `doc/todo/20` this file had been
  carrying as owed.

**And one of the phrase's six places was a defect rather than a comment.** `content.rs` explained
§8.6.8's list by saying `/TR` and `/TR2` "describe a marking device and are read nowhere here",
thirty lines below the `Transfer::read` that has read both since the three-hundred-and-fifty-eighth
— and the `/ExtGState` reader for them was **not** behind the uncoloured-figure flag the rest of
that list is behind, so a transfer function inside an uncoloured tiling pattern or a `d1` glyph
description decided a colour §8.6.8 reserves for whoever uses the figure. Seventeen sessions, and
the stale comment is why nobody looked. `an_uncoloured_cell_that_sets_a_transfer_function_is_ignored`
fails without the guard, painting black where the clause requires the `scn` blue.

**What that adds to the method**: a comment explaining *why* a list is what it is will be read as
the reason not to check the list. The sweeps hunt claims about capabilities; this was a claim about
**which entries a rule covers**, and the code drifted out from under it while the sentence stayed
plausible.

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

## An eighth sweep, and it is the first that checks a note's *citations*: **does the file it names exist?**

The seven above read what a row *claims*. None of them reads what a row *points at*, and a
pointer decays the same way a claim does — faster, because deleting a file is a thing sessions do
on purpose. `tools/conformance` already checks the `code` and `test` arrays; nothing checks the
paths inside a note's prose, or inside a doc comment.

Twenty lines: pull every `doc/todo/NN`, `doc/adr/NNNN`, `crates/….rs` and `examples/…` out of the
ledger's notes and out of every `//` comment in `crates/`, and glob for each one.

**Its first run, in the three-hundred-and-seventy-fifth, produced seven and every one was dead.**

- **§8.9.6.1's `doc/todo/20`**, which this file has carried under "what is still owed" since the
  three-hundred-and-sixtieth session as *a dangling reference whose sentence might still be real*.
  It was not: the sentence said §8.9.6.2 "refuses a stencil painted with a *tiling* pattern", and
  ADR 0169 implemented exactly that in the two-hundred-and-eighteenth session and deleted
  `doc/todo/20` in the same commit — while §8.9.6.2's own row has said so ever since. **The file
  it named was deleted by the session that made the sentence false**, which is what makes this
  sweep cheaper than reading the row: the pointer and the claim died together, and only one of
  them is greppable without knowing anything about the clause.
- **`doc/todo/12`, six times in `crates/`** — `render-quorra/src/lib.rs`, `viewer-ui`'s
  `chrome_ladder` example and its `chrome_over_a_magnified_page` test. The todo was *done* and
  deleted; ADR 0198 is where its argument lives, and the comments now say so.

**What it costs to keep clean, and the one false positive it has.** A corrected note that quotes
its own retired wording reproduces the dead path inside quotation marks, so §8.9.6.1 will hit
every run from now on — the same shape the fourth sweep's oldest false positive has (a note
quoting itself). Read the hit before believing it; one line of context is enough to tell a
citation from a quotation.

**And the shape generalises past files.** A note that cites `crates/foo.rs::some_test` is making
the same kind of claim, and the checker only verifies the ones in the `test` array. A round that
wants a ninth sweep could take the *symbol* halves of every citation in a note's prose.

## A ninth sweep was tried and it is worth knowing it produced noise: **parents *ahead* of their children**

The sixth sweep asks which parents are behind their children. The inverse looks like it should be
stronger — a row saying `implemented` while one of its own direct children still owes something is
claiming every normative requirement in the clause is executed while the ledger itself says one is
not. **It produced 25 hits and none was wrong**, because this ledger's convention is that a parent
row covers the clause's *own* prose and its children own theirs: §7.4's framing is implemented
while five filters are `partial`, and that is a true pair of statements rather than a contradiction.

Run it once to know that; do not run it every round. The one hit worth reading was §O.2, and its
answer was already written into §O's note by the session that built the annex.

## The fifth sweep run again in the three-hundred-and-eighty-sixth, and it paid on the round's own work

**231 `pub fn`s in `pdf-model`, 84 named by no host** — up from 198 and 71, over four host crates now
rather than two, `viewer-confined` having joined `viewer-core`, `viewer-ui` and
`viewer-accessibility`. The populations are the three this file already knows. Two hits are worth
recording and only one of them is closed.

- **`Attachment::checksum_matches`, unreachable from the boundary the same round built.** §7.11.4's
  stream deliberately does not cross with the attachment *list* — a panel drawing five rows would
  otherwise pull five payloads across a pipe — so a confined host holds Table 45's `/CheckSum` in one
  message and the decoded bytes in another, and the clause's rule about the two of them together had
  no way to be asked. `pdf_model::attachment::checksum_matches` is a free function now; the method
  calls it, `viewer_confined::Attachment` calls it, and `tests/confined.rs` asks it end to end.
  **This is the sweep's shape at one round's remove**: nothing was unread when the round began, and
  the round's own transport made it unreachable. A sweep run in the same session as the work is the
  cheapest it will ever be, which this file already says about the fourth.
- **`Collection::initial_document`, and no host can call it at all.** It answers §12.3.5.1's `/D`
  fallbacks — the container, a named embedded file, the first file, or "an empty preview window" —
  and needs the `&Document` that only `viewer-core` holds. Not a confinement gap: `viewer_ui::chrome`
  draws the collection and cannot ask either. Written into §12.3.5.1's ledger row and into
  `doc/todo/34`; closing it is a field on `Answer::Collection` and a consumer for it.

**And one false positive worth naming, because it is new**: `Collection::all_folders` came back as
unnamed and is called by `examples/confined_panels`. The sweep greps `crates/*/src` and an example is
neither — so a function whose only caller is a *demonstration* reads as unread. That is the right
default (an example is not a host) and it is worth knowing the sweep cannot see one.

## All eight run again in the three-hundred-and-eighty-seventh, after eleven rounds with no sweep

The longest gap this file has had. `viewer-accessibility` (376), a signature's DER and CMS (377),
a font-metric band and a selection gate (378), §11.5.3's device branch and its residues (380, 383),
a confined interpreter and rasteriser (381, 386), `--backend` and `--cpu` (384) had all landed
since the last run. Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as
  read and kept. Clean.
- **Expired blockers**: 7 over the ledger and 25 over `crates/`. Three of the ledger's are the
  quoted retired wording inside a correction; §12.10.2's wait on §12.10.3 and §12.5.6.22's on
  printing are real. **Two source hits were live**, and both are below.
- **Entries claimed unread**: 14, twelve of them the known one-short-key-three-clauses population.
  **The thirteenth was §12.5.6.6's `/RC`** and it is the round's implementation.
- **Capability reasons**: 33 over the ledger, 108 over `crates/`, and every source hit was a true
  statement about a boundary a crate keeps — no clock, no filesystem, no toolkit, no trust store,
  no printer. §12.8.2.3's "this program has no feature behind such a gate" reads like the sentence
  §12.8.6's row was corrected for in the hundred-and-ninety-first and is not: it is about the
  *granting* half, and both rows say so in the same words on purpose.
- **Retired claim**, run over the nouns eleven rounds gave the tree — `AccessKit`, `DER`, `CMS`,
  `luminosity`, `backend`, `confined`, `variable text`, `/RC`. Clean but for `/RC`, which paid
  for the third time.
- **Caller sweep**: 231 `pub fn`s in `pdf-model`, 84 named by no host — the same numbers and the
  same three populations as the three-hundred-and-eighty-sixth, `Collection::initial_document`
  included and still open (`doc/todo/34`).
- **`inapplicable` (sweep 7)**: 25 of 83 rows name vocabulary the source names, and none was
  wrong. Annex Q's five are worth recording as the strongest kind of `inapplicable` there is —
  each carries the annex's own NOTE saying "this method is not required by this document".
- **Citations (sweep 8)**: clean. Two hits, both §8.9.6.1 quoting the `doc/todo/20` its own
  correction retired, which is this sweep's known false positive.

**And the first sweep's ledger hit is the longest-lived stale claim this file has recorded: 364
sessions.** §12.5.6.19 said "[w]hat is owed is the value: a text field's /V, a check box's or radio
button's state, and a push-button's /CA caption all need §12.7.4.3's variable text, so a widget
holding one draws its frame and reports the rest". It was written in the **twenty-first** session,
when constructing an appearance arrived, and it was false from the **twenty-third**, two commits
later, when `variable_text::lay_out` did. `appearance::field_text` lays out all three.

Three things make this the sweeps' own shape rather than an accident:

- **The row was corrected four times after the sentence went false** — sessions 105, 132, 138 and
  253 all added to it, by appending. Failure shape 6, and this is its record holder.
- **Its `test` array names a test that reads as confirmation and is not.**
  `a_widget_draws_its_background_and_reports_its_field_value` states no `/DA` anywhere, so the
  report it asserts is `Owed::NoFont` — one of the eight cases that genuinely still report — and
  the test's *own doc comment* had drifted into repeating the row's claim. A row and the evidence
  it cites can go stale together, because the same session writes both.
- **No grep in this file finds it from the ledger side.** "need §12.7.4.3's variable text" is the
  first sweep's shape only because §12.7.4.3 is a clause number; the sentence names no capability,
  no retired string and no unread key.

**Two live source hits from the first sweep**, neither of which any ledger row could have shown:

- **`tools/pdfref/src/main.rs`** opened "[o]ur own renderer needs a parser, which does not exist
  yet, so this cannot compare *us* against anything" — true when the tool was written and false
  from the round that opened a document. The division of labour it describes is still real, so
  what replaced it says *why* the tool compares the references with each other rather than
  claiming it cannot do otherwise.
- **`annotations.rs`'s `a_widget_draws_its_background_and_reports_its_field_value`** said "Table
  192's `/BG` is derivable and §12.7.4.3's variable text is not". The second half is the same
  claim §12.5.6.19's row carried, below, and it lived in the *test the row cites as its evidence*.

## A ninth sweep, and it is the first to check that a citation names the **right** table

The eight above read what a row claims, what it points at, and what its vocabulary implies. None
of them reads a *number*. `tools/conformance` checks that a cited table **exists** and prints its
title — a check the eighty-second session added after finding three ISO 32000-1 numbers in the
ledger — and a number that exists and names the wrong table reads exactly like a right one.

Twenty lines: parse every `Table N -Title` heading out of `doc/md/ISO_32000-2_sponsored_EC3.md`
with its first-column keys, then take every `Table NNN`'s `/Key` citation in `ledger.toml` and in
`crates/` and ask whether that key is one of that table's entries.

**Its first run produced 94 suspects, and eighteen were wrong.** Most of the rest are prose that
names a table and then a key belonging to the dictionary the table describes rather than to the
table itself ("Table 227's `/Ff`", where 227 is the flags inside `/Ff` and 226 is the entry) —
read the hit before believing it, as with every sweep here.

Nine of the eighteen are two **blocks**, which is what makes this sweep different from the others: a wrong
table number does not arrive alone, it arrives as a run of consecutive rows written in one sitting
against the older standard.

| row or file | said | ISO 32000-2 |
|---|---|---|
| §12.5.6.17 movie | Table 188, and a `/Aw` | Table **189**; there is no `/Aw` anywhere in the standard |
| §12.5.6.18 screen | Table 189, and a `/P` | Table **190**, whose five entries do not include `/P` |
| §12.5.6.19 widget | Table 192's `/H` | Table **191**'s; 192 is the `/MK` dictionary the rest of the row is about |
| §12.5.6.20 printer's mark | Table 190's `/MN` | Table **398**'s — which §14.11.3's row already said |
| §12.5.6.22 watermark | Table 191's `/FixedPrint` | Table **193**'s, whose value is Table 194 |
| §14.8.5.5 list | Table 381 | Table **382** |
| §14.8.5.7 table | Table 383 | Table **384** |
| §14.8.5.8 artifact | Table 384, with a `/Subtype` | Table **385**, which has two entries; `/Subtype` is Table 363's |
| §14.11.7 OPI | Table 402's `/OPI` | Tables 87 and 93 state the entry; **405** is its value |
| `pdf-font/src/collection.rs` | Table 127 defines `/FontFile2` | Table **124** |
| `pdf-model/tests/font_collections.rs` | the same sentence | Table **124** |
| `pdf-model/tests/oracle.rs` | Table 111 defines `/Widths` | Table **109** — and the quotation beside it is 109's |
| `pdf-model/tests/oracle.rs` | Table 174's `/Border` | Table **166**'s, beside the `/C` the same sentence puts there |
| `pdf-model/src/view.rs` | Table 179's `/Subtype` | Table **182**'s; 179 is the line ending styles |
| `pdf-model/tests/oracle.rs` | Table 145's `/BC` | Table **142**'s; 145 is the group attributes |
| `viewer-core/src/open.rs` | Table 179's `/QuadPoints` | Table **182**'s — the same pair as `view.rs` |
| `viewer-core/src/query.rs` | Table 98's `/Name` | Table **96**'s, which is the clause its own blockquote cites |
| `pdf-model/tests/actions.rs` | Table 197's `/A` | Table **166**'s; 197 is where the `/AA /U` it beats lives |

**§12.5.6.23's own note is why this sweep should have been run two hundred and eighty sessions
ago.** It says, in the row: "[t]he row previously cited "Table 193", which is the watermark
annotation's table and an ISO 32000-1 number; the redaction table is 195." The hundred-and-fifth
session found *one* of these, named the mechanism exactly, corrected its own row, and swept
nothing. Four of its immediate neighbours were carrying the same error, and one of them —
§12.5.6.22 — is the very watermark row whose table number §12.5.6.23 had been given by mistake.

**What it adds to the method**: a sweep is worth building the moment a correction names a
*mechanism* rather than a sentence, and "an ISO 32000-1 number" is a mechanism. The other place
this rule has already paid is the fourth sweep's "run it over the noun, not the string".

**A gate is not the answer here and that is a decision, not a deferral.** 94 suspects and 18
defects is the wrong ratio for a build failure, and tightening the heuristic enough to gate would
mean deciding which of English's ways of saying "the flags in Table 227's `/Ff`" are legitimate —
a checker that has to be right every time, which is the standard `citation.rs` already sets itself
for `another_document`. It stays a sweep, and it is cheap: one run is under a second.

## What is still owed, named

- ~~**§12.8.2.3's `should`**~~ — closed in the hundred-and-ninety-eighth session (ADR 0159).
  Table 258's rights are read, `ViewState::save` rewrites the permissions dictionary without its
  `/UR3` where a save would exceed them, and the condition was *counted* before it was trusted:
  all four corpus documents carrying a `/UR3` grant what this program does, so no file here can
  trip it. What is still owed under §12.8.2.3 is §12.8.2.2.2's comparison of two revisions, which
  needs the digest.
- **~132 `partial` rows** not yet re-read against the code, of 252.
- ~~**§12.5.6.6's `/RC`**~~ — closed in the three-hundred-and-eighty-seventh by the second sweep
  (ADR 0224). It is the **fourth** row to have carried "`/RC` … is XFA rich text, which principle 5
  excludes", after §12.5.6.2's in the three-hundred-and-forty-second and §12.5.2's in the
  three-hundred-and-seventy-fifth — and the first where the sentence hid a *different* `shall`:
  Table 177's `/RC` "shall be used to generate the appearance of the annotation", so a free text
  annotation stating only that entry drew a blank page.
- ~~**§12.3.5.1's `/D` fallback, implemented and reachable from no host**~~ — closed in the
  three-hundred-and-ninety-fourth (ADR 0231), eight rounds after the fifth sweep found it and two
  after this file recorded it as owed. It took what the entry predicted, a field on
  `Answer::Collection` and a consumer, and **a correction recorded in a todo file is still not a
  correction**: it sat named through seven rounds that each had room for it.
- ~~**§12.5.6.19's seven unread Table 192 entries**~~ — **three of the seven closed in the
  four-hundred-and-second** (ADR 0239): `/I`'s form XObject icon, `/IF`'s Table 250 fit whole, and
  three of `/TP`'s seven codes. What is still owed is `/TP`'s codes 2 to 5, which name the side the
  caption goes on and state no proportion for it, and `/RI`, `/IX`, `/RC` and `/AC`, which are
  pointer states a *constructed* appearance has no room for — one stream where §12.5.5 gives a stored
  one three. **The count came first and it decided the shape**: `examples/push_button_census` finds
  42 push-buttons in the corpus, 33 with their own `/AP /N`, so nine can reach the construction at
  all — and the only entry any of the nine states is `/IF`, in a document that states no icon.
- ~~**Annex I.2's version number**~~ — closed in the three-hundred-and-sixty-first session, the
  round after the sweep that found it (ADR 0207). It was worth one line here for exactly one round:
  a `should` nobody had read, two lines from a parser already standing on the number.
- ~~**A dangling `doc/todo/20`**~~ — closed in the three-hundred-and-seventy-fifth by the eighth
  sweep. It was in §8.9.6.**1**'s note rather than §8.9.6.2's, which is part of why nobody found it
  by reading the clause it was about: the refusal had been implemented sixteen sessions before this
  entry was written and a hundred and fifty-seven before it was corrected (ADR 0169), and
  §8.9.6.2's own row had said so all along.
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

## A tenth, and it is not a sweep — it is the gate that already runs, with a hole in it

Found in the three-hundred-and-ninety-first session by writing a comment and watching the gate
refuse it *inconsistently*: `QUORRA_FEEDBACK.md section 13` is the spelling this tree uses and the
draft had written `§13` twice, once with a `doc/` in front of it and once without. Only one was
refused.

`tools/conformance`'s `another_document` decides a `§` belongs to some other document when the word
in front of it is an upper-case stem with a `.md` suffix. `doc/` is not upper case. So **a citation
written with a path passed the arm for the whole of its life** — eight in the tree, six of them
naming `QUORRA_FEEDBACK.md`, which is the document the arm's own comment cites as the case it
exists to catch. All eight were being checked against ISO 32000-2's clauses and passing by landing
on one, which is the exact failure its message describes.

One `rsplit('/')` and a test, plus eight rewrites. The citation count went 5095 → 5133, of which
**minus eight** is this correction.

**What it adds to the eight above is a target rather than a technique**: they read the ledger's
prose and the tree's comments, and this one read a *checker*. A predicate about how a string is
spelled is a test of how the author spelled it, and the way to find the next one is to write the
thing the gate is meant to catch and check that it *is* caught — in every spelling a person would
plausibly use.

## All nine run again in the three-hundred-and-ninety-fourth, after seven rounds with no sweep

`Query::Offset` and `Query::FieldSelection` (388), a sub-pixel rule drawn as the pixel line it lies
in (389), `--trace`'s stages and clock (390), `Image::area_averaged` at 7.6× (391), a signature
verified under the signer's key (392) and §12.4.4's transitions drawn (393) had all landed since the
last run. Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as read
  and kept. Clean, for the fourth run running.
- **Expired blockers**: 13 over the ledger and 9 over `crates/`. Four of the ledger's are the quoted
  retired wording inside a correction (§11.3.7.2, §11.6.4.3, §11.7.4.4, §12.5.6.19); §12.10.2's wait
  on §12.10.3 and §12.5.6.22's on printing are real. `pdf-syntax/src/tree.rs`'s "four families …
  were blocked on one small piece of clause 7" reads as a hit and is in the past tense, which is a
  false-positive shape worth naming: **a sweep for a blocker cannot see a tense.**
- **Entries claimed unread**: 24, and every one is the known one-short-key-three-clauses population
  or a list whose entries were checked in the two-hundred-and-ninth, three-hundred-and-thirty-second
  and three-hundred-and-eighty-seventh runs. §12.5.6.19's "[a]ll seven are read by nothing" was
  re-checked against the tree rather than believed — `/I`, `/RI`, `/IX`, `/IF`, `/AC`, `/RC` and
  `/TP` are read nowhere, and the two hits a grep finds for `RI` and `IF` are §8.6.5.8's rendering
  intent and §12.7.8's FDF field. **A true row is a result**; it stays `partial` and stays named.
- **Capability reasons**: 41 over the ledger, 112 over `crates/`, and every source hit was a true
  statement about a boundary a crate keeps. `navigation.rs`'s "this crate has no clock to run one
  with" survives ADR 0230 by construction, because the round that drew a transition put the clock in
  `viewer-core` and said so in the same module comment.
- **Caller sweep**: 242 `pub fn`s in `pdf-model`, 87 named by no host — up from 231 and 84, the
  growth being session 392's DER, CMS and X.509 readers, which are the known "functions `pdf-model`
  calls itself" population. **`Collection::initial_document` is off the list**, which is this
  round's work.
- **`inapplicable` (sweep 7)**: 64 of 83 rows name vocabulary the source names, on a looser
  stop-list than the three-hundred-and-eighty-seventh's 25 of 83. None was wrong.
- **Citations (sweep 8)**: clean. Two hits, both §8.9.6.1 quoting the `doc/todo/20` its own
  correction retired, which is this sweep's known false positive. **The first run of it had a
  parser bug worth recording**: `examples/foo` lives under `crates/<crate>/examples/`, so a glob
  anchored at the repository root reported 32 live paths as dead. An instrument that says a
  citation is broken has to be right about where a file lives.
- **Retired claim**, run over the nouns seven rounds gave the tree — `selection`, `verif`,
  `trust store`, `transition`, `presentation mode`, `no clock`, `sub-pixel`, `trace`. **It paid
  twice, and both were parent rows.**

### The two the fourth sweep found, and they are the fifth failure shape at family scale

**§12.1 is clause 12's own map, and it said "§12.8's signatures read and never verified"** — retired
by ADR 0229 one round earlier, in nine rows of §12.8 that all say the opposite. The map row is
written once and amended by nobody, because the sessions that implement a member do not cite it.

**§12.6.4 said "three are performed and they are the three that change what is displayed"** — and
its own eighteen children say eight. `/GoToE` (§12.6.4.4), `/GoToDp` (§12.6.4.5), `/Thread`
(§12.6.4.7) and `/Named` (§12.6.4.12) had each been implemented by a different session, and
`/Trans` (§12.6.4.15) by the round before this one; §12.6's row repeated the same three one clause
up. **The sixth sweep cannot see this**: it asks whether every child is *settled*, and four of
§12.6.4's are `reported` or `out-of-scope` for good reasons, so the family never qualifies. What
finds it is counting the children — and the last sentence of §12.6.4's note was the fourth sweep's
own subject, a claim ADR 0230 had retired in the row next door: "`/Trans` is the fourth that could
change a mark and does not".

**What that adds to the method**: a parent row that states a *number* about its children is
checkable arithmetic, and no sweep here did that. The sixth sweep compares statuses; this compares
a count in the prose with the rows below it. Worth a tenth sweep the next time a family's parent
says "three of the twenty".

### And the ninth sweep paid on its second run, once on the round before's work

- **`pdf-model/src/navigation.rs`** opened "[`transition`] is Table 164's `/Trans`" — written in the
  three-hundred-and-ninety-third, and `/Trans` is **Table 31**'s, a page object entry. Nine lines
  down the same module comment says "Table 31 lists both as entries of a page object", so the module
  held both answers at once from the day it was written. Table 164 is what the entry's *value* is.
- **§12.6.4.2** cited "Table 206's `/D`" and the go-to action's `/D` is **Table 202**'s; 206 is
  §12.6.4.5's GoToDp dictionary, whose two entries are `/S` and `/Dp`. This one was in the
  three-hundred-and-eighty-seventh's 94 suspects and was not among its eighteen corrections, which
  is what a sweep with a 5:1 noise ratio costs: **the run has to be read to the end.**

80 suspects after both corrections, from 81 before, and the rest are the known prose shape — a
sentence naming a table and then a key belonging to the dictionary that table describes.


## All ten run again in the four-hundred-and-second, and the tenth was built

Eight rounds with no sweep: `doc/HANDOVER.md` restructured into ten files (395), JPEG 2000's
reduced-resolution decode (396), §11.4.6's knockout shape and `/AIS` (397), a check box that could be
checked (398), a shading's clip cropped (399), §11.4.4's non-isolated groups (400), and §12.5.6.6's
free text created and typed into (401). Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as read
  and kept. Clean, for the fifth run running.
- **Expired blockers**: 9 over the ledger and 13 over `crates/`. Four of the ledger's are the quoted
  retired wording inside a correction (§8.6.8, §11.3.7.2, §11.6.4.3, §11.7.4.4); §12.10.2's wait on
  §12.10.3 and §12.5.6.22's on printing are real; the rest are past tense, which **a sweep for a
  blocker cannot see** and which the three-hundred-and-ninety-fourth already named.
- **Entries claimed unread (sweep 2)**: 31, thirty of them the known one-short-key-three-clauses
  population or lists checked in earlier runs — **and the thirty-first is this round's oldest
  finding.** §12.5.6.19's "[a]ll seven are read by nothing" was re-checked rather than believed and
  was true, which is what the round then implemented.
- **Capability reasons (sweep 3)**: 30 over the ledger and 96 over `crates/`, and every source hit
  was a true statement about a boundary a crate keeps — no clock, no filesystem, no toolkit, no
  trust store, no printer. Two of the ledger's are the quoted retired wording inside a correction.
- **Retired claim (sweep 4)**, run over the nouns eight rounds gave the tree — `knockout`, `/AIS`,
  `check box`, `free text`, `/DR`, `non-isolated`, `reduced resolution`. **It paid twice, both on
  `/AIS`, and the pair is this sweep's own subject**: §11.6.4.3's row was corrected in the
  three-hundred-and-ninety-seventh and the two other rows describing the same mechanism were not.
  §8.4.5 still listed `/AIS` among Table 57's *not read* entries with ADR 0027 as the reason — the
  argument ADR 0234 retired — and §11.5.1 still said it was "immaterial and deliberately not read",
  whose conclusion survives and whose second half does not. **And the same row gave a third**:
  §8.4.5's `/TR`/`/TR2` sentence pointed at "§10.5, which is `silent` now", forty-five rounds after
  ADR 0204 made §10.5 `implemented` — a row saying its neighbour is the ledger's *last* silence when
  the ledger has had none since the three-hundred-and-sixty-ninth.
- **Caller sweep (5)**: 246 `pub fn`s in `pdf-model`, 86 named by no host — up from 242 and 87. The
  new names are `measurement.rs`'s four (§12.10.2's real wait on §12.10.3), `named_page.rs`'s
  `disagreements` and `article.rs`'s `page_array_agrees`, all of them the known "functions
  `pdf-model` calls itself, or that only a test reaches" population. `document_part::first_page`
  reads as unnamed and is reached by every `GoToDp`, through `DocumentPartJump::page_in` — a host
  calling the *wrapper* is a shape this sweep cannot see and is worth knowing it cannot.
- **`inapplicable` (sweep 7)**: 30 of 83 rows name vocabulary the source names, none of them wrong.
- **Citations (sweep 8)**: one hit over files, §8.9.6.1 quoting the `doc/todo/20` its own correction
  retired, which is this sweep's known false positive. **But the shape generalises past files, which
  this file has said since the sweep was built, and running it over *sections* paid**: six comments
  in `crates/` cite "`doc/HANDOVER.md`'s section 0", and the three-hundred-and-ninety-fifth moved
  that section whole into `doc/ui-boundary.md`. The file they name still exists, so the file-level
  sweep sees nothing; what a reader following the pointer finds is one row of a table. **A section is
  a citation and it decays faster than a file, because moving one is a thing a session does on
  purpose.**
- **Table numbers (sweep 9)**: 193 suspects, **three defects and all three in the source**. Both
  are the sweep's own subject — a mechanism gets one row per place that mentions it, and correcting
  one leaves the others lying:
  - `annotation.rs`'s `/H` doc comment said "Table 192 gives `/H` the default `I`". `/H` is **Table
    191**'s, the widget annotation's own entry; 192 is the `/MK` dictionary. This is the *exact*
    correction the three-hundred-and-eighty-seventh made to §12.5.6.19's ledger row, and the source
    comment one directory away was not swept with it.
  - `appearance.rs` twice and `tests/annotations.rs` once said §12.5.6.12's stamp names are "Table
    186's list". Table 186 is the **popup** annotation; the rubber stamp's `/Name` is **Table 184**'s
    — and §12.5.6.12's own ledger row has said 184 all along. **The ledger held the right answer and
    three source comments held the wrong one**, which is shape 7 across the ledger/source line rather
    than between two rows.

## A tenth sweep, built in the four-hundred-and-second: **a parent's stated count against its children**

Invented by the three-hundred-and-ninety-fourth and not built. The sixth sweep asks whether every
child of an owing parent is *settled*, which §12.6.4 never qualifies for because four of its
eighteen children are `reported` or `out-of-scope` for good reasons. What that run wanted instead is
arithmetic on the **prose**: a parent row that says "three of the twenty" is making a checkable claim
about the rows below it.

Thirty lines: for every row with direct children, find a number word or digit in the note followed
within a phrase by a verb of implementation, and print it beside the children's actual statuses.

**Its first run produced 16 hits and two were wrong.** Most of the rest are the shape worth naming
before believing any of them: a count about something that is *not* the children — §9.6's "three of
the clause's properties", §12.7.8's "two entries that would add to a document", §7.4's "[f]our of
them are stream filters". A number in a parent row is usually about the clause and not about the
family, and the sweep cannot tell which without reading it.

The two that were wrong:

- **§14.11 said "[t]wo of its seven subclauses reach a screen … and both are implemented"**, and it
  was wrong on both halves. **Three reach a screen**: §14.11.3's printer's marks left the "for a
  press" list in the three-hundred-and-fifty-ninth, when the seventh sweep read the clause's own
  flags sentence — and this row went on naming them among "[t]he rest [that] are for a press" for
  forty-three rounds, while §14.11.3's own row carried the correction and §12.5.6.20's had said the
  code drew them all along. And **neither of the two it named is `implemented`**: §14.11.2 is
  `partial`. The seventh sweep found the *child*; nothing was watching the parent.
- **§12.3 said §12.3.5's collections and §12.3.6's navigators are "both … read as data with nothing
  presenting them"**, false of the first since the three-hundred-and-fifty-second, where
  `viewer_ui::chrome`'s files tab became the presentation §12.3.5's own `shall` asks for (ADR 0202),
  and further false since the three-hundred-and-ninety-fourth (ADR 0231). §12.3.5's own row says so
  in two sentences.

**Both are the fifth failure shape at family scale**, which is what the three-hundred-and-ninety-fourth
predicted this sweep would find, and both were invisible to the sixth: §14.11's children are not all
settled and §12.3's are not either, so the arithmetic that compares *statuses* never looks at them.
What separates the tenth from the sixth is that it reads the sentence rather than the status column.

**The false-positive ratio is 7:1 and it is not a gate**, for the ninth sweep's reason: tightening it
enough to fail a build would mean deciding which of English's ways of counting are about a family.
One run is under a second.

## The fifth sweep run again in the four-hundred-and-eighth, with a **fifth** host crate

The round built `crates/viewer-gtk`, so the sweep's grep population grew from four host crates to
five. **246 `pub fn`s in `pdf-model`, 85 named by no host — and the GTK host names not one that the
other four do not.**

Two things about that number, and the second is the finding.

- **The delta is what to trust, not the level.** The four-hundred-and-fifth recorded 246 and *86*
  with its own script; this round's script says 246 and 85 over the same four crates at `HEAD`, so
  the two extractions differ by one name and neither is wrong about the population. What is exact is
  the difference a fifth host makes, which was computed by running the sweep both ways in one
  sitting: **zero**.
- **A native host reaches `pdf-model` for *types* and for no function of its own.** `viewer-gtk`
  names `form::Control`, `TextControl`, `ChoiceControl`, `Choice`, `attachment::Attachment`,
  `outline::Outline` and `outline::Item` — the shapes `viewer-core`'s answers carry — and calls
  nothing in that crate the existing hosts do not already call. That is the boundary working as
  designed rather than a null result: the sweep exists because a capability can reach the crate
  implementing a clause and never reach a program, and a whole new program needing no new entry
  point is the strongest available evidence that the entry points are the answers.

## All ten run again in the four-hundred-and-thirteenth, an **eleventh** built, and the fifth sweep run over **eight** host crates

Ten rounds with no full sweep, and the tree grew four host crates in them: `viewer-gtk` (408),
`viewer-host` and `viewer-qt` (410), `viewer-ffi` (411), beside `Command::Delegate` (409),
`RasterFormat` losing `#[non_exhaustive]` and `Answer::Field` gaining a `ShownValue` (411), and
`Edit::SetField` carrying §12.7.5.4's selection as indices (412). Over `ledger.toml` and over
`crates/`:

- **Expired blockers (sweep 1)**: 6 over the ledger and 41 over `crates/`. Four of the ledger's are
  the quoted retired wording inside a correction (§11.3.7.2, §11.6.4.3, §11.7.4.4, §12.5.3);
  §12.10.2's wait on §12.10.3 is real; §7.7.2's is past tense. **One source hit was live and it is
  below**: `viewer-gtk/src/controls.rs` said this host "cannot" ask for a page without its widget
  appearances, three rounds after it could.
- **Entries claimed unread (sweep 2)**: 19, every one the known one-short-key-three-clauses
  population or a list checked in an earlier run. §8.11.4.3's `/Configs` was re-checked rather than
  believed and is true — the only thing in the tree that names it is `examples/oc_usage_census`, and
  the sweep is right that an example is not a reader.
- **Capability reasons (sweep 3)**: 18 over the ledger and 68 over `crates/`, and every source hit
  was a true statement about a boundary a crate keeps. Two of the ledger's are the quoted retired
  wording inside a correction.
- **Retired claim (sweep 4)**, run over the nouns ten rounds gave the tree — `native host`, `GTK`,
  `Qt`, `C ABI`, `Delegate`, `list box`, `RasterFormat`, `ShownValue`, `cancel`, `memfd`,
  `bare CFF`, `consensus`. Clean over the ledger: §12.7.5.4's row was corrected by the round that
  read Table 234, §12.5.5's and §12.7.4.2's name `Command::Delegate` as the thing that arrived, and
  no row still says a list is drawn by nothing.
- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as read
  and kept. Clean, for the sixth run running.
- **`inapplicable` (sweep 7)**: 72 of 83 rows name vocabulary the source names, on a looser
  stop-list than earlier runs, and none was wrong. §14.8.5.5, §14.8.5.7 and §14.8.5.8 were re-read
  because their rare words (`PrintField`, `Decimal`, `Pagination`) come back every run, and all
  three carry the three-hundred-and-eighty-seventh's table-number corrections intact.
- **Citations (sweep 8)**: **it paid, and on a whole block.** `doc/todo/37` was deleted by the
  four-hundred-and-ninth session — the round that closed its last item — and **seven citations to it
  survived**: §12.7's ledger row and six comments in `viewer-confined`, `viewer-gtk` and
  `viewer-host`. This is the first run of this sweep to find a *live* claim behind a dead pointer
  rather than a dead claim: `controls.rs`'s said the GTK host "cannot" ask for a page drawn without
  its widget appearances, and `Command::Delegate` is what `Host::open` has sent since the round that
  deleted the file. **The pointer and the claim died in the same commit and only the pointer is
  greppable**, which is what this sweep is for. §8.9.6.1's two hits are the known false positive.
  One thing the run is worth recording for: **the sweep's own file globbing has to know where a file
  lives** — an `examples/foo` under `crates/<crate>/examples/` read as dead until the glob was
  fixed, which the three-hundred-and-ninety-fourth already recorded and which cost a second run
  here.
- **Table numbers (sweep 9)**: 73 suspects after the parser was taught that two of the standard's
  446 table headings carry a Markdown `##` — **and five defects, all five in the source, and two of
  them inside a function whose own doc comment had been corrected for the same thing one round
  before.**

### The five wrong numbers, and the first three are one entry

`/H` is **Table 191**'s, on the widget annotation, and **Table 176**'s, on the link. Table 192 is the
`/MK` appearance characteristics dictionary and states no `/H` at all. §12.5.6.19's ledger row was
corrected in the three-hundred-and-eighty-seventh; `annotation.rs`'s `highlight` doc comment in the
four-hundred-and-second; **and three more places in the same file were left**: the `Highlight`
enum's own doc comment above it, and *twice inside `highlight`'s body* — the comment naming the two
tables that define the entry, and the comment on the `_ =>` arm that takes the default. Five places,
one entry, three rounds. **The round that corrects a comment does not read the function under it.**

- `pdf-model/src/view.rs` — `mark_up`'s doc comment put `/QuadPoints` in **Table 179**, the line
  ending styles, and then in **Table 166**, which states it for no annotation. It is **Table 182**'s,
  the text markup annotations'. The three-hundred-and-eighty-seventh corrected this file's *other*
  `/QuadPoints` sentence and `viewer-core/src/open.rs`'s, and left these two.
- `pdf-font/src/lib.rs` — "§9.6.2's Table 109 names `/MissingWidth`". Table 109 is the Type 1 font
  dictionary and has no such entry; `/MissingWidth` is **Table 120**'s, on the font descriptor —
  which is what the three lines of code under the comment read it off.
- `pdf-model/src/requirements.rs` — "Table 43's `/Schema` and each file's Table 44 collection item
  dictionary", **two wrong numbers in one sentence**: `/Schema` is Table **153**'s, on the collection
  dictionary, and the collection item dictionary is Table **46**. 43 is the file specification and 44
  is the additional entries in an embedded file stream.
- `pdf-model/tests/actions.rs` — "Table 166's `/A` beats Table 197's `/AA /U`", **and 166 was itself
  a correction**: the three-hundred-and-eighty-seventh changed it from 197 to 166, and Table 166's
  nineteen entries do not include `/A`. There is no `/A` common to all annotations; the test's own
  annotation is a `/Link`, so it is **Table 176**'s. *A wrong number replaced by another wrong
  number* is what a 5:1 noise ratio costs when a run is read to the end without the table beside it,
  and it is the first time this file has recorded one.

### The tenth sweep paid again, and its finding is the second-longest-lived this file has

**§12.7.6 and §12.7.6.1 both said "the other two are refused by name" — reset performed, submission
and import refused — and the import has been *performed* since the hundred-and-thirty-second
session.** The sentence was written in the ninety-seventh, so it stood for **280 sessions**, behind
only §12.5.6.19's 364.

Three rows held the right answer the whole time and none of them is the two above: §12.7.6.4's own
row opens "[r]ead and performed" and names `Request::Import`, `Event::NeedsFile` and
`ViewState::import`; §12.6's row has counted import-data among the ten of Table 201's twenty types
performed since the three-hundred-and-ninety-fourth; and §12.7.6.4's status is `partial` rather than
the `reported` its parent claims for it. **The sixth sweep cannot see this** — §12.7.6.2 is
`reported` for a good reason, so the family never qualifies — and the fourth cannot either, because
no round ever *retired* the sentence anywhere. What finds it is reading a parent's prose against the
rows below it, which is the tenth sweep's whole subject and its second pair of hits in two runs.

### The fifth sweep over **eight** host crates, and the delta is again zero

`viewer-core`, `viewer-ui`, `viewer-accessibility`, `viewer-confined`, plus `viewer-gtk`,
`viewer-qt`, `viewer-ffi` and `viewer-host`. **327 `pub fn`s in `pdf-model` (249 distinct names), 86
named by none of the original four and 85 named by none of the eight** — so **four whole new host
programs, one of them in C, take exactly one name off the list**: `ViewState::widget_appearances`,
which is session 409's own work. The four-hundred-and-eighth measured the same delta at zero for one
new host; four of them make it one.

**And the sweep was run a second way, over `viewer-core`'s own vocabulary rather than over
`pdf-model`'s functions**, because "who calls it" has a second layer now that a host is not
`viewer-ui`: every variant of `Command`, `Query`, `Answer`, `Event` and `Edit`, against each of the
six crates that speak it. `Event` is unanimous — all fifteen named by all four programs — and the
finding is in `Query`:

- **`Query::Find` and `Query::LogicalSelection` are named by no program at all.** The only things in
  the tree that name either are `viewer-core`'s own headless test and `viewer-confined`'s transport,
  which is a pipe rather than a host. So this viewer has a text search implemented, tested and
  reachable, and **nothing a person can press**; and §14.8.2.5's logical content order, whose ledger
  row reads `implemented` on the strength of the query, had no consumer.
- **The second of the two is closed in this round**: `viewer-ui` copies the page selection on `c`,
  asking `Query::LogicalSelection` first and saying which of the two orders it got.
  `Query::Find` is left open and is named here so that the next round does not have to find it
  again — a find bar is a feature and not a sweep's business, and **a correction recorded in a todo
  file is not a correction**, so it is written down as owed rather than as done.

### An eleventh sweep, and it is the first to read the ledger's *quotation marks*

The four-hundred-and-twelfth found a note quoting Table 227 bit 1 in single quotes with wording the
standard does not use, and observed that `tools/conformance` verifies every rustdoc blockquote in
`crates/` — 567 of them — and nothing whatever in `ledger.toml`. ADR 0249 is the decision and the
numbers; the sweep is thirty lines and it is **the discriminator rather than the match** that makes
it usable:

**977** double-quoted spans of four words or more in the ledger's notes; **560** occur verbatim in
some document under `doc/md/`; **417** occur in none. A gate cannot be built on 417, because almost
all of them quote something that is not the standard and the ledger has no syntax that says so — a
row's own retired wording, `CLAUDE.md`, a report this program prints, another implementation. So the
sweep reports only the misses that **match the standard for at least five words and at least half
the quotation, and then diverge**: 12 of them, and **6 were defects**.

- **"an array of character codes and glyph names"**, in §9.6.5, §9.6.5.1 and §12.7.4.3. Table 112's
  own word is **character** names, and in a font clause the two are not interchangeable.
- **§8.4.4 quoted §10.7.2 as "a PDF processor may ignore this parameter"** — a sentence ISO 32000-2
  does not contain — while §10.7.2's own row carries the real one. Two rows, one permission, and the
  seventh failure shape inside a quotation.
- **§8.3.2.4** dropped "(initial)" out of the middle of a quotation; **§7.9.3** elided a
  cross-reference without an ellipsis.

The other six suspects are all the same false positive and it is about the instrument: the Markdown
conversion of the PDF breaks words across lines — `text-tospeech`, `hierarch y`, `T h`,
`implementationdependent` — so a quotation that is exactly right cannot be found. `quote::normalise`
does not repair those either, which is why two blockquotes written *in this round* failed the gate
until they were shortened.

**A twelfth sweep since the four-hundred-and-eighteenth, and it lives in `tools/spec-errata`
rather than here**: the same 977 spans against *Errata Collection 3* instead of against `doc/md/`.
It needs none of the syntax ADR 0249 priced, because the erratum supplies the other side of the
comparison — a span matching a sentence an erratum struck out is the standard's by construction,
whatever else the ledger quotes. First run: **21 ledger landings, 4 stale quotations in 3 rows**
(§7.6.3, §7.8.3 twice, §14.6), the rest being corrections quoting the wording they retired, which is
this file's own fourth-sweep shape. It also found a **third population of quotation**, which neither
this file nor ADR 0249 had counted: a pair of quotation marks inside ordinary rustdoc *prose*, which
`CLAUDE.md` binds exactly as hard as a blockquote and which the gate's `> ` scanner walks past —
39 landings, **6 stale quotations**, the worst of the three. `spec-errata check` is where
it runs; ADR 0254 is why it is not here and not a gate. Its one known gap: `quoted_spans` collects
double quotes only, because an apostrophe would make every possessive an opening mark, and
§12.7.5.2.2's stale quotation was in single quotes.

**The four-hundred-and-nineteenth ran it again and found a fourth and a fifth population**, both by
walking into one of them while reading §7.8.3 for an unrelated clause:

- **A quotation inside an ordinary `//` comment.** The scanner read `///` and `//!` only, and its own
  doc comment gave the reason — "a `\"` in a `//` comment is not making `CLAUDE.md`'s claim" — which
  is a claim about `CLAUDE.md` that `CLAUDE.md` contradicts twice over: it asks for the clause "in
  its doc comment, its module comment, **or the comment above the block**", and binds quotation
  marks rather than doc comments. **13 landings, 2 stale quotations** (§7.8.3's struck fourth bullet
  in `content.rs`, §8.9.7's NOTE 3 in `inline_image.rs`).
- **A quotation with an ellipsis in it.** `overlaps` compared a quotation whole, so a quotation of
  *parts* of one sentence could only match a struck passage shorter than itself — blind to exactly
  the shape `CLAUDE.md`'s own `…` convention produces. **8 landings, 4 stale quotations**, two of
  which are the two files the round before recorded itself as having missed.

**One false positive set the rule**, and it is the same lesson the twelfth sweep learned about
noise: asking whether *any* elided segment matched reported a §11.6.5.2 comment against a sentence
about `/BaseFont` on the four words "the same as the". The test is now one segment quoting the
passage whole **or** every segment inside it. ADR 0255.

**A sixth population is named and not counted**: every quotation of the standard in `doc/*.md`, in
`doc/todo/`, in `doc/HANDOVER.md` and in the 255 ADRs. Nothing reads any of it. The reason to expect
something there is the only reason any of these five was swept — each of the first sweeps found
something.

**And the sweep found a defect in the file rather than in a claim**: 17 rows carried **72
double-escaped quotation marks**, `\\\"` in the TOML, which decodes to a literal backslash before
the quote — so 36 quotations rendered with stray backslashes. Fourteen of the seventeen are the §8.4
family, written in one sitting, which is the ninth sweep's block signature applied to punctuation.
Repaired, and checked by round-tripping the file through `cargo run -p conformance --bin ledger`
rather than by reading it.

**What it adds to the method**: when a sweep's raw output is too noisy to act on, the move is not a
tighter grep but a *measure of how close the miss is*. A claim this project invented shares no words
with the standard; a misquotation shares most of them, and the difference is one binary search.

## What is still owed, named

- ~~**`Query::Find` reaches no program.**~~ **Closed in the four-hundred-and-fourteenth**: three
  hosts ask it — `viewer-ui` draws its own find bar, `viewer-gtk` a `GtkSearchBar` and `viewer-qt` a
  `QToolBar` — and the round that reached for it found the clause waiting behind it. Annex O's
  `search` needed a *document*-wide search that `Query::Find` is not, so `Command::Find`,
  `Event::Searched` and `viewer_core::search` arrived with the bar and the fragment parameter came
  off `Parameter::unhonoured`'s list. ADR 0250. **What the sweep got right is worth keeping**: it
  named the gap without fixing it, and the next round did not have to find it again.
- **~118 `partial` rows** not yet re-read against the code, of 252 — 14 more went in the
  four-hundred-and-thirteenth.

### The ninth sweep, run over the two families the four-hundred-and-fourteenth touched

Annex O's five rows and §14.7's fourteen, every `Table NNN` in them checked against the entries ISO
32000-2 actually puts in that table. **Nothing wrong**, which is the first clean run this sweep has
had and is worth recording as a result rather than as a silence: `Table Annex O.3` is the PDF object
identifiers and `Table Annex O.4` the open parameters, as both rows say; Table 354's ten entries
include `/PronunciationLexicon` and `/AF`, which §14.7.2's "unread" list names and which a 9 000-byte
window on the Markdown does *not* reach — the table is split across two header rows in the
conversion, so a check that read only the first block would have reported two false positives. The
instrument's own reach is part of the sweep, and this is the second time the Markdown's shape has
been the thing to watch after the four-hundred-and-thirteenth's broken words.

## All twelve run again in the four-hundred-and-twenty-ninth, after **fifteen** rounds with no sweep

The longest gap this file has had, and several of the fifteen were large: find bars in three hosts
and Annex O's `search` (414), the painted group's space measured (415), **the errata** — `doc/md/`
presenting struck passages as current text, `tools/spec-errata` built, some thirty stale quotations
fixed, a Type 3 glyph's own resources, and `Do`/`gs`/`scn` reporting a resource a file never defined
(416–419) — the readback cache and §9.4.4's vertical writing mode (420), `pdf-retrieve` and a
quadratic `Tree::walk` (421), three corpora submodules and pdfbox's frozen extraction (422–423),
`Document` made `Sync` (424), 1944 web documents and the first crasher (425), §11.4.7's page group
composited and §11.7.2's conversion into it (426–427), and twelve fuzz binaries found not to contain
the interpreter (428). Over `ledger.toml`, over `crates/`, and — for the first time, because
`SOURCE_ROOTS` reaches them — over `tools/` and `fuzz/`:

- **Expired blockers (sweep 1)**: 4 over the ledger and 18 over the source roots. Three of the
  ledger's are the quoted retired wording inside a correction (§11.3.7.2, §11.6.4.3, §11.7.4.4) and
  §12.10.2's wait on §12.10.3 is real. Every source hit is past tense or true; the one worth
  *checking* rather than reading is `pdf-sandbox/src/decode.rs`'s wait for `close2/hayro`'s
  `feat/reduced-resolution-allocates-less`, and `Cargo.toml` pins `2a1abd14` where that branch's
  commit is `1dc833f7`, so the blocker holds. **A blocker naming a revision is checkable and a
  blocker naming a clause is not** — worth knowing which kind a hit is before reading it.
- **Entries claimed unread (sweep 2)**: 19 over the ledger and 25 over the source, every one the
  known one-short-key-three-clauses population. Two were re-checked rather than believed:
  §12.5.6.7's `/Measure`, where the only reader in the tree is `measurement.rs` on Table 265's
  *viewport*, and §8.11.4.3's `/Configs`, unchanged since the four-hundred-and-thirteenth.
- **Capability reasons (sweep 3)**: 16 over the ledger and 100 over the source, every source hit a
  true statement about a boundary a crate keeps. §12.5.6.2's `/Subj` was re-checked against
  `viewer-confined`'s `subject` field, which is Table 349's *document information* and not Table
  172's — the one-short-key shape wearing a struct field.
- **Retired claim (sweep 4), run over the nouns fifteen rounds gave the tree** — `find bar`,
  `search`, `writing mode`, `errata`, `readback`, `retrieval`, `SafeDocs`, `pdfbox`, `page group`,
  `ink cube`, `fuzz`, `Tree::walk`, `composite font`, `Type 3`. **It paid four times and all four
  are one mechanism**, which is this sweep's own subject at its largest yet. See below.
- **Caller sweep (5)**, over eight host crates **and over `tools/` and `fuzz/`**: 335 `pub fn`s in
  `pdf-model` (257 distinct names), **111 named by none of the eight hosts, 89 named by no host, no
  tool and no fuzz target**. So **22 names come off the list because a tool or a fuzzer reaches
  them** — `logical_text`, which this file listed as unread from the two-hundred-and-fifty-third run
  onward and which `tools/pdf-retrieve` has asked since the four-hundred-and-twenty-first; `verify`,
  `signed_attributes_encoding` and `timestamp_imprint`; `spans_under` and `text_under`; `scan`,
  `section` and `sections`. **A tool is a consumer and this sweep could not see one for 176
  sessions.** The finding is below.
- **Arithmetic (sweep 6)**: three hits — §7.9.2 and §O, which this file records as read and kept,
  **and §9.4, which is new**: `partial` above four `implemented` children. Its reason had been
  retired by its own child nine rounds earlier.
- **`inapplicable` (sweep 7)**: 71 of 83 rows name vocabulary the source names, on the same loose
  stop-list as the four-hundred-and-thirteenth's 72, and none was wrong.
- **Citations (sweep 8)**: 4 hits, **0 defects and a new false-positive shape**. §8.9.6.1's
  `doc/todo/20` and §12.7's and `viewer-gtk/src/controls.rs`'s `doc/todo/37` are the known shape, a
  correction quoting the pointer it retired. The fourth is
  `tools/spec-errata/src/main.rs`'s `doc/errata.md`, and it is **a redirection target rather than a
  citation** — the file is what `emit >` writes and `.gitignore` names it for the same licence
  reason as `doc/md/`. A sweep that reads paths cannot see a `>` in front of one.
- **Table numbers (sweep 9)**: 412 headings parsed, 68 suspects, **3 defects — and two of them are
  in `tools/`, which this sweep had never reached.** Below.
- **Parent's stated count (sweep 10)**: 185 counted claims raw, 16 after the count is compared with
  the family's size and its settled total, **1 defect**: §11.7 states its count of the family twice
  and the two disagree. Below.
- **Ledger quotation marks (sweep 11)**: **1028** double-quoted spans of four words or more, 516
  verbatim in some document under `doc/md/` and 512 in none; 32 that match the standard for five
  words and then diverge, **3 of them defects**. Below. The other 29 are the two known shapes — an
  `…` elision, which `CLAUDE.md`'s own convention produces, and the conversion's broken words
  (`text-tospeech`, `hierarch y`, `T h`, `None ;`).
- **The errata (sweep 12)**, `cargo run --release -p spec-errata -- check doc/*.pdf`: **151 lines
  for 120 distinct struck passages, unchanged since the four-hundred-and-eighteenth finished reading
  them, so nothing remains unread and the number is not moving.** 28 in-clause landings, every one
  annotated in place **except one**, and a second in the "elsewhere" bucket that is the same
  sentence. Below.

### The four the fourth sweep found, and they are one mechanism in four rows

**Writing mode 1 and the predefined `CMap`s.** §9.4.4 was moved to `implemented` in the
four-hundred-and-twentieth after being `partial` for three hundred and eighty-four sessions on a
sentence that was false from the thirty-sixth; §9.7.5.2's data shipped in the hundred-and-fifty-sixth
(ADR 0140). Four rows went on describing a tree that had neither:

- **§9.4** — "§9.4.4 stays partial because the vertical branch of the displacement formula has no
  writing mode 1 to reach it", and the row's own `partial` rested on that clause alone. Now
  `implemented`; the clause states nothing above its four subclauses and all four are settled.
- **§9.7** — *both* halves of its `partial` reason were retired: "Table 116's seventy-odd predefined
  CMaps, refused by name on 15 corpus fonts" and "vertical writing, which is §9.2.4's missing /W2
  metrics rather than this clause's". §9.2.4's own row opens "[b]oth writing modes, from the
  thirty-sixth session", two clauses above.
- **§9.7.5** — "the other predefined names are refused because their data is not in the tree", the
  third row in this family to carry the sentence §9.7.6's row was corrected for in the
  two-hundred-and-ninety-eighth. **No earlier sweep could reach it**: the arithmetic that finds a
  `partial` parent above settled children never looks here, because §9.7.5.4 is `partial` for a
  reason of its own.
- **§9.3.7** — "which is trivially true here because only mode 0 exists". Right conclusion, expired
  argument, in an `implemented` row, which is the population sweeps 1, 2, 3 and 6 all skip.

**And one status the sweeps do not look for at all.** §9.7.5.1 was `partial` above a note that named
nothing owed — a row breaking the ledger's own definition of the status, which says "the note says
which are not". Every requirement §9.7.5.1 states is executed and each was already named in the note.
`implemented`, and the expiry is the same one: the predefined data landing in the hundred-and-fifty-sixth.

**What that adds to the method**: the sixth sweep asks whether a `partial` row's *children* are
settled. Nothing asks whether a `partial` row's *note* names anything owed at all — and that is
arithmetic on one row rather than on a family, cheaper than any sweep here, and would have found
§9.7.5.1 without knowing a thing about fonts. Worth a fourteenth the next time a round has room.

**And the status move cost §9.4 its evidence, which is the gate working.** The row named two whole
*test files*, tolerated on a `partial` row and refused at zero by `FILE_ONLY_EVIDENCE_CEILING` on an
`implemented` one, so `cargo test -p conformance` failed the moment the status changed. One named
test per subclause now. **A ratchet that only bites on a status change is a ratchet that bites when
a claim gets stronger**, which is when evidence matters most.

### The fifth sweep over `tools/` and `fuzz/`, and the one `pub fn` nothing names

`SOURCE_ROOTS` has been `["crates", "tools", "fuzz"]` since the four-hundred-and-twenty-eighth and
this sweep had never used it. Running it three ways in one sitting is what makes the number mean
something: 111 unnamed over the hosts, **89** once the six tools and the fourteen fuzz targets are
added, so a tool or a fuzzer is the only consumer of 22 names.

**`ViewState::clear_field` is named by nothing in the tree** — no host, no tool, no fuzz target, no
example and **no test**. It is the operation an undo needs, and its doc comment states the design
that separates it from `Edit::SetField` with an empty value: "the old value may have been the file's
own, and re-stating that as an edit would make every later save carry a change nobody made". That
distinction is invisible until a document is *written*, which is why the absence of a test is the
part worth acting on rather than the absence of a host.
`saving.rs::forgetting_an_edit_restores_the_documents_own_value_without_logging_one` asserts both
halves and the save after them. A consumer is a feature and is left named rather than built, which
is what this sweep got right about `Query::Find` in the four-hundred-and-thirteenth.

### The three wrong table numbers, and two are in a tool

- **`tools/pdf-retrieve/src/lib.rs`, twice.** `Wanted::subtypes` said an annotation subtype filter
  compares "Table 164's own names" and `Note::subtype` said "Table 164's `/Subtype`". **Table 164 is
  the transition dictionary.** The annotation types are **Table 171**'s and the entry is **Table
  166**'s, "[e]ntries common to all annotation dictionaries" — which the same doc comment cites
  correctly three lines down for `/Contents`, `/Subj` and `/QuadPoints`. Written in one sitting in
  the four-hundred-and-twenty-first, which is this sweep's block signature; the *first* run of it
  over `tools/` found them.
- **`crates/pdf-model/src/signature.rs`** — "Table 259 also records the older `/UR`". Table 259 is
  the FieldMDP transform parameters dictionary; `UR ( Deprecated in PDF 2.0 )` is a value of **Table
  256**'s `/TransformMethod`, and `/UR3` is **Table 263**'s key in the permissions dictionary. The
  comment had the *distinction* right and the table wrong, which is the shape that survives review.

### The tenth sweep's finding: a row that counts its own family twice

**§11.7 said "[t]wo of its five subclauses are satisfied" and, four sentences later, "four of its
five subclauses are satisfied".** The nineteenth session re-decided §11.7.4 and appended the second
count without reading the first — failure shape 6, standing for **410 sessions**, and the longest
this file has recorded after §12.5.6.19's 364 and §12.7.6's 280. Neither number survives contact
with the rows below it, which are one `inapplicable`, one `implemented` and three `partial`, each
naming what it owes.

**And the count was left behind by the two rounds that changed this family most.** The
four-hundred-and-twenty-sixth read §11.7.2 and §11.7.5.3 as what bounds §11.4.7's page group; the
four-hundred-and-twenty-seventh carried out the conversion §11.7.5.3 names and took a web sample's
61 blocked pages to 0. The parent row mentioned neither. **A parent row is not maintained by the
sessions that implement its members** — this file's fifth failure shape, and the tenth sweep is now
the third instrument to have found it at family scale.

### The three misquotations, and one of them is a full stop

- **§7.6.4.1** quoted "There is nothing inherent in PDF encryption that enforces the document
  permissions." The sentence continues "specified in the encryption dictionary", so the quotation
  ends a sentence the standard does not end — an elision *with a full stop inside the quotation
  marks*, which reads as complete where an `…` would have read as cut. Restored whole.
- **§8.4.3.3** quoted "at both ends of open subpaths (and dashes 8.4.3.6)", dropping
  `, "Line dash pattern"` from inside the parenthesis and "when they are stroked" from the end.
- **§12.3.3** quoted Table 152's `/Title` as "the text that shall be displayed on the screen for
  this item" where the standard capitalises. The same row writes `[t]his value`, `[w]hen an item`
  and `[c]licking` four times over, so the convention was there and one quotation missed it.

**And the sweep found a defect in the file rather than in a claim, for the second time**: §7.3.9's
row carried a `\\\"` — a double-escaped quotation mark, which decodes to a literal backslash before
the quote. The four-hundred-and-nineteenth repaired 72 of these in 17 rows and this one survived,
in a row none of the seventeen was next to. Round-tripped through
`cargo run -p conformance --bin ledger` rather than read.

### The errata sweep's one finding, and it is the fourth sweep's shape across the errata line

`check` prints **151 lines for 120 distinct passages**, which is exactly what
`doc/errata-read.md` recorded when the four-hundred-and-eighteenth finished reading them: **nothing
remains unread, and the number has not moved in eleven rounds.** That is the answer to the lead the
four-hundred-and-nineteenth left, and it is worth writing down because a passage count that stops
moving is the only evidence that a population has been read to the end.

The 28 in-clause landings are all annotations sessions 416–419 wrote in place, **except**
`crates/viewer-host/tests/host_mappings.rs:138`, which quoted §7.11.4.1's "shall map name strings to
file specifications" — struck outright by Issue #481 with the two bullets around it. And
`crates/viewer-host/src/panel.rs:144` quotes the *same sentence* and sits in the "elsewhere" bucket
only because it cites §12.3.5 rather than §7.11.4.1, so `Landing::in_clause` files it away from its
own finding. **`pdf_model::attachment` was corrected for this exact sentence in the
four-hundred-and-eighteenth and the two copies of it one crate over were not** — the fourth sweep's
subject, with an erratum in place of a session's correction. What survives §7.11.4.1 is its NOTE
about pre-PDF-1.6 identification, and that is what both comments rest on now.

## All twelve run again in the four-hundred-and-thirty-seventh, and the **fourteenth was built**

Eight rounds with no sweep, several of them large: a three-component APP14 JPEG asked for four
channels and a `/Length 0` stream reported as missing drawing (430), five pages moved between
contradicted groups and `spec-errata`'s comparison found case-sensitive (431), a diagonal sub-pixel
stroke substituted (432), **65 944 web documents surveyed** with a hang and a crasher fixed (433), a
silent population split and 22 435 lost codes taken to 780 (434), `build_soft_mask` converting a
buffer 99.96% transparent and all 84 budget refusals opened (435), and a document's own press
honoured (436). Over `ledger.toml`, `crates/`, `tools/` and `fuzz/`:

- **Expired blockers (sweep 1)**: 21 over the ledger and 71 over the source roots. Three of the
  ledger's are the quoted retired wording inside a correction; §12.10.2's wait on §12.10.3 and
  §12.5.6.22's on printing are real; the rest are past tense, which **a sweep for a blocker cannot
  see**. Every source hit is past tense or a true statement about a dependency.
- **Entries claimed unread (sweep 2)**: 51 over the ledger, every one the known
  one-short-key-three-clauses population. Five were re-read whole rather than believed — §12.4.3's
  `/N` and `/B`, §12.5.6.7's `/L` and `/Rect`, §12.7.3's `/Fields` and `/SigFlags`, §9.6's
  `/Encoding` and §12.5.6.6's `/BS` — and all five are rows that already say what they mean.
- **Capability reasons (sweep 3)**: 48 over the ledger and 124 over the source roots, and every
  source hit was a true statement about a boundary a crate keeps — no clock, no filesystem, no
  toolkit, no trust store, no printer, no network.
- **Retired claim (sweep 4)**, run over the nouns eight rounds gave the tree — `APP14`,
  `three-component`, `/Length 0`, `hairline`, `sub-pixel`, `blank glyph`, `Latin`, `B2A`,
  `demultiplied`, `budget refusal`, `press`, `web sample`. **Clean over both**, which is what a
  sweep says when the rounds that made the corrections made them everywhere.
- **Caller sweep (5)**, over eight host crates and over `tools/` and `fuzz/`: 339 `pub fn`s in
  `pdf-model` (261 distinct names), 93 named by none of the eight hosts, **74 named by no host, no
  tool and no fuzz target** — the three known populations. `ViewState::clear_field` is still on the
  list and this round's answer to it is below.
- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file records as read and
  kept. Clean, for the eighth run running.
- **`inapplicable` (sweep 7)**: 71 of 82 rows name vocabulary the source names, on the same loose
  stop-list as the four-hundred-and-twenty-ninth's, and none was wrong.
- **Citations (sweep 8)**: 4 hits, 0 defects — §8.9.6.1's `doc/todo/20`, §12.7's and
  `viewer-gtk/src/controls.rs`'s `doc/todo/37`, all three corrections quoting the pointer they
  retired, and `spec-errata`'s `doc/errata.md`, which is a redirection target rather than a citation.
- **Table numbers (sweep 9)**: 409 headings parsed, 82 suspects, **one defect**, and it is below.
- **Parent's stated count (sweep 10)**: 124 counted claims raw, 88 after the comparison. **Two
  defects and a convention**, below.
- **Ledger quotation marks (sweep 11)**: **1058** double-quoted spans of four words or more, 570
  verbatim in some document under `doc/md/` and 488 in none; 16 that match the standard for five
  words and then diverge, **0 of them defects** — every one is a known shape (an `…` elision, a
  Markdown `**` or `[Table N]` inside the span, the conversion's broken words, or a correction
  quoting its own retired wording). **And the sweep has a floor this round walked into from the
  other side**; see below.
- **The errata (sweep 12)**, `cargo run --release -p spec-errata -- check doc/*.pdf`: **151 lines
  for 120 distinct struck passages, unchanged for eight rounds**, and 27 in-clause landings, every
  one an annotation written in place. The population is still read to the end.

### The fourteenth sweep, built, and what it printed on its first run

Twenty lines over `ledger.toml` alone: every `partial` row whose note contains none of the words a
note owing something uses. **16 rows, and seven were defects — two of them statuses.**

| row | was | is |
|---|---|---|
| **§9.3** | `partial` since the thirteenth session | `implemented` |
| **§11.3.7.1** | `partial` since the fifteenth | `implemented` |
| §12.8.4.2, §8.11.1, §8.11.4, §8.11.4.1, §10.4.2 | notes naming nothing owed | notes naming what the family owes |

The other nine are the sweep's own noise and it is one shape: a note that names something owed in
words the sweep's vocabulary does not hold — "writer-side", "counted, not parsed", "which needs the
validator", "what is absent is the distinction". Read the hit before believing it, as with every
sweep here.

**§9.3 is the longest-lived stale claim this file has recorded: 365 sessions**, past §12.5.6.19's
364 and §12.7.6's 280. The row said "[e]ight of Table 102's nine text state parameters are
implemented … the ninth, text knockout, is §9.3.8, which was this tree's third `silent` row until
the fourteenth session made it report" — a sentence written in the thirteenth session and false from
the **seventy-second**, when §9.3.8's `/TK` was *drawn* rather than reported, which §9.3.8's own row
has said ever since.

Three things make it the sweeps' own shape rather than an accident, and the third is why a fourteenth
sweep was worth building:

- **The row's reason is a report, and a report that has stopped being one still reads as a reason.**
  Sweeps 1, 3 and 4 all read a row's reason: this one names no blocker, no capability and no retired
  string, only a neighbouring clause and a status that had moved.
- **The sixth sweep's arithmetic never looks here.** §9.3's children are not all settled, because
  §9.3.1 was `partial` too — and §9.3.1 was `partial` for the *same* expired reason, so the family
  disqualified itself with the very error being looked for.
- **§9.3.1 is a defect the fourteenth sweep could not print either**, and it was found by reading
  §9.3's family off the back of the hit. Its note ended "Partial only because of Tk, whose absence is
  §9.3.8" — which contains the word *partial*, so the sweep's own vocabulary counted it as owing
  something. **A row that names its debt and is wrong about it is invisible to an instrument that
  only asks whether a debt is named.** Both are `implemented` now, with §9.3.1's one untested
  sentence asserted rather than derived: a text state operator "may appear outside text objects, and
  the values they set are retained across text objects in a single content stream", and `q`/`Q`
  carries them.

**And the status move cost §9.3 its evidence, which is the gate working** — it named two whole test
*files*, refused at zero by `FILE_ONLY_EVIDENCE_CEILING` on an `implemented` row. The same ratchet
caught §9.4 in the four-hundred-and-twenty-ninth. Four named tests now, one per requirement group.

### The tenth sweep's finding, and the clause number the errata had given away

§14.8.4.7 said "See the three below" above **four** child rows, two of them carrying the *same*
title — `§14.8.4.7.3 Ruby and warichu elements` and `§14.8.4.7.4 Ruby and warichu elements`. The
first row's own note explained the duplication and had it backwards: it said the number "appears
twice in `doc/md/` … the citation checker finds the first, which is this one", reading a
**renumbering** as an artefact of the conversion.

**Errata Collection 3 Issue #133 inserts a new subclause at §14.8.4.7.3 — link elements — and
renumbers the ruby one to §14.8.4.7.4**, saying so in its own editor's note. So the ledger held
§14.8.4.7.3's content under a number that had moved, and the clause the number now names, with two
`shall`s in it, had never been read. **One row of the family already knew**: §14.8.4.7.2's note,
written in the four-hundred-and-eighteenth, cites "§14.8.4.7.3's link element" correctly. Two rows
about one mechanism, disagreeing — the seventh failure shape, one row apart. ADR 0273; `ClauseIndex::title`
takes a number's *last* heading now, which moves exactly one title of the standard's 1017 headings,
counted before the change rather than after.

**The same lag was in the source, in the place the fourth sweep says to look.** `structure.rs`
documented `Annot` and `Form` as "an association between content and …" — Table 368's wording before
Issue #437 replaced it with *encloses* — and §14.8.4.7.2's ledger row was corrected for that exact
word in the four-hundred-and-eighteenth while the two doc comments one directory away were not.

**And one thing the tenth sweep taught about itself.** Seven parent rows state a count of "the N
below" that is neither the number of direct children nor the number of descendants, and **five of
them are a convention rather than a defect**: the count excludes the family's own `General` row
(§14.10.3, §14.10.4, §14.8.4.7, §7.6.4.4, §14.8.2 all read correctly that way). The two that match
nothing were corrected — §7.6.4's "twelve rows below" against twenty, §9.7's "seventeen" against
sixteen — and §7.6.4.4's sentence gave three counts of its own family in one clause of prose, of
which none survived the rows below it.

### The eleventh sweep has a floor, and a quotation of ISO 32000-**1** falls through it

The old §14.8.4.7.3 row quoted `RP` as punctuation "used only when a PDF processor cannot place the
ruby annotation text adjacent to the ruby base text". **ISO 32000-2 does not contain that sentence**;
its Table 369 says "used only when a ruby annotation cannot be properly formatted in a ruby style and
instead is formatted as a normal comment, or when it is formatted as a warichu".

The eleventh sweep did not report it, and the reason is the discriminator ADR 0249 priced: a span is
printed only where it matches the standard for **at least five words and at least half the
quotation** before diverging, which is what separates a misquotation from a claim this project
invented. A quotation of the *older* standard is neither — four words in common and then a different
sentence — so it sits among the 488 spans that occur in no document under `doc/md/` and are never
looked at. **A ledger row written against ISO 32000-1 is a population that sweep cannot see**, and
the ninth — which finds ISO 32000-1's table *numbers* — is the only instrument here that reaches it.

### The ninth sweep's one defect

**§12.5.6.2 said "Table 172's `/CA` is read, as Table 166's entry of the same name"**, and Table 172
states no `/CA` at all: its nine entries are `/T`, `/Popup`, `/RC`, `/CreationDate`, `/IRT`, `/Subj`,
`/RT`, `/IT` and `/ExData`. `/CA` and `/ca` are both Table **166**'s, common to every annotation.
The entry was the markup annotation's in ISO 32000-1 and moved. The row's second half had the right
answer beside the wrong number, which is this sweep's most durable shape.

### The fifth sweep's open item, answered by reading the caller rather than by building one

`ViewState::clear_field` is named by no host, no tool, no fuzz target and no example, and the
four-hundred-and-twenty-ninth left it named on the grounds that "an undo is a host's feature". **The
undo exists and needs something else.** `viewer_core::Open::replay` makes undo a *replay* rather than
an inverse — the log is cleared with `clear_all_fields` and re-applied up to the cursor — so
forgetting one field is what a replay does by not reaching that entry, and a per-field clear is a
second route to a state a host can already reach. The doc comment said "[t]he operation an undo
needs" and now says what it is. **A `pub fn` nothing names is sometimes a function whose job
something else already does**, and that is a question for the caller side rather than a feature to
build.

## What is still owed, named

- **~80 `partial` rows** not yet re-read against the code, of 248 — 30 more went in the
  four-hundred-and-thirty-seventh, three of which left the population by becoming `implemented`.
- **The fourteenth sweep's own vocabulary.** Nine of its sixteen hits are rows that *do* name a debt
  in words the sweep does not hold, and §9.3.1 shows the inverse: a row naming a debt it is wrong
  about is invisible to it. Widening the vocabulary makes the first number smaller and the second
  problem no better; what the run shows is that the sweep is a *reading list* and not a gate.
- **§14.8.6.3's enclosure requirement** — Errata Collection 3 requires a `math` element under a
  `Formula` structure element and the namespace on every MathML type *and attribute*, and `Tree::role`
  checks neither. `doc/todo/48` carries it and this round walked past it in the same family.
