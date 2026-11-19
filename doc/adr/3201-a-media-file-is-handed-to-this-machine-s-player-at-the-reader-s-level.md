# 3201 — A media file is handed to this machine's player at the reader's level

Session 2125. Status: accepted. Context: `doc/questions/A254` (RFC 0009 section 11, question 3), RFC
0015 section 3, ADRs 1155, 1227, 1961, 2171, 2185, 2382, 3202.

## Decision

`viewer_host::policy::MediaFiles` is the `Media` level A254 owes: `CLAUDE.md`'s four levels over one
act — a clip, a movie's or a sound's file handed to this machine's own player — spelled
`--media-files=refuse|ask|warn|open`, `ask` by default. It sits beside `LaunchedDocuments` and is
built the same way: a field of `Settings` (`Settings::take`, and `Settings::set` with
`Policy::MediaFiles` for a host without a command line, `QUORRA_POLICY_MEDIA_FILES` = 7 in the C
ABI), an act of the restrictions menu (`Act::HandingMedia`, `ActLevel::MediaFiles`), and a question
of its own (`Subject::Media`, "Play this media clip?").

The level is the reader's alone. `viewer_core` has already asked Table 286's `/TF` at the
restrictions level before `Event::Media` is emitted; a document can neither raise nor lower
`MediaFiles`, and a clip whose `/TF` lets it through is still put to the person at the default.

`viewer_host::handing` decides, and three refusals come before the level, at every level including
`open`, because a reader who set `open` agreed that a player may be started and nothing more:

- an embedded clip whose bytes begin as an executable or a script does (`launched`'s test, ADR 2171);
- a `File` outside the document's directory, resolved by `resolve_import` and nothing looser
  (ADRs 1155, 1227); a file beside the document is checked for being a program when it is opened,
  after the level allowed reading it;
- a `Url` under a scheme other than `http` or `https` (`MEDIA_SCHEMES`).

## Where each window asks it

The GTK and Qt windows and the C ABI's window share `viewer_host::native_window`, whose `media`
handler and `Asked::Media` answer carry the decision. Their menus, and `quorra`'s card, are built from
`Restrictions::rows`, so the new act appears in each with no per-window menu code; GTK names its
action `media`. `quorra` reads the level from the menu (`Pending::Media`). `quorra-confined`'s `r`
menu now offers both acts it performs itself, a launched document and a media clip, and its
unconfined side makes the hand-off; the worker's system calls are unchanged.

## Name

`MediaFiles` instead of `Media`, so the level is never read as `viewer_core::Media`, which is the
clip itself.

## Not done here

On Windows, `Portal` declines the hand-off by name rather than attempting it. The WinUI window does
not yet handle `QUORRA_EVENT_MEDIA` and does not open a launched document either. Its hand-off,
through the platform's opener (`Launcher.LaunchFileAsync`), is the next step; the constant is
already in `Native.Constants.cs`. macOS has no host.
