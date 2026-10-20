# 2137 — A record the window cannot answer yet is declined in the library's terms, and said

Session 1650, RFC 0010 step N3a. Status: **accepted**, in `hosts/windows/Quorra.App/MainWindow.xaml.cs`;
compiled by CI only (ADR 2136). Context: ADRs 2088, 2121, `CLAUDE.md` principle 1, trap 5.

## Decision

Five owed records wait for an answer, and the first step of the WinUI window has no dialogue for
any of them. Each is answered with the refusal the header already defines, and the sentence is put
on the status line and the log:

| record | answered with | what the reader sees |
|---|---|---|
| `PASSWORD` | `quorra_window_password(document, NULL)`, "a prompt closed without one" | the prompt's text, and that the document stays closed |
| `QUESTION` | `quorra_window_answer(false)` | the reasons, and "declined" |
| `ARCHIVE` | `quorra_window_archive(-1, -1)`, "a chooser closed with nothing chosen" | the chooser's text, and that nothing was chosen |
| `SCRIPT` | `QUORRA_SCRIPT_ANSWER_UNANSWERABLE`, "a caller with no dialogue" | the dialogue's title, and that it could not be answered |
| `PRINT` | `quorra_window_print_finish` | the grant's sentence, and that there is no printer yet |

The records that only say "again" for something this window does not show — the panels, the
controls, the popups, the strip, the chrome, a placement, the system sound — and any kind this build
has no arm for are named in the log the first time each is owed, by `quorra_owed_kind_name`.

## Why

Leaving a record unanswered leaves the library waiting on a card nobody sees: a link's question at
the `ask` level would hang the act, a password prompt would leave a document neither open nor
refused. Declining is the safe direction for each (nothing leaves the machine, nothing opens), and
the header's own refusals keep it inside the ABI's meaning rather than inventing one. Saying it is
trap 5's rule: the window is partly built, and the part not built stays loud. Each later round of
N3a replaces one row of the table with its dialogue.
