# ADR 0122 — What the writing and the pointer did to the ledger

Status: accepted, 2026-08-01.

## Why a ledger session, now

Six sessions changed what this program *is*: it has a window that is a consumer, a pointer, a
selection, an edit log and a writer. The ledger is 823 claims about what this program does, and
several of them were written when it did none of those. This is the sweep, and it is the
demand-driven half's opposite number — every finding here came from reading a row against the
code rather than from anything a corpus said.

## `writer-side` re-read, which the handover had owed for seven sessions

`CLAUDE.md`'s exclusion changed in the hundred-and-thirtieth session from "we do not create
files" to "we do not *author* documents", and `ledger.toml`'s header still carried the old
sentence. Both are now the amended one, and the seven rows were re-read against it.

**Six stay, and one moves.** §7.6.4.4.7, .8 and .9 compute the `/U`, `/UE`, `/O`, `/OE` and
`/Perms` entries a writer stores *when it sets a password*, which this program never does;
§7.6.7's unencrypted wrapper is a document to be created; §14.12.2 and §14.12.3 are a `DPart`
tree's shape and its connection to pages, and every sentence in them constrains whoever builds
one.

**§7.2.2 is no longer writer-side, and that is the finding.** "Representation" says what a file
may contain: the tokens and standard keys in ASCII, string and stream data either ASCII or
binary, and a binary file transported as a binary file. A reader implements none of it — and
since ADR 0121 this tree *writes*, so all three now bind it. All three are met, and two by
construction rather than by care: every token and key `write.rs` emits is ASCII, and a string is
written in §7.3.4.3's hexadecimal form, which is ASCII whatever the bytes are.

Five more rows gained the write side in their notes — §7.5.4's table, §7.5.5's trailer, §7.5.6's
update, §7.5.8.2's stream and §14.4's identifiers — and §7.5.6 became `partial`, because an
encrypted document is refused on the way out.

## Two stale rows, both of the classes the handover names

**A reason that had expired.** §12.2's viewer preferences read every entry of Table 147 and
applied the two that decide pixels, giving as its reason for the rest "a window this program does
not have". It has had one since the hundred-and-thirty-second session. The row is now honest in
both directions: `viewer-core` hands Table 147 over whole (`Query::Preferences`), which is the
most a crate with no window by construction can do, and consumer #1 still hides nothing because
it has no tool bar to hide. ADRs 0107 and 0108 are the same shape; this is the third.

**An aggregate row that understated its children.** §14.9 said what was owed was "a *consumer* …
and §14.9.3's third location, an annotation's `/Contents`" — and §14.9.3's own row records that
third location as read since the sixty-sixth session. Session 115 met six of these in one family;
they are found by reading a parent row against its children and by nothing else.

## The gap the pointer opened

**Table 192's `/H` is a clause that became reachable when this program grew a pointer**, and it
is now the largest thing §12.5.6.19 owes.

> The annotation's highlighting mode, the visual effect that shall be used when the mouse button
> is pressed or held down inside its active area: N (None) No highlighting. I (Invert) Invert the
> colours used to display the contents of the annotation rectangle. O (Outline) Stroke the
> colours used to display the annotation border. … P (Push) Display the annotation's down
> appearance, if any. … T (Toggle) Same as P (which is preferred).

This tree shows §12.5.5's down appearance whatever `/H` says. That is right for P and T and wrong
for the other three — **including the default, which is `I`**, and so every widget that states no
`/H` at all. And one sentence makes it worse than an omission: "[a] highlighting mode other than
P shall override any down appearance", so a widget stating `/H /I` beside a `/D` is drawn with
the appearance the clause says to override.

Before the hundred-and-thirty-second session nothing pressed a mouse button and the entry
described a moment that never happened. Recorded rather than fixed here, because it wants a
drawing change and a scene to check it with, and it is the next session's subject.

## The lesson

**A feature can make a clause reachable, and nothing announces that.** The ledger's statuses are
claims about *this program*, and six sessions of new capability quietly invalidated some of them
— not by changing the code those rows describe, but by changing what the program can be asked to
do. `/H` was honestly out of reach in session 131 and honestly owed in session 132, and no gate,
grep or corpus document marks the difference.

The cheap defence is the one this session used: after a session that adds a *capability* rather
than a clause, re-read the rows whose notes give a reason beginning "this program has no".
