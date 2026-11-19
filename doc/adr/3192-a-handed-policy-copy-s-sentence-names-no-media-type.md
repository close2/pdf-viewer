# 3192 — A handed policy copy's sentence names no media type it was not given

Session 2122. Status: accepted. Context: ADRs 1753, 3169; round 2117.

## Decision

`viewer_host::policy::bound` ended a bound non-PDF copy's sentence with "its media type is above".
On the fetched path (`signature_policy_fetched`) that is true: the sentence printed the server's
status and media type. On the handed path (`signature_policy_handed`), where a C caller fetched the
copy, no media type was printed, because this program never saw the response. The tail now depends on
`Onward`: a fetched copy points at the media type; a handed one says the caller that fetched it was
told what kind of document it is and this program was not. A sentence may not refer to something it
never said.
