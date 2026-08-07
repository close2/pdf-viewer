//! Fuzzes the four decoders of `viewer-confined`'s transport — the boundary's untrusted side.
//!
//! **Why this target exists.** `pdf-view-worker` interprets hostile documents behind seccomp-BPF
//! and Landlock, and then writes its answers down a pipe to a host that is *not* confined. So the
//! host's decoder is a parser over bytes a subverted worker chose, in exactly the sense a content
//! stream is a parser over bytes a producer chose — and this project's rule is that a parser is
//! fuzzed. The worker's own decoder is here for the mirror-image reason: `pdf-view-worker` is a
//! program, and what is piped into it is whatever a person piped into it.
//!
//! Until the three-hundred-and-eighty-sixth session the transport was guarded by a deterministic
//! truncation-and-byte-flip test standing in for this, which was proportionate to twenty-eight
//! encodings of scalars. That session added the eleven answers a panel is made of — four of them
//! *trees*, one of them a decoded raster, one of them a whole page's structure with parent links
//! that index the answer — and a stand-in stopped being proportionate.
//!
//! Four properties are under test.
//!
//! **Nothing panics**, over any byte sequence, in any of the four. A panic here is the confined
//! process reaching into the host: the one thing the boundary exists to prevent.
//!
//! **Nothing allocates from a claim.** The fuzz profile keeps overflow checks on, and a decoder
//! that sized a buffer from a length field rather than from the bytes that arrived would be found
//! by an out-of-memory rather than by an assertion — which is why `Reader::list` reserves a bounded
//! prefix and grows, and why running this target under a memory limit is worth doing.
//!
//! **Decoding is a function of the bytes.** The same message read twice must give the same answer:
//! these readers carry a cursor and, in the structure tree's case, a running index, and a decoder
//! that leaked state between elements is exactly the defect one pass cannot see.
//!
//! **The two checked invariants hold on anything that decodes at all**: a raster and a thumbnail
//! carry exactly `width × height × 4` bytes, and every parent link in §14.7's answer points at a
//! node already read. Both are refusals in the decoder rather than assumptions in the host, and a
//! target that did not assert them would not notice if one were deleted.

#![no_main]

use libfuzzer_sys::fuzz_target;
use viewer_confined::{Reply, wire};

/// The invariants a decoded answer must satisfy, whatever the bytes said.
fn check(reply: &Reply) {
    match reply {
        Reply::Frame { raster, .. } => {
            let expected = (raster.width as usize)
                .saturating_mul(raster.height as usize)
                .saturating_mul(4);
            assert_eq!(
                raster.data.len(),
                expected,
                "a raster crossed with dimensions its samples do not fill"
            );
        }
        Reply::Thumbnail(thumbnail) => {
            let expected = (thumbnail.image.width as usize)
                .saturating_mul(thumbnail.image.height as usize)
                .saturating_mul(4);
            assert_eq!(
                thumbnail.image.data.len(),
                expected,
                "a thumbnail crossed with dimensions its samples do not fill"
            );
        }
        Reply::Accessibility(nodes) => {
            for (at, node) in nodes.iter().enumerate() {
                if let Some(parent) = node.parent {
                    assert!(
                        parent < at,
                        "node {at} names {parent} as its parent, which is not behind it"
                    );
                }
            }
        }
        _ => {}
    }
}

fuzz_target!(|data: &[u8]| {
    // Every decoder over the same bytes: a message is a payload without its frame header, and
    // which of the four a payload is for is the header's business rather than the payload's.
    let answer = wire::answer(data);
    let events = wire::events(data);
    let command = wire::command(data);
    let query = wire::query(data);

    if let Ok(reply) = &answer {
        check(reply);
        let again = wire::answer(data).expect("bytes that decoded once decode again");
        check(&again);
        // Compared as they print rather than with `==`, and the fuzzer is why: this format
        // carries geometry as `f32` *bits*, deliberately, so a message can state a coordinate
        // that is `NaN` — and `NaN != NaN`, which made a decoder that is perfectly deterministic
        // fail an equality test on this target's first run, inside a minute, on a `PageGeometry`
        // whose page height was all ones. `Debug` is total where `PartialEq` is not, and what is
        // under test is that the bytes decide the answer.
        assert_eq!(
            format!("{reply:?}"),
            format!("{again:?}"),
            "the same answer decoded twice gave two different answers"
        );
    }

    if let Ok(events) = &events {
        let again = wire::events(data).expect("bytes that decoded once decode again");
        assert_eq!(
            format!("{events:?}"),
            format!("{again:?}"),
            "the same events decoded twice gave two different runs"
        );
    }

    if let Ok(command) = &command {
        let again = wire::command(data).expect("bytes that decoded once decode again");
        assert_eq!(
            format!("{command:?}"),
            format!("{again:?}"),
            "the same command decoded twice gave two different commands"
        );
    }

    if let Ok(query) = &query {
        let again = wire::query(data).expect("bytes that decoded once decode again");
        assert_eq!(
            format!("{query:?}"),
            format!("{again:?}"),
            "the same question decoded twice gave two different questions"
        );
    }
});
