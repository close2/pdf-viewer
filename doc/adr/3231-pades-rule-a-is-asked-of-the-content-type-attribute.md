# 3231 — PAdES rule (a) is asked of the content-type attribute

Session 2132. Status: accepted. Context: review 2128 findings B03, D03; ISO 32000-2 §12.8.3.4.3 (a);
RFC 5652 section 11.1.

## Decision

1. §12.8.3.4.3: "The attributes in the SignedData object ... shall obey the following rules: a)
   content-type: shall be present and shall always have the value "id-data"." `pades_departures`
   reads the **signed** content-type attribute — RFC 5652 section 11.1: "The content-type attribute
   MUST be a signed attribute or an authenticated attribute" — and reports
   **`PadesDeparture::NoContentTypeAttribute`** (a new variant) where there is none.
2. **`ContentTypeIsNotData`** now covers the value half: the attribute present and not stating
   `id-data` as its one value, or the `eContentType` it describes not `id-data` (the check that
   was there before, kept).
3. `ProfileAttribute`'s comment, which said presence was already required, now names the two
   departures that check it.

## Why

Rule (a) is about an attribute; the code compared only `encapContentInfo`'s `eContentType`, so a
value with no content-type attribute met every rule it was asked. A new variant rather than a
widened one, because "absent" and "wrong" are two different things a producer did, and the report
words them apart.
