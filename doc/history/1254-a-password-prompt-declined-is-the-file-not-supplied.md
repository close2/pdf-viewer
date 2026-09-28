# 1254 — A password prompt declined is the file not supplied

The HOST-UI round, batch thirty-nine. ADR 1345. No ledger row was briefed and none moved.

## What was built

- **A cancelled prompt ends the hold** (ADR 1335's cost). `Command::Supply` with no bytes, under the
  purpose the file was supplied for, is about a file held for §7.6.4.1's password: `Viewer::withdraw`
  ends the hold, gives back an offered name with `Event::Closed`, and `interact::withdraw` says the
  password was not given, lets a named page's references go (each widget keeps its own `/AP`) and
  asks for the next awaited file. `viewer_host::Arrivals::offer` takes the purpose; `cancel` and
  `declined` hand the decline over where each window starts its next arrival (outside the pump).
- **`beside` crosses the C ABI**: `quorra_event_needs_file_beside`; `quorra.h` says how to decline.
- **`scene.rs`'s refusal names the pair**: a four-component mask group (ICCBased, or DeviceCMYK that
  blends, ADR 1342) is refused as the pair of rasters; three CIE-based components as their own `Y`.

Tests: `a_named_page_file_whose_password_is_not_given_is_declined_and_let_go`,
`a_thread_file_whose_password_is_not_given_declines_the_link_with_the_reason` (viewer-core),
`a_cancelled_prompt_about_a_named_file_declines_it_under_its_purpose` (viewer-host),
`the_question_about_a_file_says_whether_it_opens_beside` (viewer-ffi), `tests/mask_pair_refusal.rs`.

## Driven under Xvfb, all three windows

`form.pdf` (a push button with its own `/AP`, an ImportData link to `data.fdf` whose `/APRef` names
`stamp` in `library.pdf`, AES-256 user password `abc`), alone, `form.pdf note.pdf` and
`note.pdf form.pdf` (the `/UseOutlines` application note). In each: the remote-documents question
named library.pdf, the prompt named library.pdf, a wrong password gave attempt 2 of 3, and `abc`
drew the stamp into the form (quorra shows it; the tab count stayed). Cancelling said the password
was not given and declined `/APRef /N`, quorra kept the button's own artwork, and a second import
asked afresh.

## Left

- `doc/todo/38` has no buildable item: the gestures await the owner's mockups, the descriptor route a
  sender (ADR 1316), and an assembling verb needs pages read through the edit log first.
- GTK and Qt draw a native button labelled with the field's name over a push button, so neither its
  `/AP` nor an imported one is visible there; quorra draws both.
