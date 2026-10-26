# 2403 — The render node's holders are listed for this account, and no further

Status: implemented. Session 1784, 2026-10-26, an "instruments" round under the project owner's switch
to code improvements of 2026-10-14. Context: trap 262 (a GPU figure carries the render node's holders
beside it), trap 274 and ADR 2402 (the device's clock level), ADR 1537 (the device's busy share). Code:
`crates/render-raster/tests/turn_path.rs` (`render_node_holders`). It cites no clause and moves no
ledger row.

## 1. The question

Trap 262 asks that a GPU figure carry the list of processes holding `/dev/dri/renderD*` beside it.
`fuser` and `lsof`, run as this account, list only this account's processes, and the neighbour that
moves this machine's device clock is usually another account's: the owner's desktop session (ADR 2388).
The slot asked for a method this account may use, or the sentence that none exists.

## 2. What was tried, on this machine, as this account

- **The DRM `clients` file** (`/sys/kernel/debug/dri/<minor>/clients`) names every open of the device
  with its pid and account. debugfs is mounted at `/sys/kernel/debug` with mode `0700`, owner root:
  closed.
- **A process's descriptors and their `fdinfo`** (`/proc/<pid>/fd`, `/proc/<pid>/fdinfo`, where amdgpu
  also prints each client's engine time). The kernel asks for the access `ptrace` reads with, which an
  account has over its own processes only: of the 673 processes on the machine at the time, 32 were
  readable and 641 were not, among them every process of the owner's session.
- **The node's permissions**: `renderD128` is `crw-rw-rw-`, so any account opens it and no ACL records
  who did; `card1`'s ACL names the seat's user, which is who *may* open it, not who has.
- **The device-wide counters** this account can read: `gpu_busy_percent`, `pp_dpm_sclk`,
  `gpu_metrics`, `mem_info_vram_used` and `mem_info_gtt_used`. Each says the device is in use, none says
  by whom.

No method open to this account lists another account's holders. One needs root: a debugfs read, or
`CAP_SYS_PTRACE` for `fdinfo`.

## 3. The decision

- **This account's holders are listed beside every child** of the turn gate, by pid and name, read from
  `/proc/<pid>/fd` the way `fuser` reads it, with the count of processes it could not read. A sibling
  round's walk or window drive is this account's, and that is the neighbour trap 262 was written about.
- **Another account's holders are stood for by what the device says of itself**: the busy share before
  and after each child (ADR 1537) and the clock level through its frames (ADR 2402). A neighbour that
  occupies the device moves one or the other, and either declines the child.
- **No privileged listing is asked for.** Granting this account debugfs or `CAP_SYS_PTRACE` would open
  every process's memory map to it, which is a larger grant than a figure's footnote needs, and the two
  device-wide checks already decline the children such a list would explain.

## Costs

The listing reads every readable process's descriptor table once per child, a few milliseconds, before
the child starts and outside anything timed.
