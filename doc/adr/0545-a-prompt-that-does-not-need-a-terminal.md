# ADR 0545 — A prompt that does not need a terminal, and a password that does not print

Status: accepted, 2026-08-23. Session 695, the **fourth** round on the project owner's *"even
though low priority, I think we should start investing time into the UI (and its API for the
native versions)"*, taking **item 3** of the ordering ADR 0509 wrote and `doc/todo/30` carries.

Adds `viewer_core::Secret` and `viewer_host::password`; replaces `viewer-ui`'s terminal prompt with
a modal card it draws for itself; corrects a quotation two hosts carried for two hundred and
eighty-five sessions; and closes a leak that was averted by a struct's field order.

## 1. The item, and why it was third

ADR 0509's criterion ranks *what a reader can do with a document and cannot do here* first.
`viewer-ui` — the tier-2 host, the one that is *ahead* of the two native hosts on everything that
reads a document — answered an encrypted file like this:

```rust
eprint!("{name} needs a password (empty line to give up): ");
std::io::stderr().flush().ok()?;
std::io::stdin().read_line(&mut line).ok()?;
```

and, where that produced nothing:

```rust
eprintln!("{}: needs a password", self.title);
std::process::exit(1);
```

So a window launched from a desktop launcher, a file manager or a `.desktop` entry could not open
an encrypted document **at all**. It is the only place in this tree where the program answered a
document on a file descriptor, and the only one that left the process for want of one.

## 2. What the standard says, which is not what two hosts were quoting

ISO 32000-2 §7.6.4.1:

> If this authentication attempt fails, the interactive PDF processor should prompt for a password.
> Correctly supplying either password ( owner or user password) should enable the user to gain
> access to the document.

**`should`, and `interactive`.** Both native hosts carried this comment, in quotation marks:

```rust
// §7.6.4.1: "the interactive PDF processor shall … prompt the user for a password".
```

The standard does not contain that sentence. A `should` was written as a `shall`, four words were
added, and `viewer_host::policy`'s module documentation said the clause "requires" it. `CLAUDE.md`
is explicit that quotation marks mean verbatim — and **the quotation gate could not see this**,
because it reads rustdoc blockquotes under a citation and these were `//` line comments. That is
worth knowing as a limit of the instrument rather than as a fault in it: the gate checks what it
was built to check, and a `//` comment with quotation marks in it is outside its population.

The clause also says what a processor with nobody to ask is, and it is not one that gives up.
NOTE 2:

> This enables limited access to a document when a user is not be able to respond to a prompt for a
> password. For example, there can be non-interactive PDF readers that do not have a person running
> them such as printing off-line or on a server.

A window on a screen is an **interactive** processor whatever it was launched from. "There is no
terminal" is therefore not the case the NOTE describes — it is a processor that has a person and
looked for them in the wrong place. That is the whole argument for this round, and it is the
standard's rather than a preference about desktop integration.

The clause states **no number of attempts** and nothing about what a prompt looks like. Both stay
documented choices, and they now live in one place.

## 3. `viewer_host::password` — the policy, once

Three hosts held three copies of `const PASSWORD_ATTEMPTS: u32 = 3`, three `attempts` counters,
three `saturating_add`s and three comparisons — and the third copy is where two hosts stop
agreeing, which is `viewer_host::keys`' argument (ADR 0526) and `presentation`'s and `clock`'s
before it. They had already diverged in the way that matters: the two native hosts said a sentence
and left the window up, and `viewer-ui` counted to the same three and *exited*.

What is in the module:

- `Asking` — the counter, with `required()` driven from `Event::PasswordRequired` and `opened()`
  from `Event::Opened`. **Neither native host reset the count**; it did not show because each opens
  one document and never another, and Annex O's `ef` already gives this program a second one to
  open without restarting. All three reset now.
- `Ask` — a closed two-case enumeration, matched exhaustively in all three hosts.
- `Supplied` — likewise, and it carries the decision that **an empty entry is a decline and not the
  default user password**. That is not a style choice: §7.6.4.1's default user password is the
  empty string and the reader has already tried it by the time this event exists, so sending it
  would spend an attempt on the answer that already failed.
- `prompt(name, attempt, of) -> Wording` — one format string for three hosts. Each of them built
  the question for itself before this round; `viewer-qt`'s was a `QStringLiteral` that did not name
  the file at all.
- `ATTEMPTS`, `EXHAUSTED`, `CANCELLED` — the number and the two sentences.

**What a host still owns is everything with a pixel in it, and one thing without: it may not make
`Ask::Exhausted` mean *close the window*.** That is stated in the module rather than left implicit,
because it is the exact thing `viewer-ui` used to do.

## 4. The card

`viewer_ui::chrome::PasswordCard` is the tier-2 counterpart of `gtk4::PasswordEntry` and a
`QLineEdit` at `QLineEdit::Password`: a modal card over the page, drawn in `pdf-font`'s compiled-in
Helvetica at the identity transform, exactly as the About card and the find bar are. Every number
in it is this host's own and is written down as a choice, because the clause describes no window.

The echo is §12.7.5.3's, which is the one place the standard says what an unreadable echo looks
like — "such as asterisks or bullet characters" — so the card takes the second of the two examples
rather than inventing a third. It is drawn from the *count* of characters; `Secret::reveal` is
called nowhere in the module.

### The defect this round nearly repeated, and it is 687's

687 found that every overlay `viewer-ui` draws was unreachable on the graphics-device path, because
the frame comparison looked at the pages and not the chrome. This round's chrome had a **third**
way not to reach the screen, and it is sharper: `App::present` opened with

```rust
let pages = self.arrangement(edge, width, height);
let first = pages.first()?;
```

so a window with **no page** drew no frame at all — and a document that has not authenticated is
precisely a window with no page. The card would have been built, held, and never presented, on
either surface.

`Surface::without_a_page` is the fix, and it is one method for both surfaces because there is
nothing to decide between them there: no page means no magnification to pick a coverage lane for,
no retained pixels to stand in with, and no view change to approximate. The device path asks the
window for a frame of overlays over nothing (`PresentFrame` already accepted `pages: &[]` — the
processor fallback uses that shape); the processor path composes them over
`viewer_ui::software::surround`, which rasterises an empty display list at `Medium::WINDOW` so that
ADR 0446's one statement of *the surface a document is laid on* stays the one place it is applied.

A window with no page **and no chrome ever drawn** still presents nothing, which is every tick
between the window appearing and the first frame landing and is unchanged — no blank frame goes up
in front of page one. Once this path *has* put chrome up, an empty list stops meaning *nothing to
show* and starts meaning *take it away*: without that distinction the prompt stayed on the window
after the program had said it was done asking, which the screen said and no test could.

**Two more things only the screen could say, and both were in the new method.** It did not
`adopt` the frame the render thread had finished, so every tick asked for a frame nobody collected
and reported *nothing to show* for ever — 1196 ticks in twenty seconds of a window that should have
drawn once. And it armed the frame clock unconditionally rather than only where it had asked for a
frame, so once that was fixed the window presented at the tick rate for ever, which is
`doc/todo/36`'s fourth rule broken by one line. Both are the shape 687's finding had: a method
written beside two that already do it right, missing the line that made them right. Eleven frames
and one present, after.

## 4a. Two things the toolkits owed, found by pressing the keys

**`GtkWindow::close` fires `close-request` synchronously.** The prompt's Open button set a flag on
the host and *then* closed the dialogue; the close handler — which exists so that a person who walks
away is told why the document is not on the screen — ran inside `close()`, read the flag before it
was set, and reported every supplied password as a decline. The flag lives beside the dialogue now,
in an `Rc<Cell<bool>>` the three closures share, and is set before the close rather than after it.

**A plain `GtkWindow` binds no key at all.** `GtkDialog` binds Escape and is deprecated in the
release this crate targets, so without an `EventControllerKey` the only way out of the GTK prompt
was the window manager's close button — and a reader with a keyboard could not decline. Under
`Xvfb` there is no window manager to have hidden it: the other two hosts declined on Escape and
this one did nothing. This is ADR 0508's rule paying for itself in the other direction — the check
was one key press.

## 5. The security surface, and what was decided about each part

`CLAUDE.md` principle 3 asks that the password not reach a log, a trace, an error message, a core
dump or a window title. Four decisions:

**The password is a type, `viewer_core::Secret`, and `Command::Open` carries one.** This is the
finding rather than a precaution. `Command` derives `Debug`, and `viewer-gtk` and `viewer-qt` trace
a command with `format!("{command:?}")` truncated to 120 characters — so what stood between a
reader's password and a launch log on disk was the **field order of a struct variant**: `bytes` is
declared before `password`, a `Vec<u8>`'s `Debug` is about five characters a byte, and the
truncation therefore cut the line before the secret. That is an accident, not a security property.
A variant reordered, a truncation widened, or a document of twenty bytes would each have undone it
in silence. `Secret`'s `Debug` says how many characters there are and not one of them; there is no
`Display` and no `AsRef<str>`, and the one way to read it is `reveal`, named so that a reader of a
call site sees the moment it happens. It is called in exactly three places in the tree: the open in
`viewer-core`, the encoder in `viewer-confined`, and a test.

**Every consumer failed to compile.** `viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-ffi`,
`viewer-confined` and two censuses, which is the fifth application of the mechanism ADRs 0166,
0167, 0247 and 0248 established.

**It is zeroed on drop, best effort, and the honest limit is written down.** `Drop` takes the
`String` out, converts it to its bytes — `String::into_bytes` is the same allocation, so this
clears the buffer the password was typed into rather than a copy — fills it with zeroes, and passes
it to `std::hint::black_box` so the write is not dead-store-eliminated. `black_box` promises no more
than opacity to the optimiser: a value the compiler kept in a register is not this buffer, and a
page swapped to disk was never ours to clear.

**`zeroize` was considered and not taken**, and the reason is principle 3 rather than dependency
count: `Zeroizing` writes through a volatile pointer, a volatile write is `unsafe`, both
`viewer-core` and `viewer-host` carry `#![forbid(unsafe_code)]` because they touch PDF bytes, and a
dependency added to reach an `unsafe` primitive is that rule paid off in another crate's ledger.
What is recorded is the weaker guarantee, honestly, rather than a stronger one bought elsewhere.

**The buffer is sized from the standard.** A `String` that reallocates while a person types leaves
the bytes typed so far in freed memory that nothing can reach to clear. §7.6.4.1 truncates a
revision 6 password "to the first 127 bytes if the string is longer than 127 bytes", so `Secret::new`
reserves 128 — the number is where the standard stops reading, and a password that can affect the
outcome therefore never moves its buffer. Tested.

**What is not ours is named rather than glossed.** GTK's `GtkEntryBuffer` and Qt's `QLineEdit` hold
their own storage in glib's and Qt's heaps; both hosts empty the widget on the way past, which is
the part a host can reach, and the rest is said out loud here. The confined worker receives the
password over its pipe because it is the process that decrypts, which is inherent and unchanged.
Nothing writes it to a window title, a file or a trace.

**One gate went blind and got a replacement.** `viewer-confined`'s `every_carried_command_round_trips`
compares `Debug` strings, so with a redacting `Secret` it can no longer see a transport that
corrupted a password into another of the same length. `a_password_crosses_the_transport_unchanged`
is the one field's own test.

## 6. The instrument, and an honest word about its strength

ADR 0509's third criterion asks for something that makes the level-hosts decision *checkable*.
What this round has is the compile-time half of 687's shape: `Ask` and `Supplied` are closed, not
`#[non_exhaustive]`, and matched exhaustively in all three hosts, so a case added to `viewer-host`
fails to compile in three places. The shared constants and `prompt` do the rest — a host cannot
re-word the question without deleting a call.

**It is weaker than `Key::ALL`'s walk and the reason is worth stating rather than dressing up.**
That test has teeth because a key table is thirty rows that can each be mistranslated, so there is
a *runtime translation* to assert agreement with. A prompt is one event and two outcomes; there is
no translation to check, and a per-host test walking two variants onto two handler names would be
ceremony. What replaces it is the driven run below, which is the instrument that would actually
have caught the defect this round fixed.

## 7. What was not done, and why

**`Command::Open`'s password did not become an `Option<&Secret>`** — an owned value is what a host
builds and hands over, and a borrow would put the buffer's lifetime in the host rather than in the
command that is dropped with it.

**The four levels `CLAUDE.md` asks for do not apply here.** A password prompt is not one of the
document's restrictions over its reader; it is authentication. §7.6.4.1's `/P` flags *are* such a
restriction and are already `pdf_model::restriction`'s, under ADR 0212's policy.

**`viewer-ffi` gained no entry point.** A C caller supplies the password on `pdfv_open` and places
its own prompt, exactly as it owns its own keyboard (ADR 0526's finding about the fourth consumer).
What it did gain is that the `String` its NUL-terminated bytes became now **moves** into the
`Secret` rather than being copied.
