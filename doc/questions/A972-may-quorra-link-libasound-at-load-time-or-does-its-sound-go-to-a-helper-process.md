Status: complete
Given: 2026-11-20, in conversation — transcribed by the round
Owes: none — option B (`quorra-sound` helper binary started lazily on the first sound, leaving `quorra` linking no audio library at load time) was already built in commit 75457369 (ADR 3317)

> Q972: Agree with you recommendation

Reading: adopts option B (`quorra-sound` helper process, already landed in commit `75457369` / ADR 3317) — keeps `quorra`'s launch path free of load-time audio library dependencies (`libasound.so.2`) and ensures GTK and other windows follow the same lazy helper pattern rather than linking `libasound` eagerly.
