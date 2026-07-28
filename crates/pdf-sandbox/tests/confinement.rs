//! What the sandbox is for, tested by doing it.
//!
//! Two kinds of test live here and they answer different questions. The confinement tests
//! ask whether a confined process can still reach the filesystem or the network — the
//! property the whole crate exists to provide, and one that no amount of reading the source
//! establishes, because it is the kernel that decides. The protocol tests ask whether a
//! worker survives being handed rubbish, which is what a corpus of real documents will do
//! to it several hundred times.
//!
//! # The image these tests decode is the specification's own
//!
//! ISO 32000-2 §7.4.7 works a complete JBIG2 example through: a 52×66 image whose symbol
//! dictionary is a global segment, whose page information and text region segments are the
//! image `XObject`'s stream, and whose file header and end-of-page segments are discarded
//! because PDF's embedded organisation forbids them. The bytes below are that example,
//! split exactly where the specification splits it. Nothing here was taken from another
//! implementation's output.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::trivially_copy_pass_by_ref,
    clippy::unwrap_used,
    reason = "test code walking a 52x66 fixture whose every index is bounded by the \
              dimensions asserted two lines above it; `unwrap` in a helper called only \
              from tests should fail as loudly as it does inside one"
)]

use pdf_sandbox::{Decoded, Isolation, Request, Sandbox, SandboxError};

/// The `/JBIG2Globals` stream: segment 0, a symbol dictionary, page association 0.
///
/// ISO 32000-2 §7.4.7, part (b) of the worked example.
const GLOBALS: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x03, 0xFF, 0xFD,
    0xFF, 0x02, 0xFE, 0xFE, 0xFE, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x2A, 0xE2, 0x25,
    0xAE, 0xA9, 0xA5, 0xA5, 0x38, 0xB4, 0xD9, 0x99, 0x9C, 0x5C, 0x8E, 0x56, 0xEF, 0x0F, 0x87, 0x27,
    0xF2, 0xB5, 0x3D, 0x4E, 0x37, 0xEF, 0x79, 0x5C, 0xC5, 0x50, 0x6D, 0xFF, 0xAC,
];

/// The image `XObject`'s stream: segment 1, page information, and segment 2, an immediate
/// text region.
///
/// ISO 32000-2 §7.4.7, part (c) of the worked example.
const IMAGE: &[u8] = &[
    0x00, 0x00, 0x00, 0x01, 0x30, 0x00, 0x01, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x34, 0x00,
    0x00, 0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x02, 0x06, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1E, 0x00, 0x00, 0x00, 0x34, 0x00, 0x00,
    0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x10, 0x00, 0x00, 0x00,
    0x02, 0x31, 0xDB, 0x51, 0xCE, 0x51, 0xFF, 0xAC,
];

/// The dimensions the image dictionary in the same example declares.
const WIDTH: u32 = 52;
const HEIGHT: u32 = 66;

/// Names the probe a re-executed test binary should run.
const PROBE_VARIABLE: &str = "PDF_SANDBOX_TEST_PROBE";

/// Exit code from a probe whose forbidden operation was refused.
const REFUSED: i32 = 17;
/// Exit code from a probe whose forbidden operation *succeeded*, which is the failure.
const ALLOWED: i32 = 18;

fn request() -> Request<'static> {
    Request::Jbig2 {
        data: IMAGE,
        globals: GLOBALS,
    }
}

#[test]
fn the_specifications_own_example_decodes() {
    let Decoded::Bilevel(image) = Sandbox::shared().decode(&request()).unwrap() else {
        panic!("JBIG2 must decode to bilevel samples");
    };

    assert_eq!((image.width, image.height), (WIDTH, HEIGHT));
    // Packed one bit per pixel with each row starting on a byte boundary: 52 pixels is
    // seven bytes, of which the last carries four used bits.
    assert_eq!(image.rows.len(), 7 * 66);

    // A decoder that returned a blank page would satisfy every check above. This is the one
    // that says something was actually drawn — and the sense of it is the filter's, not
    // JBIG2's: a set bit is white, so an all-ones buffer is the blank page.
    assert!(
        image.rows.iter().any(|byte| *byte != 0xFF),
        "the page came back blank"
    );
    assert!(
        image.rows.iter().any(|byte| *byte != 0x00),
        "the page came back solid black, which is the inversion being wrong"
    );

    // The text region segment's `SBNUMINSTANCES` field is 2 and the symbol dictionary holds
    // one symbol, so the page is the same glyph drawn twice — and it is, a letter C in the
    // upper half and again in the lower. Measuring both boxes is what turns "some ink
    // arrived" into a statement about *placement*: a symbol dictionary read at the wrong
    // offset, or a second instance decoded from exhausted arithmetic-coder state, gives two
    // marks of different sizes.
    let marks = ink_bounds(&image);
    assert_eq!(marks.len(), 2, "the page should carry exactly two marks");
    let [first, second] = marks.as_slice() else {
        unreachable!("just counted")
    };
    assert_eq!(
        (first.width, first.height),
        (second.width, second.height),
        "the two instances of one symbol must be the same size"
    );
    // Not a square: a bounding box that happened to be square could not tell the two axes
    // apart, so this assertion is what makes the previous one about both of them.
    assert_ne!(first.width, first.height);
}

/// Where a mark sits, in pixels.
#[derive(Debug)]
struct Mark {
    width: u32,
    height: u32,
}

/// The bounding box of each horizontal band of black pixels.
///
/// Bands rather than halves: splitting the page down the middle would need a row number
/// chosen by looking at the answer, and this derives the division from the image instead.
fn ink_bounds(image: &pdf_sandbox::Bilevel) -> Vec<Mark> {
    let row_bytes = image.width.div_ceil(8) as usize;
    // Black is a *clear* bit: this is the filter's sense, not JBIG2's.
    let black = |x: u32, y: u32| {
        let byte = image.rows[y as usize * row_bytes + (x / 8) as usize];
        (byte >> (7 - x % 8)) & 1 == 0
    };

    let mut marks: Vec<Mark> = Vec::new();
    let mut band: Option<(u32, u32, u32)> = None;
    for y in 0..=image.height {
        let extent = (y < image.height)
            .then(|| {
                let inked: Vec<u32> = (0..image.width).filter(|x| black(*x, y)).collect();
                inked.iter().min().copied().zip(inked.iter().max().copied())
            })
            .flatten();
        match (extent, band.as_mut()) {
            (Some((left, right)), Some(open)) => {
                open.0 = open.0.min(left);
                open.1 = open.1.max(right);
                open.2 += 1;
            }
            (Some((left, right)), None) => band = Some((left, right, 1)),
            (None, Some(open)) => {
                marks.push(Mark {
                    width: open.1 - open.0 + 1,
                    height: open.2,
                });
                band = None;
            }
            (None, None) => {}
        }
    }
    marks
}

#[test]
fn the_two_isolation_settings_produce_the_same_bytes() {
    // The flag must change where the work happens and nothing else. Both settings call the
    // same two functions, and this is what pins that: a divergence here would mean one of
    // the paths had grown a special case.
    let confined = Sandbox::shared().decode(&request()).unwrap();

    pdf_sandbox::set_isolation(Isolation::InProcess);
    let here = pdf_sandbox::decode(&request()).unwrap();
    pdf_sandbox::set_isolation(Isolation::Sandboxed);
    let confined_again = pdf_sandbox::decode(&request()).unwrap();

    assert_eq!(confined, here);
    assert_eq!(confined, confined_again);
}

#[test]
fn rubbish_is_refused_and_the_worker_survives_it() {
    let sandbox = Sandbox::shared();

    let error = sandbox
        .decode(&Request::Jbig2 {
            data: b"this is not a JBIG2 segment stream",
            globals: b"",
        })
        .unwrap_err();
    assert!(
        matches!(error, SandboxError::Undecodable { .. }),
        "a malformed image must be a refusal, not a transport failure: {error}"
    );

    // The point of the previous assertion: a corpus hands a worker hundreds of malformed
    // images, and if each one cost a process the arrangement would be unusable. The same
    // worker answers the next request.
    assert!(sandbox.decode(&request()).is_ok());
}

#[test]
fn an_empty_image_is_refused_rather_than_returned() {
    let error = Sandbox::shared()
        .decode(&Request::Jpx {
            data: b"",
            indices: false,
        })
        .unwrap_err();
    assert!(matches!(error, SandboxError::Undecodable { .. }), "{error}");
}

#[test]
fn the_worker_reports_full_landlock_enforcement() {
    // This asserts a property of the machine as much as of the code, and that is deliberate:
    // this kernel supports every Landlock right the ruleset asks for, so anything less than
    // full enforcement here means the ruleset stopped being accepted — a silent weakening
    // that nothing else would notice.
    let confinement = Sandbox::shared().confinement().unwrap();
    assert_eq!(
        confinement.landlock,
        pdf_sandbox::lockdown::LandlockLevel::Enforced,
        "Landlock is not fully enforced on a kernel that supports it"
    );
    assert_eq!(confinement.address_space_limit, 1 << 30);
}

#[test]
fn a_confined_process_cannot_open_a_file() {
    let status = run_probe("open");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined process read a file it should not have been able to name"
    );
    assert!(
        refused(&status),
        "expected the open to be refused or the process killed, got {status:?}"
    );
}

#[test]
fn a_confined_process_cannot_reach_the_network() {
    let status = run_probe("socket");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined process opened a socket"
    );
    assert!(
        refused(&status),
        "expected the socket to be refused or the process killed, got {status:?}"
    );
}

/// Whether a probe was stopped rather than served.
///
/// Two outcomes count. `SIGSYS` is the seccomp filter firing, which is what happens when the
/// system call is not on the allow-list at all — the usual case, because `openat` and
/// `socket` are absent from it. A clean `REFUSED` exit is Landlock having denied the path
/// before the call could fail for any other reason. Both are the sandbox working; only
/// `ALLOWED` is not.
fn refused(status: &std::process::ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal() == Some(libc::SIGSYS) || status.code() == Some(REFUSED)
}

/// Runs one probe in a fresh child and waits for it.
///
/// The child is this same test binary, re-executed with a filter that selects the one test
/// below. Confinement cannot be tested in the process doing the testing: it is irreversible
/// and process-wide, so the first thing it would break is the test harness.
fn run_probe(probe: &str) -> std::process::ExitStatus {
    std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "confined_probe", "--test-threads=1"])
        .env(PROBE_VARIABLE, probe)
        .output()
        .unwrap()
        .status
}

/// The child half of the two confinement tests, and not a test of anything by itself.
///
/// Run normally it does nothing, because the variable is unset. Run by [`run_probe`] it
/// confines itself and then attempts the thing the sandbox exists to prevent, reporting the
/// answer as an exit code — the only channel it has left.
#[test]
fn confined_probe() {
    let Ok(probe) = std::env::var(PROBE_VARIABLE) else {
        return;
    };

    pdf_sandbox::lockdown::apply().expect("a probe that cannot confine itself proves nothing");

    let permitted = match probe.as_str() {
        // `/proc/self/maps` is chosen over anything under `/etc` because it is guaranteed to
        // exist and to be readable by this user: a failure here has to be the sandbox, not
        // a missing file.
        "open" => std::fs::File::open("/proc/self/maps").is_ok(),
        "socket" => std::net::UdpSocket::bind("127.0.0.1:0").is_ok(),
        other => panic!("no probe named {other}"),
    };

    std::process::exit(if permitted { ALLOWED } else { REFUSED });
}
