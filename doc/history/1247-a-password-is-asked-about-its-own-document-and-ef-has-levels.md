# 1247 — A password is asked about its own document, and `ef` has four levels

The HOST-UI round, sequential mode. ADRs 1331, 1332. No ledger row was briefed and none moved.

## The defect (ADR 1332)

The retry path for any name that was not arriving did reopen the front document's bytes (GTK and Qt
through `open_document`, `quorra` through its front path), but no GoToR target ever reached it:
`resume_remote` opened the file with the empty password and declined the link. Now the core holds
the jump (`interact::Locked`), raises `PasswordRequired` under the name the document would open
under and takes `Command::Open` as the retry (`place_remote` shared with the unprompted jump). The
host's reservation moved into `Arrivals` (`offer`, `locked`, `settle`), so GoToR and `ef` retry
through `Arriving::bytes`; `documents::may_ask_about` says which documents a prompt may be put
about.

## The levels (ADR 1331)

`EmbeddedDocuments`: `refuse|ask|warn|open`, `ask` by default (A67 and A98's argument, one act
over). It is read once in `may_open_extracted`, sits on the third menu group in all three windows
(`Row::ActLevel`), is set by `--embedded-documents=`, and is `refuse` in `quorra-confined`.

## Driven under Xvfb

- GoToR to an encrypted `second.pdf` (user password `abc`), `/NewWindow true`, `first.pdf` alone,
  in GTK, Qt and `quorra`: remote-documents question, then the password prompt naming second.pdf.
  A wrong password gives attempt 2 of 3. The right one opens second.pdf at page 2 in its own tab,
  and first.pdf is untouched on Ctrl + Tab. The link document was also opened second, behind a
  plain one (Qt: three tabs).
- `/NewWindow false` from a tab opened second (all three): it becomes second.pdf at page 3.
- `'issue17056.pdf#ef=destination-doc.pdf&page=3'` in all three: `ask` puts up the card and opening
  gives page 3 of 30 in a tab. `refuse` declines with the sentence. `warn` opens with the sentence,
  alone and opened second. The menu rows were ticked and a pick was reported (GTK, Qt, `quorra`).

Tests: `a_remote_file_that_asks_for_a_password_is_asked_about_under_its_own_name` (viewer-core),
`a_remote_document_refused_for_a_password_is_retried_as_itself`, `tests/embedded_documents.rs`,
`the_card_offers_the_embedded_documents_levels_and_ticks_the_chosen_one` (chrome).

Left: a replacement without a prompt keeps the tab's old label and path; §12.6.4.7's and §12.7.8's
files still decline on a password; the remote-documents question says "in place of" under
`/NewWindow true` too.
