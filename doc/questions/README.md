# Questions for the project owner, and their answers

**The convention, amended by the owner on 2026-09-12.** Every question a round is waiting on the
owner for gets a file here whose name begins with **`Q`**. The owner answers with a file of **the
same name, `Q` replaced by `A`** — written by their hand, or transcribed from any medium by the
round that received the answer.

**An `A` file means the owner has spoken. It does not mean the question is settled.** Answers are
not all completions, and the 2026-09-04 rule — *a question with no matching `A` file is open; that
is the whole index* — was wrong exactly there: `A15` and `A17` said *on hold*, `A03` *declined for
now*, `A19` and `A49` *there is nothing to decide*, `A51` answered half of what was asked, `A05`
replaced its own earlier text, and `A46` ended two of those holds with nothing in this directory
saying so. So the index moved out of file existence and into the `A` file's **header**, three lines
the gate can check:

```
Status: complete
Given: 2026-09-12, in conversation — transcribed by the round
Owes: none
```

## The three header lines

**`Status:` says what the answer did to its question.** The vocabulary is closed, and every word
in it was already needed by a file in this directory:

| status | means | the file that needed it |
|---|---|---|
| `complete` | the question is settled; nothing waits on the owner | most of them (`A02`, `A48`, …) |
| `partial` | part of what was asked is answered; **the remainder is named in the body**, and if the remainder becomes its own question the body cites it | `A51` — part 1 bought, part 2 "a new question, asked then" |
| `deferred` | the owner declines to decide now; **the reopen condition is named in the body**. A round treats the stated default as standing and does not reopen it; only the owner does, by a new answer or a new `Q` | `A03` — "declined for now" |
| `void` | the question dissolved; there is nothing to answer | `A19` — "retired"; `A49` — "Void" |
| `superseded by Q##` | a later question's answer replaced this one, and the later `A` file is where the current word lives; the later file's body names what it replaced | `A15` and `A17` — their "on hold" is over by `A46`'s own sentence |

A question with no `A` file at all is still simply **open** — that half of the old rule survives,
because nothing else is as cheap for a round to see.

**`Given:` records whose word the body is and when it arrived.** The two sources look the same on
disk and are not:

- `by the owner's hand` — the owner wrote the file.
- `in conversation — transcribed by the round` — the owner answered in chat, in a call, anywhere,
  and the round wrote this file down. The date is the day the answer was given, not the day of
  the transcription.

**`Owes:` is the work the answer itself commands beyond the decision** — "document this in an
ADR", "provide a vacuum mode", "make the amendment". Three rules keep it honest:

- `Owes: none` when the answer decides and commands nothing. Most answers are this.
- An item is **removed by the round that does the work**, and that round's ADR cites the `A` file
  it answered — the link survives in the ADR, so removing the item loses nothing. Until then the
  item is this directory's statement that work is outstanding.
- An item the 2026-09-12 retrofit could not verify as done is suffixed `(retrofit: unverified)` —
  the honest name for a debt the old format could not track. The round or owner that knows the
  truth of one clears it.

## The body: the owner's word, and a round's reading, visibly separate

The body **quotes the owner verbatim** — a blockquote when the answer was transcribed, plain text
when it was written by hand; typos and all, in both cases. When the answer is "agree with the
recommendation", the sentences adopted are quoted **and visibly attributed**, and everything the
round adds — the consequences it draws, the phrasing it supplied — lives in a `Reading:` section
on the round's own account. A round writing its interpretation in the owner's voice is the
forgery this format exists to prevent; the failure it is drawn from is real, and is `A58` as first
written, which put "one amendment, not two" in the owner's mouth when the owner had ratified a
summary containing it.

## Rules for a round

- **Ask here, not in a report.** A question raised only in a history file or an ADR is a question
  nobody can find. If a round needs the owner's word, it writes a `Q` file in the same commit as
  the work that raised it.
- **One question per file**, numbered in the order they were asked. A number is never reused, and
  an answered question keeps its `Q` file beside the owner's `A` file, because the argument that
  raised it is worth as much as the answer.
- **The number comes from the round's own reserved block, never from `ls`.** Parallel rounds
  branch from trees that cannot see each other, so the highest number in this directory is a
  claim about one branch rather than about the project: on 2026-09-04 rounds 926 and 927 each
  took `Q27` by reading it, honestly, on the same day, and the two met in a merge three rounds
  later ([ADR 0908](../adr/0908-two-questions-called-q27.md); session 934 renumbered one of them
  to `Q35`, which is why the numbers here have a gap). The allocator is the block a round is
  given in its instruction — that is the one counter every round can see — and a gap between
  blocks is correct rather than a mistake to tidy.
- **A collision cannot reach `main` unnoticed**, which is the half the tree can enforce:
  `tools/conformance/tests/questions.rs` fails when two `Q` files share a number, when an `A`
  file answers no `Q`, when an `A` file's name is not its `Q` file's with the letter changed, when
  a filename here is not `<letter><number>-<slug>.md` — and, since this amendment, when an `A`
  file's header is missing, out of order, or names a status that is not in the vocabulary above.
  It runs in `cargo test -p conformance`, which is the last line of `doc/todo/02` §2's sequence
  and which every merge round runs.
- **Every `Q` file says four things**: the question, why it cannot be settled without the owner,
  **what the tree does meanwhile** (there is always something: a default, a refusal, a reading
  held), and a recommendation. A question with no recommendation is a round asking the owner to
  do its thinking.
- **A `Q` file carries no status line.** Whether a question is open is a property of this
  directory, not of prose inside a file: no `A` file means open, and everything beyond that the
  `A` file's header says. (`Q53`'s *provisional, not a blocker* sentence is not a status line and
  stays — blocker-ness is one of the four things a `Q` file may well state.)
- **A question is not a blocker unless it is.** Say plainly whether work is stopped or merely
  provisional. Most of these are provisional: the code ships with a stated default, and the
  answer would change it.
- **When an `A` file appears**, the round that acts on it records the decision in an ADR and
  amends whatever the provisional answer was — and clears the `Owes` item it was, if it was one.
  The `A` file is the owner's word; the ADR is what the tree did about it.

## How a round reads the state of every question

`tools/state.sh questions` prints the index — every number, `open` or its `A` file's status, and
every item still owed — so no round reads prose to know what is waiting. That is the property the
2026-09-04 rule was right to want, and this format keeps it: the whole index is computable from
filenames and three header lines.

**Where these came from.** `Q01` to `Q06` are RFC 0002 §13's, less its first, which the owner
ratified on 2026-09-03. `Q07` to `Q14` are RFC 0003 §9's plus the layout departure session 899
made. `Q15` to `Q22` are RFC 0006 §10's. `Q23` is a rendering reading two sessions arrived at
independently. `Q53` to `Q60` are session 942's and session 954's, and `Q54` to `Q60` are RFC
0007 §7's and §4.7's.