# 3525 — A confinement probe passes only on the mechanism that should refuse

Session 2206. Status: accepted. Context: `crates/pdf-sandbox/tests/confinement.rs`; ADRs 1608,
3297, 3514; trap 13.

## Context

Every probe of the script and media profiles, and the decoder's `open`, `socket` and `fcntl`,
passed on "a refusal or the filter's kill": an exit the probe chose after its call failed counted
the same as `SIGSYS`. Round 2203 found that under `Profile::Media`'s 64 MiB `RLIMIT_AS` the
harness thread's allocator arena leaves no room, so a probe's own allocation can fail with
`ENOMEM` before the forbidden call is made. Measured here, with the filter unchanged and the old
assertion: `media-thread` and `media-exec` ended `exit 17` (the probe's refusal), never reaching
`clone`/`mprotect`; every other probe ended `SIGSYS`. With a planted filter that admits each probed
call (`openat`, `socket`, `bind`, `fcntl`, `mprotect`, the thread calls; Media's descriptor ceiling
one higher):

| probe | planted outcome, old probe | seen by the old assertion |
|---|---|---|
| `open`, `script-open`, `media-open` | exit 17: Landlock's `EACCES` | no |
| `script-socket` | exit 17: `RLIMIT_NOFILE` 0, `EMFILE` | no |
| `media-socket` | `SIGSYS` from `close` when the bound socket dropped, after `socket` and `bind` were served | no |
| `media-thread`, `media-exec` | exit 17: `ENOMEM` (without one arena) | no |
| `fcntl`, `socket`, `script-fcntl`/`-thread`/`-exec`, `media-fcntl` | exit 18 | yes |
| `media-ring-second` | exit 17 whatever stopped the second descriptor | no |

**No hole.** No probed call is on any profile's list (`lockdown_linux.rs`'s `PERMITTED`,
`PERMITTED_SCRIPT`, `PERMITTED_MEDIA_RING`): with the filter as it is, every probe ends `SIGSYS`
under one arena. The probes were blind to a widening, not passing over one.

## Decision

1. `assert_the_filter_kills` replaces `refused`: only `SIGSYS` passes. Landlock and `RLIMIT_NOFILE`
   stay defence in depth, each now seen in the table above rather than counted as the filter.
2. Every probe runs under `MALLOC_ARENA_MAX=1` (`run_probe`).
3. What a served call returns is forgotten, never dropped: neither profile admits `close`.
4. `media-ring-second` passes only on `MSG_CTRUNC` with no descriptor after the first arrived; any
   other path exits 20.

## Consequences

The planted filter fails all six probing tests, and the unplanted file passes 16 of 16. A profile
widened later fails its probe at the call widened.
