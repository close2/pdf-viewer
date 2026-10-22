//! Real GTK allocation controls, isolated on an owned software display in a child process.

use super::{Picture, place};
use gtk4::prelude::*;

#[test]
fn texture_pixels_keep_their_device_extent_through_reused_slots()
-> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("QUORRA_GTK_ALLOCATION_CHILD").is_none() {
        return on_software_display(
            "host::stand_in::tests::texture_pixels_keep_their_device_extent_through_reused_slots",
        );
    }
    gtk4::init()?;
    let fixed = gtk4::Fixed::new();
    let slot = gtk4::Picture::new();
    slot.set_content_fit(gtk4::ContentFit::Fill);
    slot.set_can_shrink(false);
    fixed.put(&slot, 0.0, 0.0);
    // Reuse the actual widget through larger/smaller and scaled/fresh textures. A fresh
    // viewport window at a negative origin must obey the same device-pixel contract.
    for scale in [1.0, 2.0, 1.5] {
        for (width, height, extent, origin) in [
            (200, 160, (200, 160), (13.25, 17.75)),
            (200, 160, (100, 80), (13.25, 17.75)),
            (101, 81, (101, 81), (-21.25, -23.75)),
            (301, 241, (151, 121), (-101.25, 37.75)),
            (101, 81, (101, 81), (21.25, 23.75)),
        ] {
            check(&fixed, &slot, width, height, extent, origin, scale)?;
        }
    }
    receipt();
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the parent requires this positive child completion receipt"
)]
fn receipt() {
    println!("GTK_ALLOCATION_15_PLACEMENTS_PASSED");
}

fn check(
    fixed: &gtk4::Fixed,
    slot: &gtk4::Picture,
    width: i32,
    height: i32,
    extent: (u32, u32),
    origin: (f32, f32),
    scale: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let stride = usize::try_from(width)?
        .checked_mul(4)
        .ok_or("test stride overflow")?;
    let size = stride
        .checked_mul(usize::try_from(height)?)
        .ok_or("test texture overflow")?;
    let bytes = gtk4::glib::Bytes::from_owned(vec![255u8; size]);
    let texture = gtk4::gdk::MemoryTexture::new(
        width,
        height,
        gtk4::gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        stride,
    );
    place(
        fixed,
        slot,
        &Picture {
            texture,
            origin,
            extent,
            bytes: bytes.len(),
        },
        scale,
    );
    fixed.allocate(800, 600, -1, None);
    assert_eq!((slot.width(), slot.height()), (width, height));
    let bounds = slot
        .compute_bounds(fixed)
        .ok_or("picture has no bounds in its fixed")?;
    let fresh = (i64::from(extent.0), i64::from(extent.1)) == (i64::from(width), i64::from(height));
    let expected_origin = if fresh {
        (origin.0.round(), origin.1.round())
    } else {
        origin
    };
    for (actual, expected) in [
        (f64::from(bounds.x()) * scale, f64::from(expected_origin.0)),
        (f64::from(bounds.y()) * scale, f64::from(expected_origin.1)),
        (f64::from(bounds.width()) * scale, f64::from(extent.0)),
        (f64::from(bounds.height()) * scale, f64::from(extent.1)),
    ] {
        assert!(
            (actual - expected).abs() < 0.001,
            "scale={scale}: actual={actual}, expected={expected}"
        );
    }
    Ok(())
}

/// Only the child initializes GTK, so normal parallel unit tests never share its thread or display.
fn on_software_display(filter: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let (_display, name) = display()?;
    let child = Command::new(std::env::current_exe()?)
        .args(["--exact", filter, "--nocapture"])
        .env("QUORRA_GTK_ALLOCATION_CHILD", "1")
        .env("DISPLAY", name)
        .env_remove("WAYLAND_DISPLAY")
        .env("GDK_BACKEND", "x11")
        .env("GSK_RENDERER", "cairo")
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut child = OwnedChild(child);
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(15) {
        if let Some(status) = child.0.try_wait()? {
            use std::io::Read as _;
            let mut output = String::new();
            child
                .0
                .stdout
                .take()
                .ok_or("child stdout was not captured")?
                .read_to_string(&mut output)?;
            return if status.success()
                && output
                    .lines()
                    .any(|line| line.ends_with("GTK_ALLOCATION_15_PLACEMENTS_PASSED"))
            {
                Ok(())
            } else {
                Err(format!("GTK allocation child: {status}; missing success receipt or failed assertions: {output}").into())
            };
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("GTK allocation child exceeded 15 seconds".into())
}

/// A lock file must name our actual `Xvfb` child before a test may connect to its display.
fn display() -> Result<(OwnedChild, String), Box<dyn std::error::Error>> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let began = Instant::now();
    for number in 4160..4260_u32 {
        if began.elapsed() > Duration::from_secs(5) {
            break;
        }
        let lock = std::path::PathBuf::from(format!("/tmp/.X{number}-lock"));
        if lock.exists() {
            continue;
        }
        let name = format!(":{number}");
        let mut server = OwnedChild(
            Command::new("Xvfb")
                .args([
                    name.as_str(),
                    "-screen",
                    "0",
                    "1000x800x24",
                    "-nolisten",
                    "tcp",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        );
        while began.elapsed() < Duration::from_secs(5) {
            if let Ok(held) = std::fs::read_to_string(&lock) {
                if held.trim().parse::<u32>().ok() == Some(server.0.id()) {
                    // A lock is created before the server accepts clients. Require a completed
                    // display handshake too; the probe is bounded independently of the child.
                    if Command::new("timeout")
                        .args(["1s", "xdpyinfo", "-display", &name])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()?
                        .success()
                    {
                        return Ok((server, name));
                    }
                }
                if !held.trim().is_empty() && held.trim().parse::<u32>().ok() != Some(server.0.id())
                {
                    break;
                }
            }
            if server.0.try_wait()?.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    Err("no owned Xvfb display became ready within five seconds".into())
}

/// Every error and assertion path reaps only the process this test started.
struct OwnedChild(std::process::Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Calibration: libtest exits successfully for a stale filter, but that proves no geometry.
#[test]
fn a_zero_test_child_cannot_pass_the_geometry_gate() {
    match on_software_display("host::stand_in::tests::deliberately_missing_filter") {
        Ok(()) => panic!("a zero-test child was accepted"),
        Err(error) => assert!(
            error.to_string().contains("missing success receipt"),
            "{error}"
        ),
    }
}
