# 3250 — A compressed object ends where the next one begins, and is found by its number only

Session 2137. Status: **implemented.** Context: audit 2136 findings B01 and D01, ISO 32000-2 §7.5.7 NOTE 6 and NOTE 7,
Table 18, §7.3.10, ADR 0366.

## 1. The findings

§7.5.7 NOTE 7 (2020): "processing of each object in an object stream starts at the specified byte offset in the
decompressed stream and ends prior to the byte offset of the next object or when the end of stream is
encountered", because the 2020 edition removed the white-space between members. `expand_object_stream` computed
that end (ADR 0366 uses it to decide what a truncated stream lost) and then parsed over the whole data, so `57`
stored as object 10 at 0 and object 11 at 1 read as 57 (B01). `load` fell back to `expanded.values().nth(index)`
where the stream held no member of the entry's number; the map is in object-number order, which NOTE 6 says need
not be the stream's, so the fallback handed back another member's value under this number (D01).

## 2. The decision

The parser is given the bytes up to the stated end and no further. The fallback is removed: an entry whose
stream holds no member of its number reads as no object, which §7.3.10 makes null — the same answer the offset
branch gives where the object at an offset is not the one named, short of its header search, which an object
stream does not have.
