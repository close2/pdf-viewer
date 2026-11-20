# 3351 — A sound held at its start is played by the space bar

Session 2162. Status: accepted. Context: ADRs 3301, 3329, 3341, 3343, 3350; ISO 32000-2 §13.2.5's
Table 291 `/A`.

## Decision

Table 291's `/A` false: "the media shall be initially paused when activated". The player has held
such a sound at its start since ADR 3341, and nothing in a window could start it. Now **Space plays
it** in `quorra`, GTK and Qt, taken before the key table exactly as Escape is taken while a sound
sounds; the title says "a sound is held at its start, Space plays it and Escape stops it"
(`viewer_host::sound::PAUSED`). GTK's and Qt's windows go through `Sounds::resume`, `quorra`
through `resume_the_sound`; both answer whether a sound was held, so that Space keeps every other
meaning while none is.

**The key is a choice.** The specification names no control. Space is the key media players give
play and pause, and it is taken only while a sound is held, so a page that scrolls by Space loses
nothing while no sound waits. A held sound is a paused one, so Escape still stops it.

## Cost

Three small key branches and a method; a held sound costs the reader one key.
