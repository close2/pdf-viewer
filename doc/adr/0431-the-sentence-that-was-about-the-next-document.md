# ADR 0431 — The sentence that was about the next document

Status: accepted, 2026-08-19. Session 596.

## Context

ISO 32000-2 Annex O's `ef` parameter has two sentences in its row, and this tree carried out one of
them. Table Annex O.3, §O.2.1:

> The name argument shall be a byte string used to match a file specification dictionary in the
> EmbeddedFiles name tree. Any remaining parameters after this parameter apply to the selected
> embedded file. When used as part of a PDF open parameter, the PDF processor shall open the
> embedded file contained within the EmbeddedFiles name tree identified by name . Security should be
> strongly considered when opening an embedded file. When opening a file that is not from a trusted
> source, a PDF processor may choose to prompt the user or even prevent opening of the file.

The `shall` was carried out by ADR 0310: §7.11.4's bytes come out of the document as
`viewer_core::Event::Extracted`, and where they then go is the host's. The middle sentence — "[a]ny
remaining parameters after this parameter apply to the selected embedded file" — was the last thing
Annex O owed, and it was the *only* reason §O.2.1's ledger row read `partial` for a hundred and
twenty-one sessions. What stood in its place was a refusal said out loud: `apply_fragment` stopped
at `ef` and reported how many parameters it had not applied.

The refusal's reason was architectural rather than doubtful. Applying `page=3` after `ef` means
opening a *second document* and applying it to that one, and opening a document is
`Command::Open` — a host's message, sent to this crate rather than by it (rule 2).

## Decision

**The sentence is carried out, split three ways along boundaries that already existed.**

1. **`pdf_model::fragment::Fragment::parse` stops at `ef`** and keeps everything after it whole and
   undecoded in a new field, `after_embedded_file`. Those parameters are not this document's at
   all, so reading them into `Fragment::parameters` — where the only thing that could happen to
   them is the wrong thing — was itself the mistake. The remainder is a *substring of the URI*
   rather than a re-spelling, because a fragment this reader had split and printed again would be
   this program's sentence rather than the one somebody wrote.
2. **`Event::Extracted` carries it beside the bytes.** A variant changed shape; no message was
   added. `doc/ui-boundary.md`'s rule for a new message is "a question a host cannot answer for
   itself" — and a host *has* the whole fragment, so a new `Event` would have failed that test.
   What a host does not have is §O.2's grammar, and six consumers each splitting a fragment for
   themselves is six chances to disagree about where a parameter ends. So it travels as a field of
   the event that already carries the file, which is the shape this boundary prefers: "where a host
   needs two things a variant carried one of, the variant changes".
3. **A host composes the two into `Command::Open`**, which is where the annex's sentence finally
   becomes true. `viewer-ui`'s `pdf-viewer` opens the embedded bytes into the window that named
   them, with the remainder as their fragment, so `page`, `search` and `highlight` after `ef` are
   applied to the file that came out.

**Two policy questions in `viewer_host::policy` where there was one.** `may_write_extracted` stays
as it was: a URI's `ef` does not put a file on somebody's disk. `may_open_extracted` is new and
answers `Ok` for both provenances — showing the file *in this reader* is the row's own `shall`, it
happens inside a process principle 3 gives no filesystem and no network, and nothing is left behind
when the window closes. The annex's caution is about the act that leaves something behind. Both are
functions rather than inline answers so that `doc/todo/38`'s *ask* and *warn* levels are a change in
one place, which is what `CLAUDE.md` §3 requires of a restriction.

## Two things that needed no rule

**Termination.** A document may embed a document, and a fragment may name an `ef` inside the file a
previous `ef` opened. No counter guards it: each open consumes at least `ef=` and its argument from
the fragment, so the remainder is strictly shorter every time and the chain runs out of fragment.

**What the window shows.** This host has one document, so the embedded file *replaces* the one that
named it, and that is a host's choice rather than the annex's requirement — everything after `ef` is
a sentence about the embedded file, so the file the URI is about is the file the window shows. A
host with tabs opens a second `DocumentId` and needs no other change. It cost one field on the host:
§7.6.4.1's password prompt re-opens the document, and an embedded one has no path to re-read.

## Consequences

- §O.2.1 is `implemented`; **every `shall` in Annex O is now carried out and none is reported**.
  `tools/state.sh annex-o` reads `Parameter::unhonoured`, which named none before this round and
  names none after it — the row it changed is the ledger's.
- **Only a PDF is opened.** An embedded spreadsheet goes to `write_extracted` and its policy, on
  §7.5.2's header, because this window can show nothing else and a person did not ask for a failure.
- One test moved: `an_embedded_file_stops_the_parameters_after_it` is
  `an_embedded_file_carries_the_parameters_after_it`, and it now opens the second document and
  asserts the page the fragment named — in `viewer-core`, and again in the real window, where
  `issue17056.pdf#ef=destination-doc.pdf&page=3` shows *destination-doc.pdf — 3 — page 3 of 30*.
- `pdf-model` learned nothing about URLs, which is where `doc/todo/39` said the boundary runs: what
  it gained is the *end* of a fragment, which is Annex O's own grammar and nothing else's.
