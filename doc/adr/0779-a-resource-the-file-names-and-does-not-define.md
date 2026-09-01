# 0779 — A resource the file names and does not define

Session 855. Status: **accepted**.

## Context

The eight-hundred-and-fifty-fifth session fetched SafeDocs' *Issue Tracker* corpus for the first
time — bug attachments from the trackers of the PDF tools themselves — and walked its first chunk.
Among the reports on `evince-1360-1.pdf`, a 5101-byte cairo page reduced by whoever filed it:

```
Font { detail: "no /Font resource named /f-0-0" }        ... six times, /f-0-0 to /f-5-0
```

The page's `/Resources` is object 3, and object 3 says

```
/Font << /f-0-0 6 0 R /f-1-0 7 0 R /f-2-0 8 0 R /f-3-0 9 0 R /f-4-0 10 0 R /f-5-0 11 0 R >>
```

**The file names every one of the six.** What it does not carry is objects 6 to 11 — the reduction
that made the bug report small threw them away. So the sentence the reader printed six times is
false about the document, and it sends whoever reads it to §7.8.3 when the clause that decides the
case is §7.3.10.

## Decision

**Tell the two conditions apart, and say which one it is.**

- The resource dictionary states no entry under the name — §7.8.3, whose `shall` is that a
  resource dictionary "enumerate the named resources needed by the operators in the content
  stream". Wording unchanged: `no /Font resource named /F1`.
- The entry is stated and what it names is not a font dictionary — §7.3.10, "[a]n indirect
  reference to an undefined object shall not be considered an error by a PDF processor; it shall
  be treated as a reference to the null object", and null is not a dictionary. New wording, naming
  the clause.

`Interpreter::font` is the only place that still holds the *unresolved* entry, so that is where
the two are told apart; `load_font` takes the answer as an `Absent` and both routes to a font pass
one, including Table 57's `/Font`, whose failure now reads *the /Font entry object 6 0 …* rather
than *no /Font resource named /object 6 0*.

## Why this is worth a change and not a wording preference

**It is the last resource category folding them.** `XObject` has distinguished *is not in
`/XObject`* from *is not a stream* since ADR 0255, and `Shading` says *`/Sh0` is not in
`/Shading`*. `Font` said the first of the two for both conditions since the interpreter had fonts,
so the population ADR 0255 counts under §7.8.3 has been carrying a second population that belongs
to another clause — trap 11's shape, in a report rather than in a gate.

## What it moves, measured

One survey per population, the same build either side:

| population | reports | §7.8.3 | §7.3.10 |
|---|---|---|---|
| `doc/pdf.js/test/pdfs`, 974 | 2 | 2 | **0** |
| the issue tracker's `cairo-gitlab` + `evince`, 270 | 12 | 0 | **12** |
| its `PDFBOX` tracker, 3318 of 3792 | 80 | 70 | **10** |

**The 974 do not move at all**, which is the result to want: ADR 0255's counted population keeps
its wording, and what changes is the documents that were never in it. The curated corpus has no
witness for this condition and two orders of magnitude more issue-tracker documents have
twenty-two, which is `doc/traps/parsers-and-streams.md` trap 8's own sentence about what a corpus
can and cannot show.

## What was rejected

**Reporting nothing for the second case.** A font the page shows through and this reader cannot
load costs marks either way, and trap 5's rule is that the shortfall stays loud. What changed is
the sentence, never whether there is one.

**Folding it into `Unsupported::MissingResource`.** `Font` has its own variant because a font
fails for a dozen reasons that are not a lookup, and moving one of them out would split a font's
reports across two vocabularies for no reader's benefit.
