# ADR 0604 — The way out two windows named and did not have

Status: accepted, 2026-08-25. Session 721. The debt ADR 0603's reading found on the row nobody had
been asked to read, and closed in the same round.

`CLAUDE.md` states one thing about a document's restrictions without qualification:

> **They are low priority**, and **it shall always be possible to turn them off.** A restriction a
> reader cannot switch off is a restriction imposed on the reader by somebody else's file, and this
> program is the reader's.

It was possible in one host of three. The other two said it was possible and then refused.

## 1. What was there

`viewer_core::Command::Restrict(RestrictionLevel)` is the policy value ADR 0212 built, sent once
before a document opens because a policy applied halfway through is not a policy. `viewer-ui` has
supplied it from `--ignore-restrictions` since that session.

Both native hosts answered `Event::Refused` with this, each having written it for itself:

```rust
Event::Refused { notes, .. } => self.say(&format!(
    "{} — this reader is obeying that; --ignore-restrictions turns it off",
    notes.join("; ")
)),
```

and both argument parsers end with

```rust
} else if word.starts_with("--") {
    return Err(format!("{word} is not an option this program has"));
```

So `pdf-viewer-gtk --ignore-restrictions x.pdf` printed *"--ignore-restrictions is not an option
this program has"* and exited 1, and a person who hit a refusal in either native host was told the
way out and then refused it. Nothing anywhere sent `Command::Restrict` in either host, so
`RestrictionLevel::On` — the core's default, correctly — was the only level either window could ever
be in.

## 2. Why it survived, and what found it

**It is invisible from inside each host.** The sentence is right about what it means; the parser is
right about words it does not know; nothing reads both. The two hosts agreed with each other
perfectly, which is the failure `viewer-host` exists to prevent and which it could not prevent here
because what was copied was a *string literal* rather than a decision.

**And the one instrument that could see it was reporting the opposite.** `tools/state.sh windows`
answered `every Command reaches at least one window`, on the strength of `viewer-ui`'s trace
formatter and a `PathCommand::Close` in a chrome file (ADR 0603 §2). With the count made honest,
`Command::Restrict` appeared on both native hosts' missing lists — and *that* is what a reading has
to be done on, because a variant a host does not send is sometimes a delegation and sometimes this.

Two smaller things it did not survive on:

- `doc/todo/38`'s "**No user interface**, by the owner's instruction, until it is asked for" is about
  a *menu with four levels*. A command-line flag is what `viewer-ui` already had and is not one, so
  nothing here was waiting on permission.
- The restriction clauses are not what was missing. §7.6.4.2's Table 22 and §12.8.2.2's `/DocMDP`
  are read, composed and refused with the right sentences by `pdf-model`; what was missing was a
  *host* able to say no to them.

## 3. The decision

**One word and one sentence, in `viewer_host::policy`, and both native hosts take the word.**

- `viewer_host::IGNORE_RESTRICTIONS` is the flag's spelling. All three parsers compare against it.
- `viewer_host::refused(notes)` is the sentence, and it names the constant by interpolation, so a
  window cannot print a word that is not the word a parser takes.
- `pdf-viewer-gtk` and `pdf-viewer-qt` carry the level to `Host::open`, and `open_document` sends
  `Command::Restrict` at the head of the same queue `Command::Delegate` already went in — both are
  policy, both go before the document, for one reason.
- `viewer-ui` adopted both, losing its third copy of the sentence.

**They stay in one module** rather than the constant living with the arguments and the sentence with
the status bar, and that is the whole decision rather than tidiness: apart is exactly how they came
to disagree, and `viewer_host::status` holds three other sentences a window says without any of them
naming a word a person has to type.

## 4. What checks it, and why it takes three tests rather than one

Neither end alone would have caught this. The sentences agreed with each other and with nothing
else; the parsers agreed with each other and with nothing else.

| test | where | what it holds |
|---|---|---|
| `the_refusal_names_the_word_that_turns_the_restrictions_off` | `viewer-host` | the **sentence** names the constant |
| `the_word_the_refusal_names_turns_the_restrictions_off` | each native binary | the **parser** takes the constant |
| `a_restricted_document_refuses_and_the_reader_can_turn_that_off` | `viewer-qt` | the **chain**, end to end |

The third is the one worth the words. `viewer-qt`'s `Host` is a plain struct C++ happens to own, so
a test builds one with no display and no `QApplication` (ADR 0246's ownership inversion paying a
dividend), opens `issue17215.pdf` — one of the seven corpus witnesses ADR 0212 measured, which opens
on the empty user password and withholds both filling a form and annotating — presses `a` and then
`h`, and reads the window's own status line back. Under `On` it contains the flag; under `Off` it
does not and the document is dirty. It is the only test in this tree that walks the whole path a
person walks, and the defect lived exactly between two links that each had a test.

All three were run against injected defects before being believed (trap 13): the `Command::Restrict`
line removed from `open_document`, the constant removed from the sentence, and the flag's arm removed
from the parser — each fails the test that is about it and no other.

Driven under `Xvfb` as well, on the same document: §5's `pdf-viewer-gtk`, `a` then `h`, and the
status bar carries the refusal while the title has no dirty mark; the same run with the flag on the
command line refuses nothing and the title gains its `•`.

**And the photograph found the same defect one layer out, which is the reason to take one.** The
GTK label is `EllipsizeMode::End`, so the longest sentence this window says loses its tail — and the
tail of the longest sentence is the way out: the screen read *"… — it was not done — this reader
i…"*. Qt's `QLabel` in a status bar is clipped by the window rather than elided, identically. Both
set a tooltip carrying the whole text now, which is each toolkit's own idiom for an elided label and
costs nothing when the text fits.

**What is *not* claimed about that fix is that it was photographed.** There is no window manager on
this machine (`doc/environment.md`), a GTK tooltip is a timer and a crossing event away from a
pointer this environment cannot deliver convincingly, and `xwd` hands back what the window last
painted — so what was checked is the call and the text handed to it, not a picture of the tooltip.
Saying so is cheaper than a screenshot that would have proved nothing.

## 5. What this does not do

It adds no level. The project owner's four are still two, `Event::Refused` still carries the
operation so that it can become a question, and `doc/todo/38`'s *ask* and *warn* are unshipped for
the reason that file gives — a variant nothing produces and nothing answers is a level that silently
behaves like another one.

It adds no menu. When the owner asks for one, this is the value it will set, in the place all three
hosts already read it from.

And it changes nothing about what a save may assert. `RestrictionLevel` reaches `Viewer::refusal`
and nothing else; §12.8.2.3's `/UR3` withdrawal is still reached from `ViewState::save` with no
policy in scope at all. Turning a restriction off is the reader's; making the file lie is not.
