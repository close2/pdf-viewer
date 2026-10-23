# ADR 2196 — A frame price names the view that measured it

Status: **accepted** at the merge that took it, `b9d05ee8` (line added by ADR 2260).

The owner's GPU-estimate report (todo75 §8) has a concrete cause the model permits: lane prices
were keyed by document/page/ink and multiplied by the whole arrangement's command count. When
CPU drew successive views it updated only its price, while an older GPU price remained a measured
prediction for every zoom and pan. Coverage changes also reused that price. The displayed GPU
number can instead be a seed; a constant number alone does not diagnose stale feedback.

A completed sample now carries the full target dimensions and affine transform for every page,
the window output extent where separate, its command count, and the GPU coverage lane. The
arrangement identity and the device's built/kept buckets remain distinct. Only a compatible sample
prices the current view. Otherwise the existing bucket's seed applies, explicitly labelled as
unmeasured, another view's sample, or another coverage's sample. Both window and page-readback
paths use this rule; background scene-build pricing uses it too.

The arrangement's once-only device trial, refusal memory and held-scene record are independent
of sample applicability. Zooming does not reset them, allocate a device, or schedule a probe.
The current policy still keeps a device prediction inside WARN on the device, and beyond it
chooses the smaller prediction. Seed fallback removes a stale-sample reason for choosing CPU;
it does not promise GPU exploration when the seeds themselves choose CPU.

Samples remain bounded by the existing 32-arrangement history, with at most one latest sample
for each CPU/GPU-built/GPU-kept bucket. Returning to a sampled view can use that sample while it
remains the latest in its bucket. The estimate is a graphics-path frame cost, including host
work, not an adapter timestamp or a guarantee of the next frame's duration.

Six stateful CPU regressions cover fit/zoom/pan/fit, size/page/document/ink/coverage changes,
actual sample replacement, background pricing, held-scene preservation and no forced trial.
The shared formatter reports why a seed remains constant. Real-device observation and timing
acceptance are recorded in round1681's history after their gates, not inferred from these tests.

Transition frames carry the arriving page identity but different commands. They neither borrow
nor replace page samples, even when counts match; completed draws still update arrangement trial
bookkeeping. A focused regression preserves the page price across two different transition timings.
