# 2661 — The turn gate judges a machine with one class of core unpinned, as the launch gate does

Session 1913, a single round (measure). Status: **accepted**. Context: ADR 2660, the launch gate's
`the_performance_cores`. It cites no clause and moves no ledger row.

## 1. What the gate did

`crates/render-raster/tests/turn_path.rs` declined every figure with "no faster class of core to pin
to" wherever `performance_cores` found no `cpuinfo_max_freq` that differs between CPUs. On the server
VM there is no `cpufreq` directory at all, so the gate printed 99 figures, judged none, and exited
green: a gate that checks nothing.

## 2. The decision

**The decline is dropped; an unpinned child is judged.** Pinning exists to remove a lottery between
two classes of core — on the notebook, four Zen 5 cores at 5.16 GHz and eight Zen 5c at 3.29 read the
same probe a factor of two apart. A machine with one class of core has no lottery to remove, which is
the launch gate's reading already ("a machine with one class of core is not pinned at all") and the
toolkit gate's. The child is still declined for a calibration probe out of band, a load over the
ceiling and a busy device, which are the checks that say whether the machine was the machine.

## 3. What it costs

A machine whose cores differ but do not say so in `cpufreq` is now judged unpinned rather than
declined; its calibration probe is then what declines a child that landed on a slow core.
