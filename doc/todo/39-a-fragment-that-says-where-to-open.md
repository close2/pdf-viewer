# A fragment that says where to open

Status: **done** for the annex's rows. All eleven parameters are carried out since the five-hundred-and-twenty-second
session and `Parameter::unhonoured` names none; **§O.2.1's last sentence — the parameters after `ef`
— is carried out since the five-hundred-and-ninety-sixth**, so all four of Annex O's rows are
`implemented` and nothing in the annex is reported. **One design is owed**: `fdf` naming an
absolute URI, which is a fetch over the network and is refused out loud until it is built under
`Submissions`' level (below). The rest is *not this annex's*: the two limits named below, and
`doc/todo/38`'s ask level. ADRs 0209, 0250, 0310, 0357, 0431.
Cited by: `CLAUDE.md`'s normative-annexes entry, beside `tools/state.sh annex-o` — the reason the file is kept whole rather than deleted (ADR 1416).
Priority: 39 — one design owed (`fdf` over the network); kept as the reading beside `tools/state.sh annex-o`
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
Table 29's `/OpenAction` as §O.2.2 asks, and the parameters *after* `ef` leave with the file they
are about. `Command::Open` carries the fragment undecoded, and
`quorra doc.pdf#page=5` is the first caller.

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
not read them as ones, and one design is owed:

- **Eleven parameters, not seventeen.** `pagemode`, `toolbar`, `statusbar`, `scrollbar`,
  `navpanes`, `messages` and `collab` are another vendor's open parameters, printed in no table of
  ISO 32000-2, and `xfdf` is not a parameter at all — the annex's `fdf` names "an FDF or XFDF
  file". A fragment carrying one is named and the rest still runs (`fragments.rs`'s
  `a_parameter_this_program_cannot_read_is_named_and_the_others_still_run`). ADR 1523.
- **A `zoom` outside 2% to 6400%** lands on the nearer bound and is named: "the percentage to
  which the document should be zoomed" is a `should` with no bound, and the bound is the one a
  person meets with the keys (ADR 1523).

### Owed: `fdf` over the network — a design, not built

Both formats are read: `pdf_model::xfdf` (ADR 1108) and §12.7.8's FDF, imported on one channel and
tested end to end in `fragments.rs`. **What is not done is fetching an absolute URI.** "The URI
shall be either a relative or absolute URI to an FDF or XFDF file", and `viewer_host::resolve_import`
answers an absolute one with `ImportRefusal::OutsideTheDocumentsDirectory`, out loud. The build,
when it is taken:

1. **The act is the network, so the level is `viewer_host::Submissions`'.** A GET of a URL a
   fragment named tells that server this document was opened here — the same act class as
   §12.7.6.2's submission, a machine contacting a server on somebody else's word — and ADR 1291's
   one level is read for it rather than a fifth level invented. `refuse` declines out loud, `ask`
   (the default) puts the URL and its host to the person, `warn` fetches and says so, `send`
   fetches. **The fragment's origin is not consulted**: a fragment can arrive from a person's
   command line or from a document's own `ef` remainder, and a policy that trusted the first would
   have to tell them apart where nothing does.
2. **One function, in `viewer_host::policy`**, `may_fetch_import(url, level) -> Sending` on
   `may_submit`'s shape and order: no scheme or a scheme outside `SUBMIT_SCHEMES` is refused at
   every level, and `submit::check_url` runs before the TLS stack (ADR 1327); only then the level.
   The four levels attach there and nowhere else (`CLAUDE.md` principle 3's shape).
3. **The fetch is `viewer_host::submit`'s client**, a GET instead of a POST, with the response
   bounded in bytes as a submission's answer is; the format is still `action::data_format`'s answer
   off the URI's name, never the response's media type, so one opinion of what an `.xfdf` is holds.
4. **The bytes cross as they do now**: `Command::Supply { purpose: Purpose::ImportData, .. }`, so
   `viewer-core` gains nothing and stays without a network (`doc/ui-boundary.md`'s rule 2). The
   confined window refuses at every level, as it refuses a submission.
5. **The test is a local server**, `viewer-host/tests/submit.rs`'s, answering an FDF — and a
   refusal test per level.

## What `ef` owed, and how the sentence was finally composed

§O.2.1: "[a]ny remaining parameters after this parameter apply to the selected embedded file." That
means opening a *second document* from the first and applying the rest of the fragment to it, which
`DocumentId` could always express and nothing composed — `Command::Open` is a host's. The
five-hundred-and-ninety-sixth session composed it in three pieces, each on a boundary that already
existed (ADR 0431): `Fragment::parse` **stops** at `ef` and keeps the remainder whole and undecoded
in `after_embedded_file`, because those parameters are not this document's; `Event::Extracted`
carries that remainder beside the bytes — a variant changing shape, not a message added, since a
host has the fragment but not §O.2's grammar; and a host hands both back as `Command::Open`. The
window verifies it: `quorra 'issue17056.pdf#ef=destination-doc.pdf&page=3'` titles itself
*destination-doc.pdf — 3 — page 3 of 30*.

**`viewer_host::may_open_extracted` is the second policy question**, beside `may_write_extracted`:
showing a file in this reader is the row's `shall`, and it is asked at one of four levels,
`viewer_host::EmbeddedDocuments` — `refuse`, `ask`, `warn`, `open`, `ask` by default, set with
`--embedded-documents=` and from the restriction menu's third group in all three windows, and
`refuse` in `quorra-confined` (ADR 1331). *Ask* is the annex's "prompt the user", *refuse* its
"prevent opening". A file a person asked for from the files panel is not under the level. Writing
the file into somebody's directory is still declined for a URI.

`Event::Extracted` is what says which of the two asked, so the annex's own words — "a PDF processor
may choose to prompt the user or even prevent opening of the file" — are answered off a value rather
than guessed at.

## What not to do

- **Not a fourth copy of the highlight.** `Query::Highlight` answers the rectangle in device pixels
  through `Viewer::device_quad`, which is ADR 0118's one arithmetic; a host draws it in a colour of
  its own and computes nothing.
- **Not a URI parser.** RFC 3986 splitting is the host's; what crosses is the fragment alone. The
  rule `quorra` uses is in ADR 0209: the filesystem decides, not the punctuation.
- **Not a second reading of Table 149.** `View::from_keyword` is the one place, with §12.3.2.2's
  array and Annex O's `view` parameter as its two callers.
- **Not a fourth copy of the extraction policy.** Three hosts, one `viewer_host::policy` function;
  a fourth host calls it rather than deciding again. **Nor a second default**: `ask` is ADR 1331's,
  on the argument the owner ratified for links and forms (`doc/questions/A67`, `A98`).
- **Not a counter on the chain.** A document may embed a document whose fragment names another
  `ef`, and nothing guards the depth because nothing has to: each open consumes at least `ef=` and
  its argument, so the remainder is strictly shorter every time.
- **Not a second window rule.** All three windows open the embedded document in a tab of its own,
  through `viewer_host::Arrivals::wait_held`, and the document that named it keeps its tab: in front
  where the holder was in front, behind where it was behind (ADR 1316). `viewer_host::opens_as_document`
  is the one test for whether extracted bytes are opened or written.
