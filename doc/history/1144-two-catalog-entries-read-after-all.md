# 1144 — Two catalog entries a later clause came to read, and three rows that owe a build

Round 1144. Four near-done `partial` rows in the §7.7/§7.11 catalog family, read against their
clauses. ADR 1119's test: `departed` needs one declined `shall` inside the clause with a deciding
ADR; a declined permission is implemented-on-permission, not departed (round 1131).

## The four rows, each left `partial`

- **§7.7.2 (catalog, Table 29).** The refusal-to-repair (`update::the_catalog_reaches` refusing
  insert/delete over a catalog whose `/Pages` does not resolve) is a writer **honouring** Table 29's
  "Required; shall be an indirect reference", not a declined `shall` — not a departure. The residue
  is unread reader entries, and the "genuinely unread" list was stale for two: `/AF` and
  `/DPartRoot` are read from the catalog after all — `attachment::attachments` reads §14.13.3's
  catalog `/AF` and walks §14.12.2's hierarchy from Table 29's `/DPartRoot` for §14.13.8's per-part
  `/AF` (ADR 1040). Corrected: residue six → four (`/SpiderInfo`, `/PieceInfo`, `/NeedsRendering`
  excluded, and document-level `/AA` §12.6.3 — no catalog reader; page `/AA` is read). Stays `partial`.
- **§7.7.4 (name dictionary, Table 32).** None of the six unread trees is quoted as a name-dict
  category (`/AP` §12.7.4.3 in scope and owed; `/JavaScript`, `/Renditions` excluded;
  `/IDS`/`/URLS`/`/AlternatePresentations` deprecated). `/AP` owes a build → `partial`.
- **§7.11.3 (file spec, Table 43).** `/AFRelationship` writing only `Unspecified` (the default),
  declining the other seven because none can be established, honours an Optional entry — not a
  declined `shall`. Reader residue `/Thumb` (Table 43's, unread; collection.rs reads Table 159's)
  and `/EP` (unread; §7.6.7/§12.3.5's payload presentation) each owe a build → `partial`.
- **§7.7.3.3 (page objects, Table 31).** `/PZ` is a declined permission (§14.10.6 `may`), not a
  `shall`; the seven other unread entries are named in `PAGE_ONLY_ENTRIES` for damage
  discrimination — key, not value. No single declined `shall` → not `departed`; residue owed → `partial`.

No status moved; §7.7 aggregate unchanged (children still owe, ADR 1035 §4).

## Measurement and gates

- Readers confirmed in code (trap 8): `attachment::attachments` reads catalog `/AF` and the
  `/DPartRoot` walk; no reader for name-dict `/AP`/`/EP`/Table 43 `/Thumb` or the seven page values.
- `cargo test -p conformance`: FAILED — but only on the sibling `pdf-colour` extraction (ledger
  §10.x/§11.x `code` still names moved `pdf-model/src/{colour,icc,mesh,shading,transfer}.rs`); no
  §7.7/§7.11 row in the failure list, my TOML parses and this row's paths all exist.
- `pdf-model --test corpus` (gates): exit 0; ratchets 0/10/1/5/61 all at ceiling, unchanged.
- `raster_golden` (gates): exit 0; held 974, moved 0 — a text-only edit to reader/structure rows.

Only `doc/conformance/ledger.toml` changed (one line of the §7.7.2 note). No new ADR — no departure decided.
