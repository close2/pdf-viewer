# 3751 — A file specification naming no file is logged

Session 2262. Status: **accepted**. Context: ADR 3477, ADR 3749; ISO 32000-2 §7.11.3 Tables 43
and 44.

## Context

`attachment.rs` held four sites: three readers named an attachment by
`FileSpec::display_name().unwrap_or_default()`, and `/Params` of the wrong type was read as empty.

## Decision

- Table 43 makes `/F` "Required if the DOS, Mac, and Unix entries are all absent". One function,
  `shown_name`, names an attachment: where none of `/UF`, `/F`, `/DOS`, `/Mac` and `/Unix` holds a
  string, the specification names no file, is shown with the empty name, and is logged as the new
  `model/file-specification-entry-unusable`. A specification that states the empty string has
  named its file so, and is not logged.
- Table 44 types `/Params` a dictionary; one of another type is read as absent, logged the same.
- Section 5.1 alone; the rewrite carries the specification as stated and the output's own check
  refuses by the class's name.

## Consequences

- `attachment.rs` held 4 → 0. The site is the file: the readers are handed the specification's
  dictionary, not its object.
