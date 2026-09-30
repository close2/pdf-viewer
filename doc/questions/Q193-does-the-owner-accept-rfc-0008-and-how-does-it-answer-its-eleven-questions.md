# Q193 — Does the owner accept RFC 0008, and how does it answer its eleven questions?

**The question.** `doc/rfc/0008-a-script-is-a-document-acting-on-its-reader.md` is the RFC the
owner commissioned on 2026-09-30 (*"use one round to create an RFC for adding JavaScript. what
would it mean, where should we draw the line, which library..."*). Its §10 puts eleven numbered
questions, each with a recommendation; this file is the one `Q` the round's block holds, and it
asks the owner to answer them **by number**, in one `A193`, the way `A54`–`A60` answered RFC 0007's.
The eleven, in one line each:

1. the default level for scripts — `off` or `on`;
2. whether Tier 0 (the native `AF*` library and `/CO`, no engine) is built before any engine;
3. the library — Boa, or QuickJS through `rquickjs`;
4. whether `app.launchURL` and `this.submitForm` go through the existing link and submission
   policies rather than being refused;
5. whether the engine lives in a third confined process rather than in `pdf-view-worker`;
6. whether the open-time scripts run after the first present;
7. whether Adobe's reference is the cited source for every API member, pinned by commit;
8. whether `CLAUDE.md`'s exclusion is amended, with the sentence §10 writes;
9. whether ECMAScript 2020 is the language target;
10. whether `app.setTimeOut`/`setInterval` are admitted, bounded to the document;
11. what `app.viewerType` and `app.viewerVersion` answer — the census found Adobe's own
    viewer-version check to be the commonest script in the world, and its branches turn on them.

**Why it cannot be settled without the owner.** Questions 1, 3 and 8 change a principle's text, a
dependency and the default a reader meets; `doc/rfc/README.md` makes the owner the decider of every
RFC's status, and `CLAUDE.md` says an exclusion is revisited "by argument, never by attrition".

**What the tree does meanwhile.** Exactly what it does today: every script action is refused by
name (`crates/pdf-model/src/action.rs`), `EnableJavaScripts` is declined with a sentence, `/CO` is
carried and not executed, and the census instrument
(`crates/pdf-model/examples/javascript_census.rs`) is in the tree and runnable whatever the answer.
No round starts Tier 0 or Tier 1 before `A193`.

**Recommendation.** Accept the RFC with its eleven recommendations as written in §10 — in one line:
`off` by default until the `script_corpus` gate has been green for a batch; Tier 0 first; Boa, with
QuickJS named as the defensible alternative; `launchURL` and `submitForm` through the existing
policies; a third confined process, spawned on the first trigger the level lets run; open-time
scripts after the first present; Adobe's reference cited and pinned; the exclusion amended with
§10's sentence; ECMAScript 2020; timers admitted and bounded; `viewerType` and `viewerVersion` answered
truthfully.
