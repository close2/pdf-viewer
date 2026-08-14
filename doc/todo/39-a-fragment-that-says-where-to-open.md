# A fragment that says where to open

Status: **all eleven parameters are carried out since the five-hundred-and-twenty-second session,
and `Parameter::unhonoured` names none.** §O.1, §O.2 and §O.2.2 are `implemented`; §O.2.1 is
`partial` for one sentence of Table Annex O.3's `ef` row and nothing else, which is what is left of
this item. ADRs 0209, 0250, 0310, 0357.
Priority: 39 — capability, and what is left of it is one sentence about a second document
Clauses: Annex O (§O.2.1, §O.2.2), §12.7.8, §7.11.4
Code: `crates/pdf-model/src/fragment.rs`, `crates/viewer-core/src/open.rs`,
`crates/viewer-host/src/policy.rs`

**Run `tools/state.sh annex-o` before reading any of this.** It reads
`Parameter::unhonoured` — the program's own answer — and this file is a reading beside it.

## What is built

`pdf_model::fragment::Fragment::parse` reads all eleven parameters from the text after `#`, in the
order §O.2 makes normative, naming what it could not read rather than dropping it.
`viewer_core::Open::apply_fragment` carries out **all eleven** — `page`, `nameddest`, `structelem`,
`comment`, `ef`, `zoom`, `view`, `viewrect`, `highlight`, `search` and `fdf` — immediately after
Table 29's `/OpenAction` as §O.2.2 asks. `Command::Open` carries the fragment undecoded, and
`pdf-viewer doc.pdf#page=5` is the first caller.

**Four parameters have come off the refused list, and not one for the reason the list gave.**
`search` in the four-hundred-and-fourteenth, when `viewer_core::Command::Find` became a
document-wide search: the plan is made as the document opens and the *host* walks it, one page per
`Find::Continue`, because reading all 1023 pages of ISO 32000-2 is 5.84 s of interpretation and
`CLAUDE.md`'s startup rules do not permit that before page one is drawn. The word list is Annex O's
own — any of the words matching is a match — and the search does not wrap, because "the first
matching word **in the document**" would otherwise mean nothing (ADR 0250). `ef` in the
four-hundred-and-seventy-fifth, when nothing arrived at all: its reason was two claims joined by an
"and", and only the second was ever about `ef` (ADR 0310). And `highlight` and `fdf` in the
five-hundred-and-twenty-second (ADR 0357): the first because a refusal that ends "no host has asked
for one to draw" is answered by the *annex* asking — ADR 0316's precedent, sharpened by the fact
that no host can answer this question for itself, since no host sees the fragment — and the second
because `Event::NeedsFile` had reached three hosts while "no host supplies one yet" stood in the
code.

## What is refused, and two limits that are not refusals of this annex

Nothing in Table Annex O.3 or Table Annex O.4 is refused. `Parameter::unhonoured` answers `None`
for every one of the eleven and is kept for the decay in the other direction — a format withdrawn
or a dependency lost has somewhere to say so, and `tools/state.sh annex-o` reads that function
whichever way it answers.

Two limits are worth naming because they are *not* refusals of this annex and a later round should
not read them as ones:

- **XFDF.** `fdf`'s URI may name "an FDF or XFDF file", and ISO 19444-1's XML spelling is declined
  by name for want of an XML parser — which is a dependency rather than a clause, is the decision
  §12.7.6.4 already took, and is now taken once for both by `pdf_model::action::data_format`. It
  belongs to §12.7.6.4's row.
- **Where a host looks for the file.** `viewer_host::resolve_import` is the policy: a single path
  component beside the open document, so a *relative* URI imports and an absolute one is refused
  out loud. That is a statement about these three hosts rather than about the annex, and a host
  with a network and a base URI would satisfy the same `Event::NeedsFile`.

## What `ef` still owes, which is one sentence rather than the parameter

§O.2.1: "[a]ny remaining parameters after this parameter apply to the selected embedded file." That
would mean opening a *second document* from the first and applying the rest of the fragment to it,
which `DocumentId` can express and nothing composes — `Command::Open` is a host's. So
`apply_fragment` carries `ef` out and then stops, saying how many parameters it did not apply, and
§O.2.1's ledger row is `partial` for that and nothing else.

**The other half of `ef` is a host policy and it is built**: `viewer_host::may_write_extracted`,
beside §12.7.6.4's import policy. `Event::Extracted` says whether a person or a URI asked, and the
three hosts write the first and decline the second, in the annex's own words — "a PDF processor may
choose to prompt the user or even prevent opening of the file". `doc/todo/38`'s *ask* level is where
*prevent* becomes *prompt*, and the policy is already in the one place that would change.

## What not to do

- **Not a fourth copy of the highlight.** `Query::Highlight` answers the rectangle in device pixels
  through `Viewer::device_quad`, which is ADR 0118's one arithmetic; a host draws it in a colour of
  its own and computes nothing.
- **Not a URI parser.** RFC 3986 splitting is the host's; what crosses is the fragment alone. The
  rule `pdf-viewer` uses is in ADR 0209: the filesystem decides, not the punctuation.
- **Not a second reading of Table 149.** `View::from_keyword` is the one place, with §12.3.2.2's
  array and Annex O's `view` parameter as its two callers.
- **Not a fourth copy of the extraction policy.** Three hosts, one `viewer_host::policy` function;
  a fourth host calls it rather than deciding again.
