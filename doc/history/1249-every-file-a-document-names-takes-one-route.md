# 1249 — Every file a document names is asked about and replaced by one route

The HOST-UI round, batch thirty-eight. ADR 1335. No ledger row was briefed and none moved.

## What was built

- **A replacement without a prompt relabels its tab.** `Arrivals::settle` takes the offered name
  or the source's, so Table 203's `/NewWindow false` and a §12.6.4.7 thread relabel the tab and move
  the window's path as a prompted replacement does. `Arrivals::supplied` spends an offer nothing took
  once its `Command::Supply` is answered (all three windows' `run`/`pump`).
- **§12.6.4.7's and §12.7.8's files prompt.** `interact::Held` holds a remote, a thread or a named
  page. The thread retries under the source's name and opens at its bead. The named page is asked
  under the offered name and drawn into the form, and the retry answers `Event::Closed` for that name.
- **The question says beside or in place.** `Event::NeedsFile` carries `beside` (Table 203/204
  `/NewWindow true`; one bit on the confined wire), and `viewer_host::remote` words it.
- **`quorra.h`** says to answer a password raised after `quorra_supply` under the event's id, with
  that file's bytes. `doc/ui-boundary.md` says the same.

Tests: `a_thread_file_that_asks_for_a_password_is_asked_about_and_opened_at_its_bead`,
`a_named_page_file_that_asks_for_a_password_is_drawn_in_once_it_is_given` (encrypted fixtures from
`serialize_encrypted`), `the_question_carries_whether_the_file_opens_beside`,
`a_replacement_without_a_prompt_is_the_offered_file`, `the_remote_documents_question_says_beside_or_in_place`,
and viewer-ffi's `a_password_asked_about_a_supplied_file_is_answered_under_the_name_on_the_event`.

## Driven under Xvfb, all three windows

- `first.pdf` alone, `--remote-documents=open`: the `/NewWindow false` link replaced it with
  second.pdf at page 3, and the title followed (quorra, GTK, Qt).
- `first.pdf note.pdf`, ask: the question said "in place of", and after Go ahead the tab read
  "The Second Document" (§14.3.3 `/Title`), with the title second.pdf, also after switching tabs.
- `note.pdf first.pdf` (the page-mode file second, its panel obeyed): the `/NewWindow true` link was
  asked about "beside the one you are reading" and opened in a third tab. The thread link to an
  encrypted `articles.pdf` asked "in place of", then prompted naming articles.pdf. A wrong password
  gave attempt 2 of 3 (quorra), and `abc` opened it at page 3 in first.pdf's tab, relabelled.

Left: a named page's encrypted file was tested, not driven. The C ABI does not expose `beside`.
A cancelled named-page prompt leaves its references waiting until the next import asks again.
