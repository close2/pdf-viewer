Status: complete
Given: 2026-10-05, in conversation — transcribed by the round
Owes: RFC 0008 accepted and its build order run — Tier 0 now (`pdf_model::aform`, `/CO` walked in order, every rule an ADR names as a choice, `script_corpus` in its Tier 0 form, §12.7.3's row moving, §12.6.4.17 staying `out-of-scope` until ECMAScript runs); Tier 1 after, in §11's order (`pdf-script` with the engine behind a feature, the `Outcome` type, the budgets, the third confined process and its profile, the named refusals a script can see, the `Scripts` level in `viewer_host::policy` with the default `off` and `quorra-confined` pinned to it, `script_corpus` full, `script_open`'s cost line, §12.11.1 and §12.11.5); `CLAUDE.md`'s exclusion amended with §10's sentence; Adobe's reference pinned at commit `ab3b42a7` and cited once in the `pdf-script` crate's root; and **the scope note made a rule inside the tiers: the target is as much of the pinned reference's surface as it defines for this document and this viewer, the census ranking the work and never capping it — no API member inside the tiers is left unimplemented because no corpus document calls it, and nothing that would be insecure or unreasonable is implemented either, each such member refused by name with its reason** (acting round)

> I agree with all your recommendations.  Do I understand it correctly, that with you
> recommendation, we would not implement all javascript functionality?  If yes, please add a
> note, that we should try to implement as much as possible from the spec (provided it's still
> secure and reasonable) but not limit ourself to the corpus.  (if this is the answer already,
> ignore my comment).

Reading: **Yes — understood correctly, and the note is added, not ignored.** With these answers,
not all of the JavaScript the reference defines is implemented: Tier 2 — every call that leaves
the process, the network, the filesystem, mail, printing, other documents, the application's
chrome, persistence, privileged functions — stays excluded, and that boundary is the security
architecture, not a corpus limit; the note's own proviso, *provided it is still secure and
reasonable*, preserves exactly it. Where the note bites is the other half: inside Tiers 0 and 1,
the census is a ranking instrument and never a scope, and the target is as much of the
specification's surface as the pinned reference defines for this document and this viewer — the
`Owes` line makes it a rule, so no later round reads the census as a ceiling. The eleven, answered
by number as the RFC asked: **(1)** `off` until `script_corpus` has been green for a batch, then
the default re-asked with the gate's number, `quorra-confined` pinned to `off` throughout;
**(2)** Tier 0 built first, on its own criterion; **(3)** Boa, with QuickJS the named defensible
alternative and the same design behind either; **(4)** `app.launchURL` and `this.submitForm`
through the existing link and submission policies, a script raising the question a link raises and
never more; **(5)** the engine in a third confined process, spawned on the first trigger the level
lets run; **(6)** open-time scripts after the first present; **(7)** Adobe's reference cited for
every API member, pinned by commit; **(8)** the exclusion amended with §10's sentence; **(9)**
ECMAScript 2020; **(10)** `setTimeOut`/`setInterval` admitted and bounded to the document;
**(11)** `viewerType` and `viewerVersion` answered truthfully — this program's name and version —
under the one-dialog cap, the level the lever for quiet.