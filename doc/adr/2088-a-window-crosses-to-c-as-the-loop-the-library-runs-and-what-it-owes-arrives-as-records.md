# 2088 — A window crosses to C as the loop the library runs, and what it owes arrives as records

Session 1626, the first of RFC 0010 section 6's two N2 rounds, for the .NET host of
`doc/questions/A287`. Status: **accepted** and **built** for the part section 4 names. Context: ADRs
0247 (the C ABI, and why it refused callbacks), 0246 (`viewer-qt`'s update flags), 1920 and 1972
(`NativeWindow`), 2048 (the level-hosts rule), 2072 (the key table), 2092 (a press decided once).

## 1. What was missing

Through N1 a C caller could send every `Command` and translate every key, and still had to run the
loop itself: which event opens a dialogue, when the drawing thread is looked at, what goes in the
title, when page one's look stands in for a frame. `viewer-gtk` and `viewer-qt` answer those alike
because `viewer_host::native_window::NativeWindow` holds the answers once; a C# window that wrote
them again would be the copy `doc/todo/30` warns of. RFC 0010 section 3.2 proposed a second layer
that implements the trait from Rust, its required methods a table of C function pointers.

## 2. Decisions

1. **`quorra_window` is `NativeWindow` implemented in `viewer-ffi`** (`src/window.rs`): a
   `WindowState`, opened on a thread of its own by path and joined at the first resize, every page
   drawn on the drawing thread, a tick at the interval the window states, as `viewer-qt` runs them.
2. **What a required method owes the caller is a record, not a callback.** ADR 0247 section 2
   refused callbacks because a caller answering one can call back into the viewer while it is
   borrowed. That holds here twice: `say` is called from inside `drain`, while `&mut Window` is
   held, and in C# an exception that crosses an `[UnmanagedCallersOnly]` method ends the process.
   So each toolkit-shaped method leaves an `Owed` in the window's `Owing`, and the caller takes the
   batch (`quorra_window_take_owed`) after the entry point returns: ten `QUORRA_OWED_*` kinds,
   counted against `QUORRA_OWED_KIND_COUNT` beside `quorra_abi_check`, whose signature is frozen,
   and thirty-two entry points.
   This is `viewer-qt`'s shape, whose C++ side reads flags because Rust never calls a Qt object. A
   record that only says "again" (repaint, panels, chrome, placement) is kept once per batch, and a
   later title replaces the earlier where it stood. RFC 0010 gains section 3.2a saying so.
3. **A sentence is owed and not printed.** The toolkit windows print each on standard output; a
   library's caller decides where its sentences go (RFC 0010 section 8, question 3).
4. **A key is decided by `viewer_host::decide`** (ADR 2092), and the window carries out the acts
   the trait carries out for every window — scroll, copy, measure, separations, the next layout,
   the archive key, abandoning a long draw. The rest come back as the `QUORRA_ACT_*` `quorra_key_press`
   returns, because each is a widget: the find bar, the panel, a dialogue, the tab strip.
5. **The pointer, the find bar and the strip are the window's**: a pointer is a `Command::Pointer`
   unless §12.9's measuring takes the press, and answers whether it is over §12.5.6.5's region; a
   search steps a page per tick at a zero interval, as `viewer-qt`'s zero-length timer steps it; a
   file chooser's answer opens beside, and a close that would close the last document answers so
   and closes nothing, since the window is the caller's.
6. **The one question at the *ask* level is one record and one answer** (`quorra_window_answer`),
   for the eight subjects `viewer-qt` puts in its one dialogue; §7.6.4.1's prompt is another, with
   its document, answered by `quorra_window_password`. The decisions are `viewer_host`'s; what
   `window/asked.rs` holds is the carrying-out, which each toolkit window also holds.
7. **`QUORRA_UNREADABLE` (9)** is the status for a path that cannot be opened: the window reads its
   document by path, as every window does, because §12.6.4.3's and §12.7.6.4's files resolve
   against its directory.

## 3. Considered and refused

- **Callbacks, as the RFC proposed.** Refused for decision 2's reason; the RFC's price, "about 30
  callbacks", becomes ten record kinds and the batch's seven entry points.
- **A `quorra_viewer *` lent out of the window**, so that the vocabulary layer's entry points act
  on the window's viewer. A `Session` owns its `Viewer` by value and the window's is inside
  `WindowState`; lending one would be a second owner, or a viewer swapped out for each call.

## 4. Costs, and what is not built

- **Three acts are declined aloud rather than carried out**, each with its sentence (trap 5): the
  archive chooser, a print grant (ended with `Printing::Finish` so §8.11.4.5's intent does not
  stand), and a script's `app.alert`, answered as a dialogue closed unanswered. A window opens at
  `viewer_host::Settings`' defaults with the reader's words unsaid; the policy flags cross with
  RFC 0010 section 5's parser.
- **Not built, and RFC 0010's second N2 round**: the panels' rows, the controls placed over the
  page and their commits, the find bar's matches and highlights, the selection's quadrilaterals,
  and the three acts above.
- The carrying-out of a document's requests is a third copy beside `viewer-gtk`'s and
  `viewer-qt`'s, as the RFC priced it; it belongs in `viewer-host` with the rest of ADR 2092's move.
