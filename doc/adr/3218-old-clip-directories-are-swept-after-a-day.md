# 3218 — Old clip directories are swept at the next hand-off, after a day

Session 2129. Status: accepted. Context: ADR 3202 ("a later round may sweep this program's own
directories older than the session at the next hand-off").

## Decision

When `hand_media` writes a clip, it removes every `quorra-media-*` directory beside the new one that
was last changed more than `SWEPT_AFTER` (24 hours) ago and that belongs to the user who owns the
directory just created. It reads the owner from that directory rather than asking the system. On
Windows, whose temporary directory is per user, the name and the age decide. No symbolic link is
followed, and `remove_dir_all` does not follow one inside a directory. A directory that another
window removed first does not count as a failure. A directory that could not be removed is named
in the hand-off's sentence, and it is tried again at the next hand-off.

## The bound

What a removal can break decides the bound. On Unix, a player that holds the file open is not
disturbed: removing a name leaves an open file readable. On Windows, removing an open file fails,
and the next sweep tries again. A removal breaks only a player that opens the file *again by its
name*, for a replay or from a list of recent files, and that happens within the sitting in which
the clip was played. A day outlasts a sitting, including a window left open overnight. It also
limits what is kept to one day's clips, in a directory that `$XDG_RUNTIME_DIR` empties at logout
anyway. "Older than the session" (ADR 3202) cannot be read portably: no session start is recorded
anywhere this program can see. An hour was rejected because a paused film outlasts it.
