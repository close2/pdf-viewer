# 1605 — The drive waits for the window's own word, with a ceiling, rather than for a fixed time

Session 1384. Status: **accepted**. Amends ADR 1453's drive (its `launch` and its input helpers);
the steps and what each asserts are unchanged. Builds on ADR 1600's time column.
Context: ADRs 1453, 1478, 1600; `doc/history/1382-*.md`, which measured where the drive's time goes.
Code: `tools/drive-windows.sh` (`wait_for`, `said_since`, `first_frame`, `a_window`, `launch`,
`key_then`, `type_then`, `click_then`, `expect_title`, `asked`, the step bodies).

## 1. The shape: a condition, a ceiling, and no verdict of its own

`wait_for SECONDS COMMAND…` runs the command every tenth of a second until it succeeds, and gives up
after the ceiling with one line on standard error naming what it waited for. It judges nothing: a
ceiling reached is followed by the step's own check, which says `wrong` with what it saw, so a wait
can make the drive slower but never changes a verdict. The conditions are the window's own words:

- **`launch`** waits for the line each window prints under `--trace=launch` once its first frame is
  on the screen — `quorra`'s `launch path, process start to first present`, the two toolkits'
  `first frame on the screen at`, `quorra-confined`'s `first frame presented` — and for the window to
  be mapped. Ceiling 30 s. A launch whose first frame waits behind §7.6.4's password waits for any
  window instead. `launch` was `sleep 5`, in every one of about a hundred launches.
- **An input whose effect the core traces** — a Tab (`Focused(Next)`, `focus Next annotation`), a
  value typed (`SetField { field: "A", value: Text("1")`), a save (`saved N bytes to`), a search's
  end, a refusal the step judges — waits for that line after the log's length before the input.
  Ceiling 10 s.
- **A title** is waited for until it holds what the step expects; **an AT-SPI node** is asked until
  it is published (the toolkits' panel rows, a document's form node, a dialogue's button, the
  reopened form's values, a frame's active state); **a photograph** is retaken until its colour or
  its page is on it. Ceilings 10 to 20 s, and 120 s for the processor's page under a device refusal.

## 2. What still sleeps, and why

An input the window says nothing about keeps its settle: `key` and `type_in` 1.2 s, `click` and
`wheel` 1.3 s, because GTK and Qt move the keyboard and map popups from their event loops after the
handler that traced the input has returned. A tenth of a second after a traced Tab, and two tenths
after AT-SPI's click on a field, for the same reason. One second after the password prompt maps, for
its entry to take the keyboard. Two seconds before photographing Bob's page in `36-reader-name`,
because what is judged there is an absence, which no condition can wait for. One second after the
zoom keys and the zoom wheel, and after `r`, for a frame or a menu that says nothing to land on the
photograph.

## 3. Measured

The whole drive behind the heavy-walk lock, before on the batch's opening binaries and script and
after on this round's: 141 works, 0 wrong, 3 not offered, 1129 s over the verdicts' time column and
1135 s of wall clock, became 143 works (the two are ADR 1604's `39-field-shown`), 0 wrong, 3 not
offered, 440 s and 442 s. By group, every window together: `23` 82.5 to 29.5 s, `33` 74.9 to 18.0 s,
`27` 13.4 to 5.2 s, `25` 148.1 to 72.8 s, `37` 90.5 to 29.8 s; no ceiling was reached. What is left
is mostly the settles of section 2 — `25`'s four typed words and `07`'s find are their keys — and
`27-processor-fallback`, which is the window drawing `drive-coverage.pdf` on the processor.
