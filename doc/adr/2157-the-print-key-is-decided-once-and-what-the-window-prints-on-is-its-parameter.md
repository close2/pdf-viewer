# 2157 — The print key is decided once, and what a window prints on is its parameter

Session 1660, batch one hundred and two's slot 6, a host round. Status: **accepted** and **built**.
Context: ADRs 1180 (the print and its preview), 2092 (a press decided once), 2121 (the print reaches
a C caller) and habits 131 and 139.

## 1. What was copied

ADR 2121 section 3 named it: "start on the unknown sheet, or end what stands" was written in
`quorra`, `viewer-qt` and `viewer-ffi`'s window, each with its own state for a print that stands
and the same sentence for its end. `viewer-gtk` made the fourth decision its own way — ask on its
dialogue's sheet, and never end one from the key, since `GtkPrintOperation::run` ends the job.

## 2. Decisions

1. **`viewer_host::print_key(Printer) -> PrintKey`**, beside `decide` in `keys.rs`: `Ask(Command)`
   — §7.6.4.2's bit 3 asked on a sheet — or `End(PRINT_ENDED)`.
2. **The toolkit's part is the parameter** (habit 139): `Printer::Preview { standing }` for a window
   with no printer, which asks on Table 193's unknown sheet and ends what stands; `Printer::Dialogue
   (sheet)` for a toolkit's dialogue, which asks on the dialogue's default page setup whatever
   stands. Not a `bool`, for `Keyboard`'s reason: what a window prints on is a fact about its
   toolkit, and the next host states one rather than writing a branch.
3. **What a window keeps is how it lets go of a job**: `quorra` clears a flag, `viewer-qt`'s
   `print_finish` clears the job it hands C++, `quorra_window` the pages it owed. Each sends
   `Printing::Finish` itself.

## 3. What it costs

One enum of two variants and one of two; four call sites. The messages and sentences are the ones
each window sent and said before, which the drive's print step and `drive_a_window.c` judge.
