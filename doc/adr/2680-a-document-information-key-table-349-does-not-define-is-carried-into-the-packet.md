# 2680 — A document information key Table 349 does not define is carried into the packet

Session 1929, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/config.rs`: `Unmapped::Carry`;
`archive/prepare/metadata.rs`: `CARRIED_INFORMATION_NAMESPACE`, `carried_local_name`,
`xml_carries`, `carried`, `Gained`). Profiles: `doc/profiles/state-every-loss.toml`,
`doc/profiles/no-data-loss.toml`. Tests: `crates/pdf-transform/tests/archive/carried_information.rs`.
Context: ADR 1269 (the move, and section 5's refusal this revisits), ADR 2358 (`unreadable`),
ADR 2678 (the packet rows' second step), `doc/questions/Q779` (the namespace URI).
Clauses: ISO 19005-4 sections 6.1.3, 6.7.2.1, 6.7.2.2, 6.7.2.3; ISO 32000-2 §14.3.1, §14.3.3,
§14.3.4, §7.3.5; ISO 16684-1 (a property name is an XML qualified name).

## The premise, checked

Under both stating profiles at part 4, the two section 6.1.3 rows refused 60 pdf.js documents and
80 corpus documents for a custom `/Info` key (round 1928's census), `241573_ftp` among them. Their
keys are producers' (`SourceModified`, `Company`, `PTEX.Fullbanner`, `WPS-ARTICLEDOI`, …); four
documents state a key holding a space (`Base Target`, `Form fields`, `Created using`).

## What the standards say

- ISO 32000-2 §14.3.3 deprecates the dictionary and says that "[f]or any other document level
  metadata, a metadata stream ... should be used instead"; the value of a key other than the two
  dates "shall be a text string". Table 349's NOTEs name a property for its own keys and none for
  any other.
- ISO 19005-4 section 6.7.2 requires the packet to be well-formed XMP. Unlike ISO 19005-2 section
  6.6.2.3, it does not require a property's schema to be predefined or described in the packet.
  Section 6.7.2.3 only recommends a schema file in the stream's `AF`. `pdf-archive`'s two schema
  rows are part 2's alone.

So the standard admits a custom key's value in the packet at part 4. It names no namespace for it.

## Decision

1. **`unmapped = "carry"`** writes the value, unchanged, as a simple property in this project's
   namespace `https://github.com/close2/pdf-viewer/ns/document-information/1.0/`, prefix `docinfo`.
   The namespace defines one fact only: the value is what the source's dictionary stated under the
   key that the local name spells. ADR 1269 section 5 refused a namespace because choosing one would
   decide what somebody else's key *means*. A namespace that only states *where the value came from*
   makes no claim about meaning. It is a documented choice (CLAUDE.md principle 5), not a convention
   presented as derived, and the URI is the owner's to confirm (Q779). `extension-schema` stays
   refused: its container is ISO 19005-2's, and these two rows are part 4's.
2. **The local name is an injective escape of the key's bytes.** An ASCII letter stands for itself,
   and so do a digit, `-` and `.` after the first character. Every other byte, `_` included, is
   written `_HH_`. The key is recovered byte for byte. The empty key has no spelling.
3. **What cannot be carried as stated** goes to the row's `unreadable` answer (`discard`, else a
   refusal by name). That is a value that is not a text string, a character XML 1.0's `Char`
   excludes, a carriage return that XML's end-of-line handling would turn into a line feed, or the
   empty key. Carrying one of these would change the value, or invent a spelling for it.
4. **Additive, as the move is**: a property the packet already states stands (§14.3.4), and the
   report names each key, its property and its fate (`moved_information`).

## Profiles

`state-every-loss` and `no-data-loss` name both rows with `remedy = "preserve"`, `unmapped =
"carry"`. The data is kept, so `no-data-loss` takes it too. A value that cannot be carried refuses
under `no-data-loss`; under `state-every-loss` it is removed and stated (ADR 2681). `as-shown`, `as-if-printed` and `only-metadata-loss` keep `discard`.
They authorise the loss, and changing them would change outputs that already convert.
`keep-everything` names no metadata site in its short form, and is unchanged.
