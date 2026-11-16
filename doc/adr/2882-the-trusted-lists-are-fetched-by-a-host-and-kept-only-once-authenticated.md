# 2882 — The trusted lists are fetched by a host, and kept only once authenticated

Session 2033, a signatures round (`doc/questions/A800` item 3, `doc/todo/77` item 4). Status:
implemented. Rows: §12.8.3.4.4, §12.8.3.4.5. Context: ADRs 1085, 1291, 1728, 2834, 2842, 2843,
2874; `doc/questions/A98`; ETSI TS 119 615 V1.3.1 GPR-4.0-02 and -03, clauses 4.1 and 4.2, cited by
clause and paraphrased (ADR 1085).

## Context

ADR 2842 authenticates the lists a person saved into the `--trusted-lists` directory and left the
fetch to a host: TS 119 615 names every input by the location it is *obtained* from, and
PRO-4.1.4-16 and -17 update GPR-4.0-02's configuration from each authenticated LOTL, which needs
state between readings. ADR 1728 is the precedent for a fetch a host makes for a signature's
validation: A98's client, its `Submissions` level, `ask` by default.

## Decision

1. **The procedure is `pdf-signature`'s and the transmission the host's.**
   `trusted_list::refresh` is handed a transport (a location in, octets or a sentence out) and asks
   it for exactly what clauses 4.1 and 4.2 obtain: PRO-4.1.4-01's LOTL, -04's pivots and -05's
   pointer to itself, then PRO-4.2.4-03's list for every territory the authenticated LOTL points at
   with an XML list. It answers with files to store, files to remove and a fate for each location;
   it opens no socket and writes nothing. The confined worker is untouched (principle 3).
2. **Nothing fetched is kept before it is authenticated.** Clause 4.1 runs over the fetched files
   in front of the host's; where it fails nothing is stored, and no Member State's list is asked for
   (PRO-4.2.4-02). A Member State's list is stored only where clause 4.2 passes on it; one that
   fails, or that could not be fetched, leaves the held copy in place. The stored files are
   authenticated again whenever they are read (ADR 2842), so storing earns no belief.
3. **A fetched file is stored under its location whole**, every octet outside RFC 3986's unreserved
   set percent-encoded (`authentication::stored_name`), and that name is looked for before ADR
   2842's last path segment. The last segment alone collides: three Member States' lists of
   2026-11-12 end `tsl.xml`, and a refresh would overwrite two of them. A person's hand-saved copy
   keeps working by its last segment.
4. **A stored file the authenticated LOTL no longer leads to is removed** — only names that are a
   stored `http`/`https` location, which no hand-given name begins with — so a list that moved is not
   searched twice under two LOTLs.
5. **PRO-4.1.4-16 and -17 are persisted as the configuration's own files.** -16 writes
   `OJEU-LOTL-Loc`. -17 writes `OJEU-Loc` and replaces the whole `OJEU-LOTL-Certs-Set` with b) 1)'s
   certificates, the `EU` pointer's of the pivot immediately following the new URI, as
   `OJEU-LOTL-Certs-Set-<n>.der`, removing the other certificate files. b) 2) is the Official
   Journal's publication, a document for people, which this program does not read; where b) 1) has
   no pivot, neither half is made — a moved `OJEU-Loc` beside the old set would authenticate the next
   LOTL against certificates the clause replaced — and the sentence says so.
6. **The level is `Submissions`'**, and no fifth is invented: a GET to the Commission's or a Member
   State's server tells it signatures are being checked here, ADR 1728's act class.
   `restriction::SUBMITTING` names the lists. Without a `--trusted-lists` directory the fetch is
   refused at every level, since there is nowhere to keep the lists. A refresh without an
   `OJEU-LOTL-Certs-Set` file asks for nothing, because PRO-4.1.4-12 could authenticate nothing.
7. **Bounds**: `MAX_FETCHES` (256) locations a refresh, a stored name of at most 255 octets, and
   `submit::fetch`'s timeout, response limit and refusal to follow a redirect.

## Evidence

Over the lists saved on 2026-11-12 (never tracked; this session worked in `~/scratch/agent/r2033/`), with pivot
282's signer standing in for the Official Journal's set, a refresh authenticates the LOTL through
five pivots, keeps Austria's and Germany's lists, says the 29 not saved as not fetched, and moves
`OJEU-Loc` to `https://eur-lex.europa.eu/eli/C/2026/1944/oj` with pivot 378's six certificates. A
second refresh from that configuration passes with no pivot and removes the five.

## What is not done, and why

- **No window calls it.** The act, its key, the question, the thread and the directory handed again
  to the core as `Command::Trust` are effort across five hosts and the C ABI (`doc/todo/77` item 4).
- **GPR-4.0-03's *should*** (go on past a failed `https` validation with a warning) is not taken:
  the client has no unvalidated mode, and a connection that fails is said as not fetched.
- A list a policy's `TAsInTrustedList` references outside the LOTL's tree is fetched by nothing.
