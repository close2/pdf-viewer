# 2092 — A press is decided once in `viewer-host`, and a window keeps what its toolkit does

Session 1628, batch ninety-eight's slot 6, a host round. Status: **accepted** and **built**.
Context: ADRs 0526 (one key table for every window), 1357 (Space and Enter press a focused
push-button), 2072 (the key table crosses to C), habit 131.

## 1. What was copied

`viewer_host::meaning` is the one key table, but four hosts wrapped it in the same decisions of
their own: `viewer_host::pressed` asked first and its refusal said, the markup keys of §12.5.6.10
answered with "select some text first — §12.5.6.10's markups mark up text" where nothing is
selected, Table 29's mode read off `Presenting::full_screen`, and `WindowAct::ScrollBy`'s logical
distance multiplied into `Command::Scroll`. `viewer-gtk`, `viewer-qt` and `quorra` each wrote these
beside the table; `viewer-ffi`'s `keyboard::decide` was the fourth copy and the most complete.

## 2. Decisions

1. **`viewer_host::decide` answers a `Decided`** — `Activate { name, annotation }`, `Send`,
   `Window`, `Say` and `Nothing` — in the windows' order: the focused push-button, then the table,
   then the selection check before a markup. `SELECT_FIRST` is its sentence. The four hosts match
   on it; `viewer-ffi` maps it onto its acts as before.
2. **`Activate` is a variant of its own, not a `Send(Command::Activate)`**: `viewer-gtk` traces the
   field it pressed by name, and the message carries only the widget.
3. **`Keyboard::Page` or `Keyboard::Control` is a parameter.** GTK can pass the window an Enter one
   of its placed controls did not use; that key has been the control's and does not press the
   page's focused button as well. Which widget has the keyboard stays the toolkit's to know.
4. **`Mode::of(&Presenting)` and `scroll_by(by, scale)`** state the other two. GTK multiplied the
   distance by its whole-number scale in `f64` and narrowed the product; a scale of one or two is
   exact in `f32`, its product with an `f32` is exact in `f64`, and narrowing that once is the
   correctly rounded `f32` product — the same bits as the `f32` multiplication the other two
   windows used, which GTK now does as well.
5. **What a host keeps is its toolkit's**: the keyboard handed to where §12.5.1's tab walk landed
   (`follow_focus`, `aim_at_focus`), the view it scrolls, the presentation it shows.
   `NativeWindow::has_selection` and the hosts' own `press` and `has_selection` are deleted, since
   nothing asks them now.

## 3. What it costs

One more parameter at four call sites, and one enum whose variants a C caller never sees. The
order of a press is now read in one place, and a fifth host — RFC 0010's .NET window — calls it
rather than copying it.
