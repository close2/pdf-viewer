# 1192 — A Control this program does not bind means nothing

Session 1177. Status: **accepted**. Amends [ADR 0526](0526-what-a-key-means-stated-once.md)'s
decision that the key table reads one modifier, and settles the residue
[ADR 1180](1180-a-print-operation-has-two-ends-and-only-one-window-has-a-printer.md) handed over.

## 1. The finding, and why nothing failed while it stood

`viewer_host::keys::meaning` took a `shift` and nothing else. The module said why, and the sentence
was a reasonable one:

> A Control held down changes no row here — by the time a press reaches the page, the widget that
> would have wanted Ctrl + C has already had it.

That is true of a widget with the keyboard. It is not true of the **page**, which no widget owns.
All three hosts read their toolkit's modifier state, kept the Shift bit and threw the rest away
before calling the table — so a press on the page with Control held down was answered as though it
had not been. Ctrl + P entered §12.4.4's presentation. Ctrl + C copied, by coincidence: `c` alone is
§14.8.2.5's copy in this table, so the conventional key worked for the conventional reason having
nothing to do with the modifier. Ctrl + S saved for the same accident. Ctrl + T armed §12.5.6.6's
free-text drag, and Ctrl + X, which this program does not bind at all, did whatever bare `x` did —
nothing, today, and whatever the next letter bound does tomorrow.

Nothing failed, because there was no instrument that could see it. `Key::ALL` is walked by three
hosts and checks that every key is *translated*; no test asked what a modifier meant, because the
table had no way to be told about one. ADR 1180's round found it and handed it over rather than
fixing it, which is why it is here as a decision rather than as a patch.

## 2. The two halves, and the second one is the point

**A modifier this program binds gets its conventional meaning.** `ctrl_meaning` is a table of its
own, and every row in it is an operation this program *already performs*, on the key a desktop uses
for it: §7.5.6's save, §14.8.2.5's copy, RFC 0004's print job and the find bar. Nothing was invented
to fill it — there is no Ctrl + O, because no window in this tree opens a second document, each
being given a file on its command line, and a binding for a verb that does not exist is a key that
appears to do nothing.

**A modifier this program does not bind means nothing at all.** This is the half worth a decision.
A Control the table has no row for is a press addressed to something else — a window manager, a
toolkit accelerator, a shortcut a desktop added after this program was compiled — and a window that
answered it with the unmodified binding is acting on a keystroke aimed past it. So `ctrl_meaning`
returns `None` rather than falling through, and
`a_control_this_table_does_not_bind_falls_through_to_nothing` walks every key in `Key::ALL` in both
modes and both waiting states to say so.

The same argument decides what does *not* cross. Alt, Meta and the platform key are not folded into
`ctrl` in any host: a modifier passed off as one it is not would bind a key this program was not
given.

## 3. Why a value and not a second `bool`

`meaning(key, false, false, mode, waiting)` at a call site says nothing about which `false` is
which, and that is the reason `Mode` and `Waiting` are already enumerations rather than flags. The
two modifiers also do different things — `shift` chooses between two rows of one table, `ctrl`
chooses a different table — so `Modifiers { shift, ctrl }` is what crosses, with `NONE`, `SHIFT` and
`CTRL` for the call sites that hold one.

Shift is deliberately **not** read inside `ctrl_meaning`, so Ctrl + Shift + P prints. A person
holding a third key down has not asked for a fifth meaning, and the shifted rows of the unmodified
table are about *direction* — §12.5.1's tab key — which none of these four has.

## 4. What each host owed, and where its test is

Each host translates its toolkit's modifier state, and that translation is now a named function at a
seam a test can reach without a display, because a bit read wrongly inside an event closure is
exactly the defect this ADR is about:

| host | the seam | what it is |
|---|---|---|
| `viewer-gtk` | `modifiers(gdk::ModifierType)` | a set of bits; nothing calls into GTK |
| `viewer-ui` | `modifiers_held(winit::keyboard::ModifiersState)` | the same, for winit |
| `viewer-qt` | `Host::key(code, shift, ctrl)` | C++ reads `Qt::ShiftModifier` and `Qt::ControlModifier` at the event and the bridge carries both |

The first two are asserted directly: each modifier crosses, both together cross, and Alt or the
platform key crosses as neither. Qt's seam is the method, so its test is the whole chain —
`a_control_this_program_does_not_bind_leaves_the_document_alone` presses `h` with Control and
without it on a real document with the restrictions turned off, and only one of the two annotates.

## 5. Shift and P is kept

ADR 1180 chose Shift + P for printing because `p` was already §12.4.4's presentation and Shift was
the only modifier the table read. Ctrl + P now works, and Shift + P still does. A table three
windows agree about is not improved by taking a binding away from the people already using it, and
the argument that put it there — that the letter a job is named after was spoken for — is unchanged
by a second key reaching the same job.
