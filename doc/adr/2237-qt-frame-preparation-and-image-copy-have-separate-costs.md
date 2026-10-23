# ADR 2237 — Qt frame preparation and image copy have separate costs

Status: **assessment** at the merge that took it, `b17ea075`: it records no decision a later round
must not re-litigate, as `doc/reviews/1704-batches-104-to-108-against-the-principles.md` finding 5
reads it (line added by ADR 2260).

A timing interval that includes placement queries is not solely an image-copy measurement.
Qt's former retrieval path queried all placements for the count, each descriptor and each
pixel slice. Its copy interval excluded the count query but included the later getters.
Those boundaries cannot support a claim about `QImage::copy` alone.

The opt-in toolkit probe reports snapshot retrieval on Rust's local monotonic clock, the sum
of C++ image wrapping/copy intervals, and the whole synchronous collection interval. It also
reports copied bytes and image-copy count. Collection includes bridge calls, descriptor work
and appending image values; subtracting copy time does not isolate a pure getter cost. No
absolute Rust timestamp is compared with a C++ timestamp.

The existing host-followup interval and widget-paint receipts remain separate. A widget update
only queues paint. None of these intervals proves compositor presentation or scanout, and
ordinary completed-frame readback remains asynchronous worker/poll work.

A focused mechanism comparison uses identical host state and verifies copied images and
rectangles before comparing balanced legacy-getter and snapshot deliveries. Query counts and
byte identity distinguish removed preparation from changed rendering or extra pixel copies.
This evidence can establish saved host work; it cannot by itself establish that repeated
queries caused the reader's Qt zoom lag. The ISO document's real-display gesture remains an
independent experiment.

The ISO page-20 mechanism control removes three or five placement queries per refresh
in favor of one, while copied pixels and rectangles agree. Its measured retrieval saving
is tens to hundreds of nanoseconds; the full copy/delivery interval has overlapping,
noisy paired results. This is evidence of removed work, not a reliable end-to-end speedup.
The image sink is justified by its single immutable observation and bounded ownership,
not by attributing the reader's visible lag to these small getter costs.
