# ADR 0357 — The two refusals an annex outlived

Status: accepted, 2026-08-14. Session 522.

## Context

ISO 32000-2 Annex O has eleven fragment parameters and, since ADR 0209 built it, this tree carried
out nine of them and **reported the other two by name**. `pdf_model::fragment::Parameter::unhonoured`
held the two sentences, which is where `CLAUDE.md` wants a claim about this tree rather than about
the standard — a sentence that decays the way a ledger row does. `tools/state.sh annex-o` printed
them verbatim:

- `highlight` — "the annex highlights a rectangle rather than selecting one … so it is a shape of
  its own, and **no host has asked for one to draw**"
- `fdf` — "**fetching a URI is the host's, and no host supplies one yet**"

This round asked `doc/habits.md`'s sweep question of each: **has the capability the refusal names
arrived?** For both, the answer is yes, and neither expired the way its sentence predicted.

## What the annex asks, in its own words

Table Annex O.4, §O.2.2:

> Open the document with the specified rectangle highlighted. Each argument shall be an integer or
> floating point value representing the rectangle measured from the top left corner of the page. The
> nature of the highlighting is implementation-dependent.

> Open the document and then import the data from the specified FDF or XFDF file. The URI shall be
> either a relative or absolute URI to an FDF or XFDF file. The fdf parameter should be specified as
> the last parameter to a given URI.

## `highlight`: the refusal's *reading* was right and its *last clause* was not

The reading ADR 0310 recorded stands, and it is the annex's own: a highlight is not the selection.
The two neighbouring parameters that point a person at something both say **selected** — `comment`'s
"with the specified comment selected", `search`'s "selecting the first matching word" — and this one
says **highlighted**, with a nature the other two are not given. Making it the selection would have
been this program inventing a synonym the standard is careful not to use.

What fell is the sentence after it: *no host has asked for one to draw*. ADR 0164's test — do not
add a message before a host asks — has an exception this tree already took in the
four-hundred-and-eighty-first session, and ADR 0316 states it: **a clause may ask**. §12.4.4.2 got
`Command::Present` because the clause conditions a state machine on something no host could deduce;
this is the same shape pointing the other way, and it passes `doc/ui-boundary.md`'s test for a new
message twice over:

- **A host cannot answer it for itself.** The fragment arrives inside `Command::Open` *undecoded*
  and no host ever sees the parameters; the mapping from Annex O's coordinates — default user
  space's units with the page's top-left corner as the origin — into device pixels is the one
  arithmetic ADR 0118 keeps in one place, and a second opinion about it in a host is precisely the
  defect that lived here for seventy-five sessions.
- **It is not a second way to say something a host can already say.** A selection is a range of the
  readback; this is a rectangle a URI named, and the two coincide in nothing.

So `Query::Highlight` answers `Answer::Highlighted(Vec<[f32; 8]>)` — the shapes on the page being
shown, in device pixels, the form `Selected::quads` and `Answer::Found` take. **The annex's own
"implementation-dependent" is this boundary's rule stated by the standard**: what a highlight looks
like belongs to a platform, so the geometry crosses and the colour does not. Three hosts draw it —
`viewer-ui` in a third hue beside its selection blue and its match yellow, `viewer-gtk` in the
theme's foreground at a third alpha because GTK 4.22 exposes no accent colour to application code,
`viewer-qt` in `QPalette::Highlight` at the faintest of its three weights — and the C ABI gained a
hundred-and-twelfth entry point, because commands and questions there are symbols rather than
numbers and `PDFV_EVENT_KIND_COUNT` did not move.

Two things the clause does not decide, decided here and written down as choices:

- **The rectangle belongs to the page the fragment had selected when it stated it.** Every row of
  Table Annex O.4 measures "from the top left corner of the page", and §O.2 makes the parameters run
  left to right — so `page=2&highlight=…` is a rectangle on page 2 and page 1 has nothing to draw.
  It is the dependence the annex spells out for `comment` in a NOTE, applied to the parameter it
  does not spell it out for. A *list* rather than one rectangle, for the same reason: two
  `highlight` parameters in one fragment are two rectangles, possibly on two pages, and the annex
  states no rule against them.
- **Nothing takes it away.** The annex says a document is *opened* with the rectangle highlighted,
  which makes it a property of how this document was opened rather than of what a person is doing;
  no message in this vocabulary dismisses one, and inventing one would be inventing a feature.

## `fdf`: a description of the division, read as a blocker

"Fetching a URI is the host's" is not a blocker — it is the boundary working. Rule 2 of
`doc/ui-boundary.md` is that the core has no filesystem: a document naming a file is a document
asking this machine for something, and whether to give it is not a rendering decision. §12.7.6.4's
import action has crossed on exactly that channel since ADR 0090: `Event::NeedsFile { purpose,
name }` out, `Command::Supply { purpose, bytes }` back.

The second half — *no host supplies one yet* — was true when it was written and stopped being true
without anybody going back to the sentence. All three hosts answer `NeedsFile` today, and
`viewer_host::resolve_import` is the single policy behind them: a plain file name resolved against
the directory the open document is in, everything else refused out loud. So `fdf` needs **no new
message and no network**:

```
#fdf=answers.fdf → Event::NeedsFile { purpose: ImportData, name: "answers.fdf" }
                 → host policy → Command::Supply → §12.7.8's FormsData → the field says the value
```

Four things worth recording:

- **A relative URI is the ordinary case and it is a local file.** The annex says "either a relative
  or absolute URI", and a relative one resolves against the document's own URI — which for an open
  file is the directory it is in, which is what `resolve_import` already answers. An absolute one
  meets whatever the host decides; `CLAUDE.md` excludes network *actions* nowhere, and nothing here
  reaches a network.
- **XFDF is declined by name**, because ISO 19444-1's XML spelling would need an XML parser — a
  dependency rather than a clause, and the decision §12.7.6.4 already took. Which format a name
  states is now read once for both clauses by `pdf_model::action::data_format`, so the two cannot
  drift.
- **It is asked for last**, after the page, the view and the search — which is the annex's own
  recommendation to a writer read as an instruction to a reader: "the fdf parameter is recommended
  to be the last parameter so that the document can open directly to the appropriate view".
- **No new policy, deliberately.** Annex O attaches its security sentence — "a PDF processor may
  choose to prompt the user or even prevent opening of the file" — to `ef` and to no other
  parameter, and `viewer_host::may_write_extracted` is where that lives. An import writes nothing
  to a person's disk, reads only what is already beside the document, and is the same operation a
  click on §12.7.6.4's action performs; a second policy value would have been this project deciding
  something the standard did not ask and no host requested.

## Consequences

- **`Parameter::unhonoured` names nothing.** All eleven of Annex O's parameters are carried out.
  The function stays, with its shape and its `return None`, because the answer decays in the other
  direction too — a format withdrawn or a dependency lost needs somewhere to say so — and because
  `tools/state.sh annex-o` reads that function's text whichever way it answers. That is the one
  place in this change with a lint exemption, and its reason names the tool.
- **§O.2.2 is `implemented`**; §O.2.1 stays `partial` for the one sentence it always owed — "[a]ny
  remaining parameters after this parameter apply to the selected embedded file" needs a second
  document opened, which `Command::Open` makes a host's. §O's aggregate row and §12.7.6.4's are
  amended: that clause's channel has a second caller now.
- **One `Query` and one `Answer`**, the first since the four-hundred-and-first session. Three
  consumers failed to compile — `viewer-confined`'s wire protocol both ways and `viewer-core`'s own
  dispatch — which is what nothing being `#[non_exhaustive]` is for. The C ABI does not fail to
  compile and that is what its two counting tests exist for: 111 → **112** entry points, 103 → 104
  signatures.
- **Six tests**: four in `viewer-core/tests/fragments.rs` — the rectangle's device pixels derived
  from `Query::PageGeometry` and the file's own `/MediaBox`, the page it belongs to, the FDF asked
  for and imported end to end with the test playing the host, and an XFDF declined by name — and
  `pdf-model`'s own list of the eleven, which now asserts that none is refused. The fragments and
  the FDF file are written by hand and that is unavoidable: no corpus document carries a fragment
  identifier, and none carries an FDF beside it.
- **No pixel moved by the core**: nothing here changes what a page draws. What changed on a screen
  is chrome three hosts draw over it, which no gate rasterises.
- **One duplication removed on the way.** `viewer-ui` had two copies of "fill a list of
  quadrilaterals in one colour" — the selection's and the find bar's — and the third would have made
  three; `overlays::highlight_list` takes the colour now and `find.rs` keeps its hue as a constant
  with the argument for it.
