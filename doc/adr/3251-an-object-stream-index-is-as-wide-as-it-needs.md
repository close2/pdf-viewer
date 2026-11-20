# 3251 — A cross-reference stream's third field is as wide as the largest member index

Session 2137. Status: **implemented.** Context: audit 2136 finding B04, ISO 32000-2 §7.5.8.2 `/W`, §7.5.8.3 Table 18.

## 1. The finding

Table 18's type 2 field 3 is "\[t]he index of this object within the object stream". `serialize` wrote it in the
two bytes `/W [1 4 2]` gives a generation, with `unwrap_or(u16::MAX)`, and `ObjectStreams::Generate`'s
`max_objects` is the caller's and unbounded: in a stream of 70 004 members the last was stated at index 65 535.
This reader finds a member by its number and was unaffected; a reader that follows the index is not.

## 2. The decision

The field is sized, not refused. `/W`'s third width is the bytes the largest index needs, never fewer than a
generation's two, so every file of 65 536 members or fewer is written byte for byte as before, and a larger stream
gets three or four bytes. Refusing was the other answer the clause allows; it was not chosen because the caller
asked for the stream size, and §7.5.8.2 lets `/W` state any width.

`linearize.rs` writes its own `/W [1 4 2]` through `serialize::row` and is untouched here: another round owns that
file. Whether its object streams can pass 65 536 members is left for the round that next works in it.
