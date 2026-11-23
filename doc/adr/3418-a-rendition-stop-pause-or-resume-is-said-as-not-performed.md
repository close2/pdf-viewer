# 3418 — A rendition's stop, pause or resume is said as not performed

Session 2179. Status: **superseded by ADR 3433**, which performs the three. Context: ADRs 1960, 3350; ISO 32000-2 §12.6.4.14, Table 218.

## Decision

`viewer_core::interact::rendered` said "nothing plays in this program", which stopped being true
when the windows began playing a WAVE clip (ADR 3350). Table 218's stop, pause and resume act on
"any rendition being played in association with the annotation specified by AN"; a window that
plays a sound keeps no such association, so the three are not performed, and the note now says
exactly that. Pause and resume still carry the clause's "the user should be notified of the
failure to perform the operation". Performing them — the core recording which annotation a sound
was played for, and the window stopping, pausing or resuming it on the action — is a later
round's.

## The ledger

§12.6.4.14's row said these three act "on a rendition already playing, and none ever is", which
ADR 3350 made false. It is `partial` now, naming the three operations as what is owed and what
would move it.
