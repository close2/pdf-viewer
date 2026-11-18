# 3066 — Every face of a machine font collection is a candidate, and a face crosses as one program

Session 2091. Status: **accepted.** Context: ADR 3051 §2 (the defect), ADR 0141 (a collection a document
embeds), ADR 0880 (the resource port), `crates/pdf-font/src/substitute.rs`, `crates/pdf-font/src/collection.rs`,
`crates/pdf-font/src/provider.rs`. Clauses: ISO 32000-2 §9.10.2, §9.9 (Table 124); ISO/IEC 14496-22 §4.6.

## Context

`fonts-noto-cjk` installs four files, each a `ttcf` collection of five or ten faces, and no single-face file. The
catalogue admitted `.ttc` and `.otc` by extension, and every reader of a catalogue entry — the coverage check, the
qualifying search, the style read and the loader — opened it with `skrifa::FontRef::new`, which refuses a
collection. So the faces were listed and never offered, and the only face this machine offered for Japanese was
Droid Sans Fallback (ADR 3051 §2).

## Decision

1. **A face is addressed by a file and an index** (`substitute::MachineFace`). The catalogue reads the first twelve
   bytes of each `.ttc`/`.otc` and lists each of its faces (the count is bounded at 64); every other file is face
   zero. Ranking, the family walk and the coverage search carry the pair; a tie is broken by path and then index.
2. **The index stops where the file is read.** `read_cached` and the broker (`provider::open_a_face`) both call
   `face_program`, which copies a collection's face out as a standalone `sfnt` with ADR 0141's `rebuild`
   (`collection::extract_at`). What a caller holds is then one font program, whatever the file was: a page draws
   from it, the confined worker is handed it, and an archive that embeds a machine face writes it under Table
   124's `/FontFile2` or `/FontFile3`, which a collection is not. The alternative — `FontRef::from_index` at every
   reader — would put an index that is zero for every font but these into each of this crate's `FontRef::new`
   sites and every consumer of a substitute's bytes, for the same reason ADR 0141 declined it.
3. **The port's wire does not change.** The request carries a description as before; the answer is the face's
   program, never the file, and its identity names the face (`NotoSansCJK-Regular.ttc, face 2`). Its bound
   (`confined_transport::frame::MAX_RESOURCE`, 256 MiB) is above any face: Noto Serif CJK's largest file is 26 MiB
   and one face of it less.
4. **The coverage search reads each file once into an index** of each face's `cmap` as ranges, its mapping count
   and its style (`REPERTOIRES`), replacing the memo keyed by the characters asked. ADR 3067 asks one character at
   a time, and the search the memo cached read every font file per distinct question (215 ms on 1 400 faces, the
   figure the memo's own comment measured); the index pays that once.

## Consequences

- With `fonts-noto-cjk` installed, a Japanese, Chinese or Korean font with no program is drawn from Noto CJK where
  it maps more characters than Droid Sans Fallback does, which is the ranking's first key and unchanged.
- Within one collection, the region faces (JP, KR, SC, TC, HK) map the same characters, so the first in index order
  — the JP face in Noto's files — answers every CJK collection. Choosing the region face by §9.7.3's ordering is a
  separate question, not taken here.
- A copied-out face carries zero table checksums, as ADR 0141's do.
