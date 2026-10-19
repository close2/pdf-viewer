# 2021 — A destination profile's header is restated as what its own bytes are

Session 1592, the archive slot of batch ninety-four. Status: **accepted**, **built**. The two
`not-built-yet` sites the veraPDF census ranks next at 2b, `graphics/destination-profile-carries-
the-tags-its-class-requires` and `graphics/destination-profile-states-a-correct-profile-id`, both
reached only by `6-2-3-t01-fail-d.pdf` (a `PDF_A-2b` and a `PDF_A-4` copy). Code:
`crates/pdf-transform/src/archive/destination.rs`, `pdf_archive::icc_written_in_the_1998_edition`.

## 1. The witness, read in its bytes

Its destination profile is a display-class RGB matrix-and-curve profile (Adobe RGB (1998)) whose
tag types are all ICC.1:1998-09's — `desc` a `textDescriptionType`, `cprt` a `textType` — under a
header stating version 5.0.0, which is ICC.2:2023. The validator holds a destination profile to
the edition its header names (ISO 19005-2 and ISO 19005-4 section 6.2.3 ask for a valid ICC
profile stream), so it finds what ICC.2:2023 sections 8.4 and 8.5 require of the class — a table to and
a table from the connection space — missing, and a `Profile ID` that is not the digest of the
bytes. veraPDF calls the profile unsupported; the two readings agree.

**The brief's hypothesis held**: one remedy, two findings, because both turn on the header alone.

## 2. Why restating the header is a statement

`pdf_model::icc::Profile::parse` takes a profile's transform from its tags and reads the version
for one thing, whether a lookup table's CIELAB is version 4's encoding. So this reader drew the
witness through Adobe RGB (1998)'s matrix and curves, which is what the bytes say, and the header's
edition is the one claim in the profile the bytes contradict. Two rewrites state what is there:

- **A version 5 header on a profile written in ICC.1:1998-09's terms** — every tag that edition's
  clause 6.4 defines carries the type it gives (tags it does not define are private to it, which
  it provides for), and the tags its clause 6.3 requires of the class are present: the version is
  written 2.2.0, that edition's own number, and bytes 84 to 99 cleared, because that edition
  reserves them and a digest there names an edition the profile is not in.
- **A stale `Profile ID`** under an edition whose tags the profile carries: the digest of the
  profile by ICC.1:2022 section 7.2.18's method is written. No transform reads the field.

No tag moves, so no colour moves. Unlike ADR 2007's refusal to restate a class — a claim about a
device nobody made — this names the document the bytes already conform to, and the validator's
own reading of the result is the guard where the rewrite acts.

## 3. Refused by name

A tag missing under the header's own edition (supplying it would be writing the producer's
profile); a version 5 profile in any other edition's types (no edition the bytes are in to name);
a version 5 profile with a CIELAB connection space and lookup tables (restating the version would
change their encoding, and so the colour); and every case ADR 2007's one-object rule refuses.

## 4. Measured

Both copies of the witness convert — the first to 2b, the second to 4 — each page byte-identical at
72 dpi before and after, and veraPDF 1.30.3 calls both outputs compliant where it failed the sources
at 6.2.3.
