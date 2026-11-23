# 3399 — Table 294's hidden window is a sound played here

Session 2174. Status: accepted. Context: ADR 3350; ISO 32000-2 §13.2.6.1, Tables 293–295.

## Decision

1. **This program's player shows a sound in no window and paints nothing**, which is Table 294's
   `/W` 2, "\[a] hidden window". The window type settles as `MH`'s `/W`, else `BE`'s, else the
   default 3; an unrecognised `MH` value is non-viable and an unrecognised `BE` value is the
   default, as the table states.
2. **Settled at 2, the other entries are ignored silently, because the table ignores them**: `/B`
   "shall be ignored for hidden windows", `/O` "shall be ignored for full-screen and hidden
   windows", `/M` is "\[i]gnored for other types", `/F` is ignored unless `/W` is 0.
3. **Settled at any other type, the player cannot honour it**: 0 and 1 ask for a window, and 3
   asks that `/B` be painted where "the media object does not entirely cover the rectangle", which
   a sound never covers and this player never paints. Every `MH` entry is then a reason the
   rendition is non-viable, and every `BE` entry is set aside and said.
4. The hand-off is unchanged: it is told none of Table 294.
