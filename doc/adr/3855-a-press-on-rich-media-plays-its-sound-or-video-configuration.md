# 3855 — A press on rich media plays its `Sound` or `Video` configuration

Session 2288. Status: **accepted and built**; replaces ADR 3819's sentence for a rich media
configuration the players take. Code: `pdf-model`'s `media/rich_media.rs` (`activated`);
`viewer-core`'s `interact/rich_media.rs` (`pressed`, `plays`), `interact/deprecated_media.rs`
(`exhibited`) and `viewer/pointer.rs` (`exhibit`). Clauses: ISO 32000-2 §13.7.2.1, §13.7.2.2.2,
§13.7.2.3.3, §13.7.2.3.4, Tables 335, 341 to 343. Builds on RFC 0015 section 10.1, A946, A1006.

## Context

ADR 3819 answered every press on a rich media annotation with a sentence and its assets handed out
as files (ADR 2399), because the `Sound` and `Video` configurations were not built.

## Decision

1. **The configuration** is Table 335's `/Configuration`, else "the first element within the
   Configurations array". **Its kind** is Table 342's `/Subtype`, else the first instance's — the
   table's "the run time shall determines the scene type by referring to the type of asset file
   specified by the first element in the Instances array", read through Table 343's "the subtype
   shall match the asset file type". **The instance** is the first of that kind.
2. **The asset's type is its embedded file's Table 44 `/Subtype`**, the reading a movie's file has.
   This is a choice: §13.7 names no asset format (RFC 0015 section 10.1), and step 3 of
   `doc/todo/80` section 1 matches instance to asset further. A `Sound` asset plays where it is a
   RIFF WAVE file or an MP4 file of sound alone and the host plays sound; a `Video` asset where it
   is an MP4 file of `video/` type and the host shows video. It plays in the annotation's
   rectangle — Table 338's default `Embedded` — with the default play parameters.
3. **Refused by name**: the `3D` configuration ("rendering U3D and PRC artwork is excluded"), a kind
   Table 342 does not define, no instance of the kind, an asset not embedded, of a type the players
   do not take or of no stated type. The press then keeps ADR 3819's sentence and ADR 2399's files.
4. **Said as set aside where a played activation states them**: Table 335's `/Scripts` (the rich
   media run time's ECMAScript, ISO 21757-1, excluded) and, until their steps, `/Animation`,
   `/View` and `/Presentation`.
5. **A press that plays hands no file out**: the asset is the clip, and it stays listed among the
   document's attachments (`Query::Attachments`) to be saved from there. A press under `PO` or `PV`
   still activates: a press is always a person asking (ADR 3819), and `/OP` 4's association keeps
   it from starting twice.

## Tests

`rich_media.rs`: `a_sound_or_video_configuration_the_players_take_is_played_here`,
`three_d_and_unknown_configurations_are_refused_and_the_named_one_is_loaded`;
`viewer-core/tests/rich_media_activation.rs`: `a_press_plays_a_video_configuration_and_hands_no_file_out`,
`a_three_d_configuration_is_said_and_a_sound_configuration_plays`.
