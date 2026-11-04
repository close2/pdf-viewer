# 2679 — A stream is decoded through the filters it states, and no other

Session 1928, a single archive round.
Status: accepted; nothing built, a remedy declined (`doc/adr/2678` takes the loss instead).
Context: ADR 2678; `CLAUDE.md` principle 5; trap 5 (a silent fallback that renders something
plausible).
Clauses: ISO 32000-2 §7.3.8.2 Table 5 (`Filter`), §7.4.

## The question

`241573_ftp`'s object 67 states no `/Filter`, and its bytes inflate as Flate into a well-formed XMP
packet. An elegant remedy would try Flate on a metadata stream whose stated data is not text,
and keep the packet it finds.

## What the standard says

§7.4 opens: "The filter or filters for a stream shall be specified by the Filter entry in the
stream's dictionary (or the FFilter entry if the stream is external)." Table 5's `Filter` is optional and is
"The name, or an array of zero, one or several names, of filter(s) that shall be applied in
processing the stream data found between the keywords stream and endstream". Absent the entry, no filter is
applied, and the stream's data *is* the 532 bytes. Nothing in §7.3.8 or §7.4 lets a processor
infer a filter from the bytes.

## Decision

**No undeclared filter is tried, here or anywhere.** The data a stream holds is what its
dictionary says it holds. Inflating object 67 would turn bytes the file does not call a packet
into one this converter calls the producer's, and would write it back as metadata — the
convention of other readers rather than the standard. The bytes are a loss, and a profile that
states its losses states it (ADR 2678); a profile that loses nothing refuses by name.

## Reopening

Only a clause that defines inference of an absent filter, which ISO 32000-2 does not contain.
