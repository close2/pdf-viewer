# 3284 — A `/JBIG2Globals` entry that is not a stream is reported

Status: accepted. Session 2145. Context: ISO 32000-2 §7.4.7 (Table 12), §7.3.9; audit 2142 finding D03.

## The defect

`jbig2_bilevel` reported a globals stream that would not decode, because without its symbol dictionary the image
decodes to blank or to nothing, but skipped without a word a `/JBIG2Globals` present and of another type.

## Decision

Table 12 gives the entry the type `stream`. An entry of any other type — a dictionary, a number — carries none of the
global segments either, so it is the same loss and is reported the same way, as `ImageError::Malformed` naming the
type found. A direct null or a reference to an undefined object is §7.3.9's omitted entry, and the image is decoded
with no global segments, as for no entry at all.

## Cost

An image whose `/JBIG2Globals` is mistyped is refused where it was decoded without globals; that decode could only
have been right if the image's own segments referred to no global segment, and a file that wrote the entry says they
do.
