# 1155 — A link is a program this machine starts, so the reader says whether it may

Session 1159. Status: **accepted**.
Context: `crates/viewer-host/src/policy.rs` (`Links`, `LINKS`, `LINK_SCHEMES`, `URI_HANDLER`,
`Opening`, `may_open_uri`, `open_uri`, `link`, `answered`, `asked_to_open`),
`crates/viewer-gtk/src/host.rs`, `crates/viewer-qt/src/host.rs`,
`crates/viewer-ui/src/bin/quorra/{dispatch.rs,app.rs,arguments.rs}`,
`crates/viewer-ui/src/bin/quorra-confined.rs`, the three binaries' command lines.
Builds: ADR 1079 (the policy in one function), ADR 1062 (a host-supplied answer about this
machine), ADR 1144 and ADR 1145 (the four levels, and the menu that sets them), `doc/todo/38`,
`doc/todo/30`, `doc/ui-boundary.md`.
Clauses: ISO 32000-2 §12.6.4.8 (Tables 210 and 211), §12.7.6.4 (by analogy), RFC 3986 section 3.1.

## 1. What was left, and why it was the shape `CLAUDE.md` forbids

§12.6.4.8's own words are that a URI "identifies (resolves to) a resource on the Internet" and that
"[a] URI action causes a URI to be resolved". Everything about the *document* was executed — Table
210's `/URI` and `/IsMap`, Table 211's `/Base`, and `resolve_uri` against the location of the
document itself. What was not was the act, and `may_open_uri` answered every call with an `Err`
whose sentence said this reader opens nothing itself.

That is a refusal hard-coded at the point of the operation, which is precisely what `CLAUDE.md`
principle 3 says to avoid: "[a] refusal that cannot become an 'ask' is the thing to avoid." The
policy already lived in one function, so what it wanted was a value.

## 2. Four levels, and why they are not `RESTRICTIONS`'s four words

`Links` is `refuse`, `ask`, `warn`, `open`, set by `--links=` in all three windows. The words are
deliberately **not** `off|on|ask|warn`, and the reason is that they would mean the opposite thing:
in `--restrictions=` *off* is the permissive end, because the subject is a restriction the document
asserts over its reader and turning it off lets the operation through. The subject here runs the
other way — this machine starting another program on a string a document chose — so the permissive
end is `open`, and a reader who read `off` as permissive would have turned the wrong way. Two
policies, two directions, two vocabularies; one shared vocabulary would have been a trap with a
tick beside it.

**The default is `ask`, and it is a choice with a reason.** Two things have to hold at once: a
document never reaches another program on this machine by itself, and a reader is not refused by
their own viewer the act the clause describes. *Ask* is the only level that is both — nothing is
handed over without a keypress, and nobody has to configure the program before a link works. The
other three defaults were each rejected: *open* lets a file start a handler unasked, *warn* the
same with a sentence afterwards, and *refuse* imposes on the reader the thing `CLAUDE.md` says a
document may not — except that here it would be this program imposing it. A face with no dialogue
answers *ask* with `viewer_host::unanswerable` and hands nothing over, which is what keeps the
level from behaving like *open* in silence.

**`quorra-confined` is pinned to `refuse`**, out loud and with its reason beside it: that window
has no dialogue, so it could not put the question, and a window that opened links without being
able to ask would be the one face where a document reaches another program with nobody consulted.

## 3. Two refusals before the level, and the order is the point

`may_open_uri` answers `Opening::Refuse` for a still-relative reference and for a scheme outside
`LINK_SCHEMES` — `http`, `https`, `mailto` — **before** it looks at the level. Neither is something
*open* turns on. A reference nothing could resolve names no resource at any level; and a document
free to name any scheme would be choosing which of this machine's handlers runs, which is the
decision principle 3 exists to keep out of a file's hands.

`file` is outside that list although `resolve_uri` produces one for every partial reference beside
the document, and that is the sharp end of the choice. Opening a file a *document* named is
§12.7.6.4's hazard one clause over, where `read_import` answers it with a directory a **person**
supplied; nothing here may be looser than that. The list is one constant and widening it is a
change in one place.

`open_uri` checks the scheme again. That is not a duplicate: it is the function that actually
starts a program, so it is the last place the guarantee can be made, and a host reaching it without
asking the policy would otherwise hand over anything.

## 4. One launcher, three windows, and no new message

The act is `xdg-open` with the URI, spawned with its three streams closed and waited for on a
thread of its own — the handler may run as long as the program it starts, so a window that waited
would stop drawing, and a child nobody waits for stays a zombie. The thread is where the handler's
own failure is read and said, which is trap 5 on a path a person clicked.

It is in `viewer_host` rather than in each window, on `viewer_host::keys`' standing argument: what
a window is obeying is shared and what a dialogue looks like is a toolkit's. Each host matches two
arms of `Link` — a sentence to say, or a question to put — so *ask* cannot be the level a window
forgets. `doc/ui-boundary.md`'s test was applied and **no `Command`, `Event` or `Query` was
added**: nothing about this crosses the core boundary, because the core neither starts programs nor
holds anything while the question is outstanding. The prompt is `restriction::Question`'s own
two-string shape, so a reader who has met one of this program's questions has met the other; GTK
and Qt answer it with the dialogue ADR 1145 built, generalised over what the question is about, and
`quorra` with the card it already draws.

## 5. What this does not decide

Whether a *document's* restrictions reach a link. They do not: §7.6.4.2's Table 22 states no
position for following one, so there is nothing here for `restriction::Operation` to gain, and the
level is the reader's policy about this machine rather than a permission the file asserts. The
restrictions menu is therefore not where `--links=` is set, and a menu entry for it would have put
two opposite directions under one heading.
