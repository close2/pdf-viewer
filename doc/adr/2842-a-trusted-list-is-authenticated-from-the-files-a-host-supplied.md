# 2842 — A trusted list is authenticated from the files a host supplied

Session 2012, a signatures round (`doc/questions/A800` item 3, `doc/todo/77` item 1). Status:
implemented. Rows: §12.8.1, §12.8.3.4.4, §12.8.3.4.5. Context: ADRs 1085, 1728, 2834, 2835, 2843;
ETSI TS 119 615 V1.3.1 clauses 4.0 to 4.2; ETSI TS 119 612 V2.3.1 clauses 5.3.7, 5.3.13, 5.7 and
Annex B. The ETSI texts are cited by clause and paraphrased (ADR 1085).

## Context

ADR 2834 read a Member State's list and believed it on the word of the host that supplied it,
because TS 119 615 clause 4.1 (the European Commission's list of trusted lists, LOTL, against the
Official Journal's certificates, through its pivots) and clause 4.2 (each Member State's list
against the certificates the LOTL's pointer for its territory names) were not built. Both
procedures name every file by the location it is *obtained* from, and GPR-4.0-02 makes three
parameters — `OJEU-Loc`, `OJEU-LOTL-Loc`, `OJEU-LOTL-Certs-Set` — a configuration. This crate opens
no socket, and the fetch is a host's transmission not yet built (`doc/todo/77` item 5).

## Decision

1. **A file obtained from a URI is the supplied file named the URI's last path segment** — the
   name a copy saved from it carries (`eu-lotl.xml`, `eu-lotl-pivot-341.xml`). The LOTL is the file
   `OJEU-LOTL-Loc` names; each pivot is the file its scheme information URI names.
2. **A Member State's list is the supplied list whose scheme territory is the country code.**
   Clause 4.2's input is the country code, and the location only says where to fetch it; what
   authenticates the list is its signature under a certificate the authenticated LOTL names for
   that code, which no file name can forge. Names would collide where two Member States publish
   under one file name.
3. **The configuration is files named after its parameters**: `OJEU-Loc` and `OJEU-LOTL-Loc` (a
   URI each), and every file whose name begins `OJEU-LOTL-Certs-Set` (a DER certificate, or PEM
   blocks). The first two default to the values GPR-4.0-02 NOTE 1 records; the certificates have
   no default, because the standard does not print them.
4. **The procedures run where the host asked for them**: it supplied the LOTL's file or any of the
   three parameters. Then a list they do not authenticate is refused by name, with the clause's
   own indications, and is not searched; the LOTL and its pivots (territory `EU`) are clause 4.1's
   inputs and are not searched either. Where the host supplied neither, nothing could be
   authenticated, and ADR 2834's footing stands: every list is `VouchedForByTheHost`, and the report
   says that neither the LOTL nor the Official Journal's certificates were among the files.
5. **A file the procedure needs and the host did not supply** is a failure with an indication of
   this program's own, `<location> NOT_SUPPLIED`, because the clauses state none for a file that
   could not be obtained — except the LOTL's pointer to itself (PRO-4.1.4-05), whose absence
   GPR-4.0-03 makes a warning: a fetch that could not be validated does not stop the procedure.
6. **The host's clock comes with the lists.** PRO-4.1.4-13's "has passed" and the verdict are asked
   at the instant the anchors carry; a host given lists and no anchor directory now carries its
   clock in the anchors' supply rather than the epoch.

## What is not done, and why

- PRO-4.1.4-16 and -17 update the configuration from an authenticated LOTL; this crate keeps no
  state between readings, so that is a host's to persist.
- The list's signing certificate is not checked against clause 5.7.1's profile (key usage, the
  territory in the subject): clauses 4.1 and 4.2 validate it as a trust anchor, and RFC 5280
  validates nothing about an anchor.
- No host fetches the LOTL (`doc/todo/77` item 5), and a C host supplies no lists (item 7).

## Consequences

A report built on an authenticated list names the LOTL's sequence number and drops ADR 2834's
"believed because it came from the trusted lists you supplied"; one built on a list that was not
authenticated keeps it. The eIDAS level stays undetermined (ADR 2835).
