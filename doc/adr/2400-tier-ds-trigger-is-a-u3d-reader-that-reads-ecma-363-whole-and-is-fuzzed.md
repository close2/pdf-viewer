# 2400 — Tier D's trigger is a U3D reader that reads ECMA-363 whole and is fuzzed, or the owner's decision to write one

Session 1783, a "dependency" round of batch one hundred and eighteen, 2026-10-26.
Status: accepted — the owner's A632 ("Q632 as recommended please") adopted Q632's second
recommendation, and `CLAUDE.md`'s clause 13 entry now carries its words. Context: Q632, review 1768
section 1.3, RFC 0009 section 3.5, ADR 1946, RFC 0012. Moves no ledger row.

## 1. The sentence

`CLAUDE.md`'s clause 13 exclusion named its 3D trigger as "a U3D reader in Rust". It now reads, in
A632's words: **"a U3D reader in Rust that reads ECMA-363's fourth edition whole and is fuzzed, or
the owner's decision to write one in-tree"**. That parenthesis is the only change to `CLAUDE.md`.
The ratification line under the entry (Q254, A254) still dates the exclusion itself, and A632 dates
the trigger.

## 2. Why the old words had to change

On 2026-10-09 the old words were met by a crate's existence. `u3d` 0.1.0 (MIT, no `unsafe`, 2 153
lines) is a U3D reader in Rust. Its own README leaves three things unticked: the progressive-mesh
continuation, fuzzing, and Adobe's mesh resource. crates.io still lists that one release on
2026-10-26. A trigger that a 2 000-line start can meet invites the attrition `CLAUDE.md` forbids,
where an exclusion falls because something with the right name turned up. The new words name what
a reader *is*:

- **"reads ECMA-363's fourth edition whole"** — the whole of the standard's block set, the
  continuous-level-of-detail mesh and its continuation blocks included, because a 3D annotation's
  stream is whatever its producer wrote and §13.6.3 does not let a processor choose a subset. The
  standard is freely published. Its page at `ecma-international.org` answered 200 on 2026-10-26.
- **"is fuzzed"** — a U3D stream is untrusted input with its own context-adaptive compression. It
  goes into a confined worker under ADR 0014's sentence, and that worker is fuzzed from its first
  commit (principle 3).
- **"or the owner's decision to write one in-tree"** — the other route, and it belongs to the owner,
  never to a round.

## 3. What the trigger does not cover

- **PRC.** The exclusion is "rendering U3D or PRC artwork", but the trigger names only U3D. A U3D
  reader therefore reopens the U3D half. PRC (ISO 14739-1) is a paid standard this project does not
  hold, and `prc` 0.2.0 carries 50 `unsafe` (review 1768), so PRC needs an argument of its own.
  This is recorded here so that no round reads a U3D reader as reopening PRC.
- **The oracle.** Neither pdf.js nor poppler renders 3D. A Tier D renderer would have no second
  implementation for the cross-backend comparison to hold it against, and the CPU backend would
  have to draw the meshes as the oracle. That is a cost of reopening Tier D, not a condition of the
  trigger, and RFC 0009 section 3.5 prices it.

## 4. What stays as it is

§13.6's rows, §12.6.4.16 and §13.7's 3D configuration stay `out-of-scope`. A 3D annotation's
normal appearance is drawn, and §13.6.2 requires the file to supply one. Without one the annotation
is refused by naming that requirement (ADR 1946).
