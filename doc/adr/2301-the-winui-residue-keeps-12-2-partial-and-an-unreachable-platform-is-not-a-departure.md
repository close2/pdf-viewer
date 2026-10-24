# 2301 — The WinUI residue keeps §12.2 `partial`, and a platform the gates cannot reach is not a departure

Status: accepted for integration. Session 1733, 2026-10-23, the host slot of batch one hundred and
twelve. Clause: ISO 32000-2 §12.2 (the row stays `partial`; its note rewritten to what remains).
Context: ADRs 1035, 1119, 1145, 1429, 1622, 2136, 2200, 2201, 2300; `doc/questions/A287`.

## 1. What the row owes after ADR 2300

ADR 2300 built the two items `doc/todo/01`'s entry named as buildable on Linux: §12.4.4's pump now
runs in the C window, so a WinUI presentation advances and draws its transitions through
`quorra_window_tick`, and GTK and Qt park a presentation by the same calls the C window does. What
is left is the WinUI window's own placement: native full-screen and title-bar handling for Table
29's `FullScreen`, Table 147's `/FitWindow` and `/CenterWindow` answered to `QUORRA_OWED_PLACE`, the
suppression of modal dialogues Table 29's "or any other window visible" asks for while presenting,
and every one of those run on Windows rather than compiled against its SDK.

## 2. Why that residue is not `departed`

`departed` is the owner's word for a requirement "decided against with its cost recorded", after
which "[n]othing is owed" (ADR 1119, `doc/PLAN.md` §5a), and only a `shall` addressed to this
program can be departed from (ADR 1622). Nothing here was decided against. A287 adds the
Windows-native host to the application set, and §12.2 states what that window shall do as it states
it for the other three: the dictionary's entries are "controlling the way the document shall be
presented on the screen or in print". That no gate on this machine can run a WinUI window is a fact
about the gates, not a decision about the requirement; ADR 1035 §2 is the rule — `partial` is a
requirement "owed: stated by the standard, addressed to this program, not executed, and nobody has
decided against it" — and "a row is not moved on a guess". So the row stays `partial`, and moves when
a round with a Windows machine executes the placement and runs it there, or when the owner decides
that host may leave those entries unanswered, which would be a departure with its cost written down.

## 3. The two departures already in the row stay where they are

`/HideMenubar` over the menu that holds the reader's restriction levels (ADR 1145) and `/CenterWindow`
in GTK, which gives a client no call to place its own window (ADR 1429), are deliberate departures
with their reasons recorded. They do not make the row `departed`, because the WinUI residue is owed
beside them; once that residue is executed, those two are what a `departed` row would name.
