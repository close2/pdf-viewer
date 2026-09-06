# Q09 — Reordering pages, and whether intermediate names solve it

Source: RFC 0003 §9 question 3, first asked 2026-09-03. **Answered 2026-09-06 in
`A09-reorder-by-mv.md`, and the owner asked for the question to be put again with their text and our
response in it.** This is that second asking.
Status: **open** — answered when `A09-reorder-by-mv.md` says so, replacing what it says today.

## What the owner said

> Could we avoid this problem, by allowing "intermediate" names?
>
> If we for instance named every page 001.50.pdf repositing a page before could be achieved by
> naming the new page 001.30.pdf
>
> Unless you know why, the 50 is of course a "strange" number. Maybe you have another idea.
>
> This question is not answered. Integrate my text and your response to it into the question again
> and let me answer again.

## What the problem was

A page's name in the mount is its **position**, not its identity: after any write the next listing
renumbers, which is what makes insertion and deletion coherent. Renaming within `pages/` is refused
today, because the obvious meaning of `mv 0007.pdf 0002.pdf` is a reorder, and a filesystem rename
has no way to say "insert here and shift everything after" atomically. A sequence of renames that is
interrupted leaves a document in an order nobody asked for.

## Our response

**The owner's idea is the standard solution to this problem and it does work.** Sparse keys with
room between them are how ordered lists are maintained in databases and in collaborative editors,
and `001.30.pdf` between `001.00.pdf` and `001.50.pdf` needs no shifting, no atomicity beyond the
single rename the kernel already gives us, and no second grammar. It is a better answer than the
refusal we shipped. Four things follow, and they are what the owner is really being asked to rule
on.

**1. It changes what a name means, and the change is the point.** Today a name is a position and
the listing renumbers. With intermediate names a name is a *sort key*: stable across other people's
inserts, which is strictly more useful, and no longer readable as "this is page 7". Both cannot be
true at once. Our recommendation is that the sort key wins **and the label is shown elsewhere** —
`meta/` already reports the page labels §12.4.2 defines, which are what a reader actually sees on
the page.

**2. Something must eventually renormalise.** Repeated insertion between neighbours exhausts the
gap: with two decimals, inserting before the same page fifteen times runs out. The options are a
renormalising pass on the next listing (cheap, but every name changes under whoever is looking),
renormalising only on unmount or on an explicit request, or simply widening the key until exhaustion
is not reachable in practice.

**3. On the number itself — we would not use decimals at all.** The owner is right that 50 is
strange, and the reason it looks strange is that decimal fractions are a poor sort key: they sort
correctly only if every name has the same number of digits, and "insert before the first page"
has no answer at all, because there is nothing below `001.00`. Two better shapes:

- **Integers with a stride**, the BASIC line-numbering trick: pages are `00100`, `00200`, `00300`,
  and inserting before the second is `00150`. Nothing below the first? Start at `00100` rather than
  `00000`, so there are ninety-nine slots before page one. Sorts lexicographically, reads as a
  number, and renormalisation is a pass that rewrites the stride.
- **Fractional ranks in a base that never exhausts**, the collaborative-editor answer: a key is a
  string, and between any two strings another always exists (`a` < `an` < `b`). Never needs
  renormalising, and reads badly to a person, which for a name a person types is disqualifying.

We recommend **integers with a stride of 100**, starting at `00100`.

**4. What a rename must still refuse.** Renaming across directories, renaming to a name that is not
a well-formed key, and renaming onto an existing key all have to fail with a stated reason, and a
rename is a §7.5.6 append like every other write, so it is warned and undoable in the sense
`A08-write-support-default.md` describes.

**One alternative the owner should weigh against all of this**: leave reorder refused in the mount,
because `quorra transform pages --reorder` does it in one operation with none of the above. The case
for the mount doing it is that a file manager is where a person naturally drags a page; the case
against is that this is a second, weaker way to do something the transform layer already does
exactly.

**Recommendation**: implement it, with integer keys and a stride of 100, renormalising only on
explicit request, page labels shown in `meta/`, and the four refusals above stated by name.
