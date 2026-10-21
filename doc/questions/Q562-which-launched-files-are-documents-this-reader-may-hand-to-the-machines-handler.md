# Q562 — Which launched files are documents this reader may hand to the machine's handler?

Source: session 1667, ADR 2171, on the brief's ask to make a launched non-PDF a policy level a host
supplies, and to ask only if the reading leaves a choice the standard does not make. It leaves one.

## Why it needs the owner

ISO 32000-2 §12.6.4.6's Table 207 makes `/F` "[t]he application that shall be launched or the
document that shall be opened or printed", and nothing in the standard says how to tell the two
apart. ADR 1358 refuses every application, on principle 3. Handing a *document* to this machine's
own handler is the act `CLAUDE.md`'s clause-13 entry admits for a media file, at the reader's level,
and the round built that — but which file counts as a document is a decision about what this host
may cause to be started, and the handler (`xdg-open`) decides by type what it does with a file.

## What the tree does meanwhile

`--launched-documents=refuse|ask|warn|open`, **`refuse` by default**, so nothing is handed over until
a reader sets the word. A file is a **program**, refused at every level, where its permissions let it
be executed (or cannot be read), where it begins with an ELF, PE, Mach-O or `#!` signature, or where
its name ends in `appimage`, `bat`, `cmd`, `com`, `command`, `desktop`, `exe`, `jar`, `msi`, `ps1`,
`run`, `scr`, `sh`, `vbs` or `wsf`. Everything else is a document (`viewer_host::policy::launched`).

## The questions

1. **Is the classification acceptable as the line?** Recommendation: yes. It errs towards *program*
   on every test, so its failure is a document refused with a sentence. The alternative that would
   be stricter still is an allow-list of media types (by content sniffing), which costs a type
   detector this tree does not have.
2. **Is `refuse` the right default, or `ask`?** A254 made the media file `ask` by default; the brief
   for this round said `off`. Recommendation: keep `refuse` until the level has a menu entry in every
   window, then `ask`, which is A254's shape — a question no window can put is a refusal anyway.
