# Q287 — Does the winit window stay the flagship, now that ADR 0001's revisit condition has fired?

Source: the ADR-revisit sweep of 2026-09-16, re-verified 2026-10-07
(`doc/adr_revisit/0001-windowing-toolkit.md` — the one note in that directory no later ADR has
taken up).

## The question

ADR 0001 chose winit and closed with its own trigger: *"Reversible, but at real cost once the
shell is written. Revisit only if conventional widget requirements appear."* They appeared — six
panels in every window (ADR 0564), native form widgets over the page (ADRs 0244, 0245), preference
and restriction menus (ADRs 1144, 1145), printing (ADR 1180) — and the revisit never happened.
Meanwhile the tree grew exactly the build 0001 refused: `quorra-qt` on a `cxx` bridge, "the one
that costs a C++ … the `unsafe extern "C++"` header `cxx` requires" (ADR 0246), while 0001's
decision text still says "do not depend on Qt".

Since ADR 1180 the flagship is also the least capable window for what a platform owns. Only
`quorra-gtk` prints: the winit host "has no toolkit at all", so its print end is "a panel of its own
and a job submitted over IPP, which is a new dependency" — priced in RFC 0004 and deferred — and
ADR 1429's table shows the same shape for `/HideToolbar`, `/HideMenubar` and `/CenterWindow`. The
winit flagship carries roughly fourteen thousand lines of hand-built chrome (ADR 0713's count)
beside two native hosts and a C ABI that implement the same features from their toolkits.

## Why it cannot be settled without the owner

Which surface ships is a product decision no clause decides, and windowing is the owner's stated
preference territory — 0001 itself records "the project owner confirmed Qt/KDE was a preference
rather than a hard requirement". Host parity is the owner's decision too ("the project owner's
decision of 2026-08-20 is that all three hosts stay level", ADRs 0441/0442), and which window is
the flagship decides what that instrument is measuring.

## What the tree does meanwhile

`quorra` (winit) remains the flagship and the tier-2 host: its reprojection machinery, its
software surface and its hand-built chrome carry the features the native hosts get from their
toolkits, with `viewer-host` holding the shared decisions so the three stay level. `doc/todo/15`
keeps the sanctioned consolidation — moving the established windows onto the confined boundary —
as the path that would retire the duplication. Nothing blocks on this answer.

## Recommendation

**Keep the shape; correct the record.** The winit host's distinguishing machinery — reprojection,
the software surface, `--cpu` — is the flagship's investment, `todo/15`'s confined move is the
sanctioned way to consolidate, and re-licensing the flagship to GTK now would strand that
machinery. What ADR 0001 is owed is what ADR 1201 gave ADR 0002: a superseding record that states
the Qt dependence, retires the "little conventional GUI" premise, and turns "do not depend on Qt"
into a rule about the flagship's own dependencies. The alternative — promoting `quorra-gtk`, the
window that can print, place itself and compose the widget no toolkit ships — is real, and ADR
1180 gives it its teeth; it is not recommended only because the confined move reaches the same
place without orphaning the winit investment. The one product decision inside this question:
whether the winit print end (IPP, RFC 0004 §5) is wanted at all, or whether printing stays a
native-host feature.