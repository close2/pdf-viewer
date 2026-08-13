# ADR 0310 — The file a fragment names, and the sentence that was about the ones after it

Status: accepted, 2026-08-13 (session 475).

## Context

`tools/state.sh annex-o` reads `pdf_model::fragment::Parameter::unhonoured` and prints which of
Annex O's eleven parameters this program carries out. Before this session it printed three refused —
`ef`, `highlight`, `fdf` — and `ef`'s reason, verbatim from the arm:

> opening an embedded file is the host's decision, and every parameter after this one applies to
> that file rather than to this document

That is two claims joined by an "and", which `doc/habits.md` names as the fifth shape a refusal takes
when it has outlived its reason: **split a refusal into one claim per entry before believing it.**

Table Annex O.3 in §O.2.1 states the requirement, and it is the annex's only `shall` on a processor
that is not about where the window points:

> The name argument shall be a byte string used to match a file specification dictionary in the
> EmbeddedFiles name tree. Any remaining parameters after this parameter apply to the selected
> embedded file. When used as part of a PDF open parameter, the PDF processor shall open the
> embedded file contained within the EmbeddedFiles name tree identified by name . Security should
> be strongly considered when opening an embedded file. When opening a file that is not from a
> trusted source, a PDF processor may choose to prompt the user or even prevent opening of the file.

Read against the two claims:

- **"[E]very parameter after this one applies to that file"** is the annex's second sentence and it
  is true, and it is about the parameters *after* `ef` — not about `ef`. Carrying `ef` out does not
  require carrying out what follows it.
- **"[O]pening an embedded file is the host's decision"** is also true, and it is not a blocker. It
  is the *mechanism*. §7.11.4's bytes are inside the document, so nothing is fetched; `viewer-core`
  has had `Command::Extract`, `Event::Extracted` and `hand_over` since the hundred-and-sixty-ninth
  session, and §12.5.6.15's file attachment annotation has used the same channel since ADR 0295. A
  refusal whose reason is "a host decides" in a crate that has a message for handing a host a
  decision is a refusal that has run out.

`doc/todo/39` had recorded the blocker as `doc/todo/38`'s four levels — "a policy, not a mechanism".
That is the third reading and it is the one the annex settles: the security sentences are a
**should** and a **may**, and the sentence they qualify is a **shall**. A processor that opens the
file conforms; a processor that prompts or declines also conforms, and the annex says so in the same
row.

## Decision

### The fragment's `ef` is carried out, and the bytes cross where every other extraction's do

`Open::apply_fragment` records the tree key on `Open::opening_file`, exactly as Annex O's `search`
records a plan on `Open::searching`, and `Viewer::open` takes it and runs the extraction the moment
the document is open — before the first page's events, because Table Annex O.3 files this parameter
under *object identifiers* and the file is out of bytes already read rather than out of a page
nobody has drawn.

No message was added. That is the point rather than a saving: `doc/ui-boundary.md`'s rule is that a
variant is added only when an existing one does not already say the thing, and `Event::Extracted`
already says "here are §7.11.4's decoded bytes; where they go is yours".

The name is matched against §7.7.4's tree key, decoded as §7.9.2's text string, which is the same
comparison `fragment::annotation_named` makes for `comment` and for the same reason: the annex gives
its byte string no character encoding and the file's own string states one. **Ten of the corpus's
964 documents carry an `/EmbeddedFiles` tree, 23 files between them** — the population, counted.

### What is still owed is the *other* sentence, and it stays reported

"[A]ny remaining parameters after this parameter apply to the selected embedded file" would need a
second document opened, and `Command::Open` is a host's. So `apply_fragment` stops at `ef` and says
how many parameters it did not apply — which is what it did before, moved from the refusal branch to
the applied one. §O.2.1's ledger row stays `partial` for exactly that, with the sentence named.

### `Event::Extracted` says *what asked*, and this is the half that is not bookkeeping

Carrying the parameter out without this would have been a defect shipped in the same commit. All
three hosts write an extracted file to disk unasked, because until this session the only thing that
could produce one was a person: `Command::Extract` from the files panel, or a click on §12.5.6.15's
annotation. A URI is not that. §O.1 says why in the annex's own words — fragment identifiers are
"useful primarily when referring to them from external to the PDF such as a web page or web API" —
and it is the reason the security sentence is attached to this parameter and to no other in either
table.

So `Event::Extracted` gains `asked: Extraction`, with `Extraction::Asked` and
`Extraction::Fragment`. `viewer-core` takes no view of which is safe; it says which happened, once,
where a host can see it.

**A variant changing shape rather than a variant being added**, which is the mechanism
`doc/ui-boundary.md` describes for exactly this case — "where a host needs two things a variant
carried one of, the variant changes and every consumer fails to compile". One caveat learned here
and worth writing down: **a `..` pattern defeats that.** Every host matched
`Event::Extracted { name, bytes, .. }`, so adding a field broke *no* host's build; the compiler
enforces the rule for a new *variant* and not for a new *field*. The three hosts were changed
deliberately, and `PDFV_EVENT_KIND_COUNT` stayed at 16 because no kind was added.

### The decision itself is `viewer-host`'s, stated once

`viewer_host::may_write_extracted` is the third policy in `viewer_host::policy`, beside §12.7.6.4's
import file and §7.6.4.1's password. It takes the annex's second option — **prevent**, not prompt —
because none of the three hosts has a dialogue to prompt with, and it says so to the person:

```
note: the URI's fragment asked for this embedded file rather than a person, so it was not written
to disk — open it from the files panel to extract it (ISO 32000-2 §O.2.1)
```

`doc/todo/38`'s *ask* level is where this becomes the annex's other option, and nothing here has to
be revisited for it: the policy is already asked in one place, off a value a host can see.

`viewer-ui` gained a dependency on `viewer-host` for it — the crate is toolkit-free and this is what
it is for, so one policy serves three hosts rather than three copies of it drifting.

## What the other two are, re-derived rather than restated

The round was asked to check both, and the answers changed shape without changing side.

**`highlight`.** Its reason read "this program highlights a range of a page's text, and a rectangle
is not one" — a reason that names a *vocabulary*, which is the first of `doc/habits.md`'s six shapes
and usually means the refusal has expired. It has not, and the annex is what says so. Table Annex
O.4:

> Open the document with the specified rectangle highlighted. Each argument shall be an integer or
> floating point value representing the rectangle measured from the top left corner of the page. The
> nature of the highlighting is implementation-dependent.

The tempting reading is that this program's *selection* is the highlight — it is already geometry, a
`Selected::quads` is already a list of quadrilaterals, and a rectangle is one. The annex refuses it:
the two neighbouring parameters that point a person at something both say **selected** —
`comment`'s "open the document with the specified comment selected", `search`'s "selecting the first
matching word in the document" — and this one says **highlighted**, with an implementation-dependent
nature the other two are not given. A standard that meant "select" three rows apart would have
written "select". So it is a shape of its own, and carrying it out means a new `Query` answering a
rectangle in device pixels, which is ADR 0164's test: not added before a host asks. The reason in
the code now says that, in the annex's words, so the next round does not re-derive it.

**`fdf`.** The round was asked to check that `CLAUDE.md` really closes it. It does not — the
exclusion list excludes authoring, multimedia, XFA and script behaviour, and says nothing about a
network. Principle 3 keeps the filesystem and the network out of the *renderer*, which is a
statement about `pdf-model` and not about a host, and the shape a fetch would take already exists:
`Event::NeedsFile`, `Command::Supply { purpose: ImportData }`, §12.7.8's reader. What is missing is a
host with a URI to resolve the argument against — which is what the existing reason says, so it
stands unchanged and is the only one of the three whose wording needed nothing.

## Seeing it

`Xvfb :175`, `pdf-viewer --cpu 'attachment.pdf#ef=foo.txt'` — the pdf.js corpus document whose name
tree is `<</Names [(foo.txt) 15 0 R]>>`:

```
attachment.pdf: 1 page(s)
attachment.pdf: 0 outline item(s), 0 layer entr(ies), 1 embedded file(s), 0 article thread(s) …
note: this document carries an embedded file: foo.txt, 9 bytes
note: the URI's fragment asked for this embedded file rather than a person, so it was not written
to disk — open it from the files panel to extract it (ISO 32000-2 §O.2.1)
```

and the directory afterwards holds `attachment.pdf` and nothing else. With `#ef=nothing.txt&page=1`
the same run says "and the 1 parameter(s) after it apply to that embedded file rather than to this
document, so none of them was applied" and then "this document embeds no file called
\"nothing.txt\"" — the lookup happened, and a name the tree does not hold is reported rather than
swallowed.

## What is left open

- **§O.2.1 is `partial`, not `implemented`**, on one sentence: the parameters after `ef` apply to the
  embedded file, and composing that needs a second `DocumentId` the core cannot open for itself.
- **`highlight` and `fdf` still report by name**, with reasons re-derived above.
- **The prompt.** `may_write_extracted` prevents where the annex also permits asking. That is
  `doc/todo/38`'s *ask* level arriving from a second direction, and the policy is already in the one
  place it would change.
