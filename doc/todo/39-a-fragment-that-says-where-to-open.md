# A fragment that says where to open

Status: **found in the three-hundred-and-sixtieth session, by giving the conformance ledger the
annexes.** Annex O is normative, eleven of its parameters are `shall`s addressed to "the PDF
processor", and not one of them is implemented or reported. Its five rows are the ledger's only
`silent` ones.
Priority: 39 — capability, and the first thing a *host* will ask for that this vocabulary cannot say
Clauses: Annex O (§O.1, §O.2, §O.2.1, §O.2.2), §12.3.2.2, §12.3.2.4, §12.4.2, §12.5.2, §7.11.4,
§12.7.8
Code: `crates/viewer-core/src/command.rs`, `crates/viewer-core/src/viewer.rs`

## What the annex says

A fragment identifier is what follows `#` in a URI, and Annex O defines the ones that mean something
to a PDF: `report.pdf#page=12`, `report.pdf#nameddest=Chapter3`, `report.pdf#zoom=150,0,792`. The
grammar is three sentences — parameters separated by `&`, arguments by `,`, and

> Fragment identifiers shall be processed and (if required) executed from left to right as they
> appear in the character string that makes up the fragment identifier.

Eleven parameters, in two tables:

| | parameters | what this tree already has |
|---|---|---|
| **O.2.1** object identifiers | `page`, `nameddest`, `structelem`, `comment`, `ef` | §12.4.2's page numbers, §12.3.2.4's named destinations, §14.7's structure elements, Table 166's `/NM`, §7.11.4's `EmbeddedFiles` tree |
| **O.2.2** open parameters | `zoom`, `view`, `viewrect`, `highlight`, `search`, `fdf` | the magnification and scroll `viewer-core` has owned since session 132, §12.3.2.2's explicit destinations, §12.7.8's FDF reader |

**Nine of the eleven name a mechanism this tree implements.** What is missing is the one sentence
that joins a URI's fragment to them, and it does not exist anywhere: `Command::Open { id, bytes,
password }` has nowhere to put it.

## Why it is `silent` rather than `partial`

Nothing detects a fragment, so nothing can report one. A host that hands this program the bytes
behind `report.pdf#page=12` gets page one and no word about it — which is the exact shape the
ledger's `silent` status exists to name, and the reason it is worth a row rather than a shrug.

**It is also the ledger's first `silent` row that no *file* can trigger.** A document cannot contain
a fragment identifier; the fragment arrives with the request. So the corpus and the oracle are blind
to this by construction, and no amount of running them would ever have found it — which is
`CLAUDE.md`'s two-denominators argument in its purest form.

## What implementing it means

- **The fragment crosses as text, once.** `Command::Open` gains an optional fragment — the string
  after `#`, undecoded — because percent-decoding is the URI's business and `structelem`'s argument
  is defined as "a byte string with URI encoding". `viewer-core` parses; the host does not.
- **Ordering is the requirement, not the parameters.** Left to right, after the document's own
  `/OpenAction`: O.2.2 says these "should be processed immediately after any other
  document-specified open parameters have been processed", and the `comment` parameter's NOTE turns
  the order into behaviour — "[u]nless the page on which the comment resides has been selected prior
  to the comment parameter, the comment will not be selected".
- **Each parameter becomes the message that already exists.** `page` and `nameddest` are
  `Command::GoTo(PageTarget)`; `view` is §12.3.2.2's destination, which `destination.rs` already
  turns into one of eight forms; `zoom` and `viewrect` are the magnification and scroll; `highlight`
  is a quad in the same device pixels `Answer::Selection` answers in. **No new vocabulary for nine
  of the eleven**, which is the test ADR 0164 set for a feature like this.
- **Two need a decision rather than a wire.** `search` needs a text search this program does not
  have, and `fdf` needs to fetch a URI, which principle 3 forbids the renderer — so it is a host's
  fetch and a `Command::Supply`, the shape `Event::PasswordRequired` already uses.
- **`ef` needs the security sentence read first**: "[s]ecurity should be strongly considered when
  opening an embedded file … a PDF processor may choose to prompt the user or even prevent opening
  of the file". That is `doc/todo/38`'s levels, arriving from a second direction.

## What not to do

- **Not a URI parser.** RFC 3986 splitting is the host's; what crosses is the fragment alone.
- **Not before a host asks.** ADR 0166 and 0167 are two rounds' evidence that a message's shape is
  settled by its second consumer, and `viewer-ui` opens files from a path with no fragment in sight.
  The honest order is `doc/todo/30`'s native host first — which is also the consumer that would
  *have* a URI.
