# ADR 0993 — Two files, one number: which one keeps it, and what the rename would have cost

Date: 2026-09-11 (session 972)
Status: accepted
Extends ADR 0730 (a number a deleted item used to have is not free) and ADR 0974, whose postscript
recorded the defect this ADR acts on.

## Context

`doc/todo/`'s number prefix is the item's identity: `CLAUDE.md`, the ADRs and the source cite
`doc/todo/NN` and nothing resolves that citation but `ls`. ADR 0730 established the first way that
breaks — a number reused after its item was deleted, so old citations resolve silently to an item
about something else, and `--bin pointers` sees only *findings disappearing*.

ADR 0974's postscript found the second way, which is worse:

> There are **three** — `46` is both `46-a-wheel-tick-that-interprets.md` and
> `46-the-kernel-floor.md`.

Three numbers, six files. **Every pointer is live**, because both files of each pair exist, so no
sweep in this tree can see it at all — not even the count of findings moves. A citation of
`doc/todo/46` resolves to whichever file a reader opens first.

## Decision

### 1. The evidence is what cites the number, and it settled one of the three

`grep -rn 'todo/36' doc/ crates/ tools/ CLAUDE.md`, and the same for `46` and `47`, then read the
sentence around each hit.

**`36` is decided and is not close.** The frame-cadence item is cited by ADRs 0383, 0384, 0385,
0386, 0391, 0457, 0473, 0526, 0545 and 0657, by `doc/QUORRA_NONBLOCKING_RENDER.md`, by
`doc/todo/37` twice, and — the half that decides it — by
`crates/viewer-ui/src/bin/quorra/cadence.rs` four times, which is **every citation of
`doc/todo/36` written from `crates/`**. The retrieval API is cited by ADR 0257 and by three
documents. So the frame item keeps `36`, and the retrieval API moved to
`doc/todo/63-a-retrieval-api.md`, which says in its own header what it used to be called and which
kind of citation means it. The three live documents that cited it — `doc/state-of-play.md`,
`doc/verify.md`, `doc/running-the-viewer.md` — were re-pointed, which is ADR 0281's rule for a live
document; ADR 0257 was not, which is ADR 0232 §2's rule for an ADR.

### 2. `46` and `47` keep their duplicates, because the rename costs more than the ambiguity

This is the finding, and it is the reason the instruction in `doc/todo/README.md` to *grep first*
is not a formality.

All four files of those two pairs are cited **by their full filenames** —
`doc/todo/46-the-kernel-floor.md`, `doc/todo/46-a-wheel-tick-that-interprets.md`,
`doc/todo/47-the-encode-term.md`,
`doc/todo/47-the-resize-frames.md` — from ADRs 0766, 0767 and 0770 and from four files under
`doc/history/`. Those are live path pointers today. An ADR is not edited to follow a file that moved
underneath it and a history file is not edited at all, so **renaming either half of either pair
turns live pointers into absent ones in files nothing may ever repair**, and `--bin pointers` would
carry them for the life of the project.

`36` was renameable precisely because it had no such citation: outside `doc/todo/README.md`,
nothing in the tree named that file.

**So the cure is worse than the disease for two of the three, and the honest output is a
disambiguation rather than a rename.** `doc/todo/README.md` now carries a four-row table keyed on
the *subject* a citation is about — the compute kernels against §12.5.3's annotation pass for `46`,
the encode term against the resize drag for `47` — because the two subjects in each pair are
disjoint and a reader has the sentence around the citation in front of them.

### 3. A third referent, and the rule that produces it

Each of the three numbers has a *third* meaning that no longer exists, which is ADR 0730's defect
rather than this one: ADRs 0256, 0260, 0317, 0330, 0335, 0372, 0374 and 0413 cite a `doc/todo/47`
that was the cold document-wide search, deleted by ADR 0335; ADR 0373 cites a `doc/todo/46` it
deleted itself; ADRs 0200 and 0202 cite a `doc/todo/36` about a collection's ordering. Two live
files and one dead one behind a single number is the state `47` is actually in.

### 4. A band holds ten numbers, and the prefix stops being the priority when one fills

`doc/todo/README.md`'s *Sorting* section said "The number prefix **is** the priority, and `ls`
sorts by it", and that has been false for a while: `60`, `61` and `62` all carry `Priority:
50-band` in their own headers, because the 50s were full when they were written. The 30s are full
too — `30` through `39` with `36` twice — which is why the retrieval API is `63` rather than a free
slot near its neighbours.

Corrected rather than dropped, which is ADR 0974's heading 4: the sentence now says *wherever the
band has room*, and the paragraph under the table says what takes over when it does not — the
header block's `Priority:` line — and that `ls` sorts those items last rather than by priority.

## Consequences

`--bin pointers` reports 185 absent before and after. One rename, no ADR edited, three live
documents re-pointed.

**The sweep is written down and the answer is not.** `ls doc/todo/ | grep -oE '^[0-9]+' | sort |
uniq -d` is one command and cannot miss a pair; the round that found these found two of the three
by reading and the third only once that command was run. `doc/todo/README.md` carries the command
where a round looking for a free number will meet it, which is ADR 0281's rule about counted facts
applied to a defect rather than to a gate.

**What a later round could still do about `46` and `47`.** Nothing, while the citations stand — but
a round that *closes* either item deletes its file, and at that moment the surviving file of the
pair owns the number unambiguously and the disambiguation row for it can go. That is the cheapest
fix available and it costs this project nothing to wait for it.
