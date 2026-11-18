# 2091 — Machine font collections are opened, and a CJK font borrows the face for what its own lacks

Session 2091, a pixels round in `pdf-viewer-audit` on ADR 3051's two `pdf-font` defects. ADRs 3066 to 3068;
3069, 3070 and Q913 unused.

**Premises, both held.** `fonts-noto-cjk` installs only `.ttc` files and every catalogue reader opened a file with
`FontRef::new`, which refuses a collection; `composite_substitute` chose a CID font's face by the script sample
alone, so Droid Sans Fallback (no Basic Latin) drew none of `issue15053`'s 29 and `mixedfonts`' 74 characters.

**Defect 1 (ADR 3066).** A face is `substitute::MachineFace { path, index }`. The catalogue reads a collection's
`ttcf` header and lists each face (bounded at 64); `read_cached` and the broker copy the face out with ADR 0141's
`rebuild` (`collection::extract_at`), so every holder of a substitute's bytes holds one font program and the wire
does not change. The coverage search is an index of each face's `cmap` ranges read once (`REPERTOIRES`), replacing
the memo by characters asked. Test `each_face_of_a_collection_is_read_at_its_own_index`; planted (index ignored),
it failed with face one regular.

**Defect 2 (ADR 3067).** The script still chooses the font's face; a character §9.10.2 gives that the face does not
map is asked for alone, by the same request, style and walk, and drawn from `Face::Borrowed`. A per-character answer
is a function of the character and the machine, so pages drawn in any order agree, where a choice by the loading
page's characters would not (a `LoadedFont` outlives its page). `uncovered_character` asks the same. The port keeps
one copy of a program that answers several keys. Test `tests/borrowed_face.rs`: a machine of two built faces (あ
only; Latin only) behind the resource port, "This Message" through `KozMinPr6N-Regular`/`UniJIS-UTF16-H`; planted
(no borrowing), `T` was uncovered.

**Walks.** `corpus` 0: 65 incomplete against a ceiling of 65, `issue15053` and `mixedfonts` gone, ratchet list
unedited. `raster_golden` update run: held 896, moved 78. The fifteen held font pages are restated, spliced alone
from that run, each drawn by Noto Sans CJK JP (Bold for bold); the table is ADR 3068. Of the other 63, 57 moved
identically in the train's run at 66c33750 and six arrived with round 2087's fb3699c3; none restated.
`issue19389` holds.

**Left, in order.** The 63 unrestated moves, by their owners (ADR 3068); a region face for `Adobe-GB1`, `-CNS1` and
`-Korea1` (Noto's region faces tie, so JP draws Chinese); Mincho fonts drawn by Noto Sans CJK because repertoire
ranks before family (ADR 1441); a font whose sample no face maps is still refused whole (ADR 3067).

**Gates.** Over `-p pdf-font -p pdf-model`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` 0 (3295 passed; the manifest expects none, difference empty), rustdoc `-D warnings
--document-private-items` 101 then 0 after two links, `fuzz` `cargo check --bins` 0. `cargo test -p conformance`
101 (two `§` after ISO/IEC 14496-22, a stale `covering_path`, this record's gates line), 0 for those after the fix.
