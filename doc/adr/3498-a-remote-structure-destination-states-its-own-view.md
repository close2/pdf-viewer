# 3498 — A remote structure destination states its own view, and an empty element is page one

Session 2199. Status: accepted. Context: review 2197 (finding 2197-B02); ISO 32000-2 §12.3.2.3,
§12.6.4.3, Table 203, Table 149.

§12.3.2.3: "using that page, the structure destination shall behave identically to a
destination", so the rest of Table 203's `/SD` array after the element's identifier is Table
149's name and parameters, read by the same `View::read` an explicit destination's array is.
`RemoteGoTo` carries it as `structure_view`, and `RemoteGoTo::page_in` answers the page and the
view together from the one entry that decided: `/SD`'s where its element is found in the remote
document's `/IDTree`, `/D`'s otherwise. ADR 2964's filter — `/D`'s view applied only where `/D`
named the page — is now this pairing, so `/D`'s view is never shown on `/SD`'s page and `/SD`'s
own view is no longer dropped.

§12.3.2.3 also says "\[i]n the case where no page content is identified, then the page reference
shall be assumed to be the first page in the document". An element the identifier finds without
content therefore answers page 0 of the remote document, as `Destination::resolve` answers locally;
`/D` is the fallback only where no element is found at all, which is where Table 203's `should`
leaves a reader.
