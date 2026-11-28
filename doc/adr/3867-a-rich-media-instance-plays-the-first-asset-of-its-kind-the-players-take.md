# 3867 — A rich media instance plays the first asset of its kind the players take

Session 2291. Status: **accepted and built**; extends ADR 3855's decision 2. Code: `pdf-model`'s
`media/rich_media.rs` (`instances`, `asset`, `asset_type`, `Activated`). Clauses: ISO 32000-2
§13.7.2.3.2 to §13.7.2.3.4, Tables 341 to 343. Plan: `doc/todo/80` section 1 step 3.

## Context

ADR 3855 played the first instance of the configuration's kind and refused the press where its
asset did not play. Table 342 allows "an aggregate of several RichMediaInstance dictionaries with
different values for subtype"; Table 343's "The subtype shall match the asset file type of the
instance" binds the producer, and §13.7 names no asset format (RFC 0015 section 10.1).

## Decision

1. **Every instance of the configuration's kind is tried in order**, and the first whose asset the
   players take plays; each passed over is said with its reason, and where none plays the first
   instance's reason answers the press (which keeps ADR 3819's sentence and ADR 2399's files).
2. **Instances of another kind are not loaded**, said with their count: this program's players
   present one instance, of the configuration's kind.
3. **The asset's type is its Table 44 `/Subtype`, else its file name's extension** — the name
   §13.7.2.3.2 makes the assets tree's key, `/F` and `/UF` alike: `.wav` a RIFF WAVE file, `.m4a`
   an MP4 file of sound, `.m4v` one of video, `.mp4` the instance's kind's. Told from the name, it
   is said. A choice, not a reading.
4. **A mismatch is the table's**: a `Video` instance whose asset is a sound this program plays, or
   a `Sound` instance whose asset is an MP4 video, is said with Table 343's sentence and not played.
5. Played: WAVE and MP4 sound for `Sound`, MP4 `video/` (H.264, with its AAC) for `Video`; any
   other type is handed over as a file to save, by name.

## Tests

`rich_media.rs::the_first_instance_whose_asset_matches_its_kind_is_played`.
