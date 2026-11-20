# 3229 — A signature dictionary's entries are read as Tables 255 and 258 type them

Session 2132. Status: accepted. Context: review 2128 findings B04, B05, B06; ISO 32000-2 §12.8.1,
§12.8.2.3, §12.8.5.2, Tables 255 and 258.

## Decision

1. **A `/ByteRange` that is not pairs of non-negative integers describes no range.** Table 255:
   "An array of pairs of integers (starting byte offset, length in bytes) that shall describe the
   exact byte range for the digest calculation." `byte_range` refuses the whole array — an odd
   member, a member of another type, a negative one, or more than `MAX_BYTE_RANGE_PAIRS` pairs —
   and the empty answer reads `Coverage::Malformed`. It used to drop the offending member and read
   the pairs left, which could answer `Coverage::WholeFile` for a range the file never stated.
2. **A usage-rights `/V` is understood only absent or as the name `2.2`.** Table 258: "If an
   unknown version is present, no rights shall be enabled. NOTE This value is a name object, not a
   number." A number, a string or any other value present is an unknown version; `/V null` is
   absent by §7.3.9.
3. **A document timestamp is held to the whole file by its `/Type`.** Table 255's `Contents` row
   ("For document timestamp signatures, ... the ByteRange shall specify the complete PDF file
   contents") and §12.8.5.2 key the `shall` on the dictionary being a document timestamp; the
   sub-filter `ETSI.RFC3161` is what such a dictionary "should" state. `must_cover_whole_file`
   is now `/Type /DocTimeStamp`, or `/SubFilter` `ETSI.CAdES.detached` or `ETSI.RFC3161`.

## Why

Each of the three answered a question from less than the file stated: a shortened array, a type
the table's NOTE names as wrong, and a recommendation read as the condition. In each the clause
states the condition, and the code now asks it.

## Cost

A file whose `/ByteRange` carries a stray trailing member now reports a malformed range rather
than verifying; that is what the table's "shall describe the exact byte range" says of it.
