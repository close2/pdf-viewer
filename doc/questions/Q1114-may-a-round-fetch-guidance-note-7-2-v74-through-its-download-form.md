# Q1114 — May the current Guidance Note 7-2 be fetched through IOGP's download form?

Asked by round 2293 (ADR 3875). This does not block anything: the two methods stay refused by name
until it is answered.

**Situation.** `doc/md/IOGP-373-07-02.md` is the September 2019 revision. Two of `doc/todo/79`'s
rows are not in it: Azimuthal Equidistant (EPSG 1125; section 3.3.3 is a heading only) and Lambert
Conic Conformal (1SP variant B) (1102). epsg.org lists "Guidance Note 7-2 (ver 74, Jul 2026)". Its
only download is a POST to `idms.iogp.org/Documents/DownloadPublic` that asks for a first and last
name, a company, an email address and a country. A round may not send the owner's details to an
outside service.

**Question.** Will the owner fetch v74 into `doc/` (or say which details a round may submit)? Or
should 1125 and 1102 stay refused until a document needs them?
