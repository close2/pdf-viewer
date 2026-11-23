# 3502 — The stream is tested through a confined witness under `tools/`

Session 2200. Status: accepted. Context: ADR 3501; `tools/batch.sh`'s `install_binaries` and
`tools/conformance/tests/batch.rs`'s `install_names_every_program_and_library_the_workspace_builds_for_a_person`.

## Decision

`tools/stream-witness` is a program that confines itself with `Profile::Media`, greets, answers a
ping, and streams what its request names — keeping the contract, over-sending, stalling, or sending
an item of an unknown carriage. Its tests start it through `Host` and hold flow control, a stop
mid-stream, both kept-worker budgets, an over-send, a stall ended by the wall clock, a malformed
item, and a stream dropped open. The byte-level contract is also tested in `confined-transport`'s
unit tests and fuzzed by `confined_stream`.

## Why a program, and why under `tools/`

A stall and a kill are not things a byte buffer can show, and `Host::start` takes a program and no
arguments, so a test harness cannot be its own worker. A binary under `crates/` is a program the
install gate requires installed beside the windows; this one is never run but by its tests, so it
is a package under `tools/`, which the gate does not ask to be installed.

## Consequences

One more workspace member, built by a round that touches the transport. Planting an over-send the
host fails to count (`count_item` saturating instead of refusing) fails both the unit test and the
witness's `a_worker_that_over_sends_is_ended` (trap 13).
