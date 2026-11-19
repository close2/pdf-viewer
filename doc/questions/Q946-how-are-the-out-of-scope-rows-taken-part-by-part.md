# Q946 — How are the 51 out-of-scope rows taken, part by part?

Source: round 2124, RFC 0015 (`doc/rfc/0015-the-out-of-scope-rows-planned-what-playing-and-rendering-clause-13-would-take-done-safely.md`),
the planning step the owner set on 2026-10-28 and confirmed on 2026-11-19. Nothing is built or lifted
until this is answered; each exclusion and trigger is the owner's (`doc/scope-ratifications.md`).

## What the RFC found

- The 51 rows are 27 of §13.6 (3D), 18 of §13.7 (rich media), §13.1, §13.2.1, §13.2.7.5, §13.5, and
  the actions §12.6.4.16 and §12.6.4.18. Audio and video playing is only three of them; the rest of
  §13.2, §13.3 and §13.4 are `implemented` for the hand-off.
- **The hand-off the exclusion admits is not finished.** Every window receives the media event and
  says "this window starts no player yet, so nothing plays". The `Media` level `doc/questions/A254`
  owes, and starting this machine's own player, are unbuilt. That is in scope today.
- Every part has one safety shape: a parser or decoder in a confined worker of its own (a media
  worker, a scene worker), spawned on the first activation the reader's level allows and never at
  open, handing the host only samples, frames, or a bounded triangle scene; 3D is drawn on the host's
  device under the shared-device lock, with `render-cpu` as the oracle because no other open renderer
  draws 3D. Nothing reaches the launch path.
- **The U3D trigger names the fourth edition of ECMA-363; ISO 32000-2 names the third**, and says it
  does not cite the fourth (its introduction, its bibliography, and its definition 3.67).
- **PRC has no trigger** (ADR 2400 section 3), and the project does not hold ISO 14739-1, so a PRC
  reader cannot be planned or written under principle 5. Two Rust PRC readers now exist
  (`prc` 0.3.0, `pdfcer-3d` 0.163.0); without the standard neither claim can be judged.

Estimated costs, in rounds: the hand-off 2; audio 9; video 14 after audio; U3D 24; PRC 30–40 (low
confidence, after the standard is held); rich media 5 on top of its parts; Go-To-3D-View 1 after U3D.

## The questions, one per part

1. **The owed hand-off.** Build the `Media` level and the start of this machine's own player now
   (2 rounds)? *Recommendation*: yes, first.
2. **Audio.** Open decoding and presenting *audio* alone — the sound object's own samples and AAC
   (`rusty_aac`, scalar build) in a confined media worker, an audio sink in each window — leaving
   video closed? *Recommendation*: yes in RFC 0015 section 5's order, if you want this program to be
   the player at all; the hand-off of question 1 is the alternative §13.2.1 itself names.
3. **Video.** Keep it closed until audio has shipped and its clock and sinks are measured in use?
   *Recommendation*: yes, then reopen on your word in section 6's order (14 rounds).
4. **U3D.** Restate the trigger to the edition the standard names — "a U3D reader in Rust that reads
   ECMA-363's third edition whole and is fuzzed, or the owner's decision to write one in-tree" — and
   keep it closed? *Recommendation*: yes; if you decide to write one, section 7's 24 rounds, the
   confined scene worker first.
5. **PRC.** Add the trigger it lacks: "the project holds ISO 14739-1, and a PRC reader in Rust reads
   it whole and is fuzzed, or the owner's decision to write one in-tree"? *Recommendation*: yes;
   buying the standard is yours to decide, and nothing about PRC is plannable before it.
6. **Rich media and the two actions.** Let rich media and Go-To-3D-View follow the parts they are made
   of, with no trigger of their own; leave Rich-Media-Execute and §13.7's script run time with the
   script exclusion's ISO 21757-1 clause; and record §13.5 (alternate presentations, a web runtime
   the clause itself releases a reader from) as planned by no part? *Recommendation*: yes.
