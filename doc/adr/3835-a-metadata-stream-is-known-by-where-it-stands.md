# 3835 — A metadata stream is known by where it stands, and states its type

Session 2283. Status: **accepted**. Context: review 2282 finding B04; ISO 19005-2 section 6.6.2.1,
ISO 19005-4 section 6.7.2.1 (paraphrased); ISO 32000-2 §14.3.2 Table 347, §7.7.3.3 Table 31.

## The finding

The metadata rows' population was the streams stating `/Type /Metadata`. A page's `/Metadata`
stream that omits that required entry is still "\[a] metadata stream that shall contain metadata for
the page"; its packet was read by no row, and nothing reported the missing `/Type`.

## What changed

1. **The population** (`metadata_population`, cached on `Examination` and shared with the public
   `metadata_streams` the converter reads) is every stream stating `/Type /Metadata` together with
   every stream a `/Metadata` entry of a cross-referenced object names, at any depth of that
   object's own values, bounded. Every packet row reads it.
2. **A new row, `metadata/metadata-streams-state-their-type`** (both parts' 6.6.2.1 / 6.7.2.1, by
   way of the base standard), reports a metadata stream other than the catalog's that does not
   state `/Type /Metadata`; the catalog's stays `metadata/catalog-metadata-stream`'s, which reads
   `/Subtype` as well. `pdf_archive::coverage` carries it under each part's every-metadata-stream
   sentence.
3. **The converter's answer** is mechanical, `Rewrite::MetadataStreamType`: the stream states
   `/Type /Metadata` and nothing else of it changes. A stream whose packet another metadata rewrite
   writes gains the entry from `metadata_stream`, which states it on every packet it writes.

## Why a row rather than section 5.1's

`conformance/adheres-to-the-base-standard` reads the reader's repair log, and the converter maps
each of its findings to a repair class (ADR 3477, ADR 3506). A missing required entry is no repair,
so a finding of that row with no class behind it would be one the converter could not answer.
