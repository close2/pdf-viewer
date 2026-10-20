# 2073 — The interop layer is built by CI on the Linux runner, with no cache, under a priced ceiling

Session 1618, the CI job RFC 0010 section 6's N0 left owed. Status: **accepted**; the job is
written and its YAML parsed, and **no run of it existed when this was written**. Context: ADRs 0450
(every job has a ceiling), 2024 (a ceiling is twice the longest healthy run), 2054 (the layer).

## 1. Decisions

1. **One job, `interop`, on `ubuntu-24.04`**, the image every other Linux job names. The layer
   names no Windows type (ADR 2054), so `windows-latest` buys nothing until a window exists (N3a).
2. **`actions/setup-dotnet@v5` with `10.0.x`**, the project's `net10.0`. v5 is the Node 24
   generation `actions/checkout@v5` already is; v6 (2026-07-16) changes the action's packaging and
   nothing this job uses.
3. **No cache.** The brief's hypothesis was a cache; the project references no package, so a
   restore fetches nothing and a NuGet cache would hold nothing. The SDK is the setup step's.
4. **A ceiling of 15 minutes, priced, not measured**: one SDK fetched and seventeen files with no
   dependency compiled is a minute or two on a healthy runner. ADR 2024's rule is twice the longest
   healthy run; there is none yet, so the first runs restate the number.
5. **Not in `snapshot`'s `needs`**: nothing the snapshot ships contains this library.

## 2. Consequences

The C# has never been compiled, so the first push that runs this job is its first compilation,
and a red run there is a finding about the declarations rather than about the job.
`header_and_interop_agree.rs` remains what holds the layer to the header on the machine the gates
run on.
