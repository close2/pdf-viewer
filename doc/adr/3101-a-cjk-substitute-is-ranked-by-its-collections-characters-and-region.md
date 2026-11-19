# 3101 — A CJK substitute is ranked by its collection's characters, then by the region it names

Session 2100. Status: **accepted.** Context: ADR 1441 (repertoire before style), ADR 3066 (the coverage index), ADR
3067 (per-character borrowing), ADR 3068 (the JP face drawing Chinese), `crates/pdf-font/src/substitute.rs`
(`Collection`, `Repertoire::breadth`, `ranked_covering`, `name_languages`), `crates/pdf-font/src/provider.rs` (the
description's version 3). Clauses: ISO 32000-2 §9.7.3 (Table 114), §9.7.5.2 (Table 116), §9.10.2.

## Context

§9.7.3 has a `CIDFont` state its character collection in `/CIDSystemInfo` (Table 114), and Table 116 lists the
predefined `CMap`s under four headings — Chinese (Simplified), Chinese (Traditional), Japanese, Korean — whose
Unicode `CMap`s are each the "encoding for the Adobe-GB1 character collection", Adobe-CNS1, Adobe-Japan1 and
Adobe-Korea1; §9.7.5.2 adds Adobe-KR. So a collection names the language its characters are written in. One Han
character takes different forms in mainland China, Taiwan, Japan and Korea, and Noto Sans CJK ships a face per
region over one repertoire: its five faces each map 44 810 characters and state the same style and code pages.
`ranked_covering` broke that tie by catalogue order, face 0, Japanese, so every Chinese and Korean font drew
Japanese forms (ADR 3068's `issue19182`, `issue2128r`, `issue3521`, `issue8372`).

## Decision

- **`Style` carries the collection** (`Option<Collection>`, from `/CIDSystemInfo`, else a Japanese or Korean
  `/Lang`), because it does the job the style does: it decides among faces of one repertoire. It crosses the
  resource port as one byte (description version 3), so a confined worker's ranking is the broker's.
- **The region ranks after the repertoire and before the weight.** A face names its language by the Windows
  language identifiers its `name` table gives its family name in (Noto's JP face 0x0411, KR 0x0412, SC 0x0804, TC
  0x0404, HK 0x0C04; Microsoft's MS Mincho, SimSun, MingLiU and Batang the same way). Each collection's regions are
  ranked (CNS1: Taiwan, Hong Kong, Macao; GB1: China, Singapore), then the same primary language elsewhere, then
  any face. Before the weight, because a region's form is which character the reader sees and the weight is how
  dark it is; every Noto weight has every region, so on this machine nothing is traded.
- **A repertoire is counted against the collection's own `-UCS2` table where the font names one**: the characters
  such a font can show (§9.10.2's third method). A whole `cmap` count is kept for a font with no collection and
  for the interface's own words.

How a face states its language is not the standard's, and the `name`-table reading is a documented choice; a face
whose name table gives only English ranks after a face of the collection's language and ties with the rest.

## Consequences

- Chinese and Korean fonts draw their own region's forms; Japanese fonts are unchanged. `issue19182`, `issue2128r`,
  `issue3521` and `issue8372` move in the golden by design.
- The collection's table is read once per collection and each face's count once per collection, on the first
  question that needs it.
