# Q35 — Should a reader guess a font the file does not carry at all?

Raised by session 926, out of `corpus-cache/tika-issue-tracker/batch5/pdfminer.six`'s ink ranking.
The reading is [ADR 0893](../adr/0893-a-font-dictionary-that-is-the-null-object.md); this is the
half that is not the specification's to answer.

**Asked as `Q27`, and renumbered here by session 934.** Rounds 926 and 927 both took the number 27
on 2026-09-04 from branches that could not see each other, and round 929's merge is where the two
met; `Q28` recorded the collision rather than fixing it unilaterally. This file is the one that
moved, because it was named in fewer places than the other — three links and five mentions against
sixteen mentions — and it took a number out of session 934's own reserved block, which is the only
number a round can know is free while its neighbours are running.
[ADR 0908](../adr/0908-two-questions-called-q27.md) is the argument, and `README.md` now carries
the rule and the check that keeps it.

## The question

A page shows text through a `/Font` resource whose entry names an object the file does not define.
§7.3.10 makes that the null object and says so is "not … an error"; §9.5 makes a *substitute*
something chosen from the font dictionary's own entries, and there is no dictionary. So the
specification defines the case as an absence and offers nothing to fill it with.

**Should this program nevertheless show the text, in a face of its own choosing at advances it
invents, and say so — or leave the text out and report it?**

## Why it cannot be settled without you

It is not a clause reading. It is a decision about what a reader shows a person when the file has
lost the information, and the two references disagree with each other about it — in opposite
directions on two documents of the same 123:

| | `pdfminer.six-90-0.pdf` | `pdfminer.six-50-0.pdf` |
|---|---|---|
| what the codes are | ASCII through eight fonts a truncation removed | `<000102…0c>`, a subset's own codes |
| `pdftoppm` | refuses the file entirely | substitutes — **solid black blocks** where the labels belong |
| `mutool draw` | substitutes — a **legible letter** | draws no text, as we do |
| this tree | draws no text | draws no text |

So on one page the guess reads as the document and on the other it destroys a drawing, and nothing
in the file distinguishes the two in advance. That is a judgement about the product's manners, and
`CLAUDE.md` principle 5 is explicit that where the standard defines nothing the answer is "a
documented choice".

## What the tree does meanwhile

**Nothing is blocked.** The page draws everything else it states — images, rules, paths, and any
font whose dictionary the file does carry — and reports each missing one by name and clause:

```
Font { detail: "the /Font entry F1 is stated and is not a font dictionary — §7.3.10 makes a
reference to an object the file does not define the null object, which is not one" }
```

That is the behaviour ADR 0893 has now argued for rather than merely inherited, and it is what
ships until this is answered. 40 of 24 324 corpus documents reach it.

## Recommendation

**Keep the refusal as the default, and make the guess a thing the reader can ask for**, if it is
wanted at all — the same shape `doc/todo/38` gives a document's restrictions: the *policy* is asked
once, in a place a host can supply, rather than decided inside `pdf-model`. Concretely: the
interpreter keeps reporting, and a host that wants the guess passes an option that turns the null
entry into a substituted face with `/MissingWidth`-style default advances, with the page still
carrying the report so that nothing about it is silent.

The reason for that ordering rather than the reverse is the second column of the table above: a
default that is right on one page and produces black bars on the other is a default that makes the
program untrustworthy in the case where its output is least checkable — and an absence a reader can
see is recoverable, while a page of confident nonsense is not.
