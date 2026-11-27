# 3823 — The methods past the thirteen are a list, not ledger rows

Session 2280. Status: **accepted**. Context: A1095 ("[b]uild the remaining projection methods ... as
their own follow-up rows"), `doc/PLAN.md` §5a, ADRs 3821 and 3822.

## Decision

The ledger has one row per numbered subclause of the standard, and its population is a checked
claim (PLAN §5a), so a projection method cannot be a ledger row without changing what the ledger is.
Each method is instead an entry of its own in `doc/todo/79`, ordered by how many projected systems of
the carried table state it, with its EPSG code and its Guidance Note 7-2 section. §12.10.4 stays
`partial`, its note naming the 185 systems and 34 methods and pointing at that list; each method built
moves the note, and the last moves the row to `implemented`. §12.10 is the aggregate above it.

The count is the table's own, and this is the command that prints it:

    python3 - <<'PY'
    import re, collections
    s = open('crates/pdf-model/src/geospatial/epsg/dataset.rs').read()
    conv = {int(c): int(m) for c, m in re.findall(r'Conversion\((\d+), (\d+),', s)}
    names = dict((int(c), n) for c, n in re.findall(r'Method\((\d+), "(.*?)"\)', s))
    use = collections.Counter(conv[int(c)] for c in re.findall(r'Projected\(\d+, ".*?", \d+, (\d+), \d+\)', s))
    for code, n in use.most_common(): print(n, code, names[code])
    PY

## Consequences

If the owner meant a ledger mechanism for sub-clause requirements, that is a change to PLAN §5a and
its checker, and a question; until then the list carries the order and the row carries the debt.
