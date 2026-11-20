//! What the GTK and Qt windows' sound tests share: a document with one sound annotation, and a
//! recording sink standing in for the audio device (ADR 3329; `viewer-host`'s `played_sounds.rs`
//! is the same double for the player alone).
//!
//! **No speaker is in these tests, and no person has heard a sound through either window**: the
//! build machine has no sound card. What is proved is that the window's path reaches the device
//! with the samples §13.3's arithmetic gives, and that the window's keys and closes end the sound.

use std::sync::{Arc, Mutex};

use viewer_host::sound::{DeviceFormat, OpenSink, Sink};

/// The sound annotation's object.
pub(super) const ANNOTATION: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(4, 0);

/// The device's format: the sound's own rate and one channel, so that what reaches it is the
/// worker's decoding unconverted — `0x40` and `0xC0` as `+½` and `−½` of full scale (Table 305's
/// `Signed`, §13.3's "Twos-complement values").
pub(super) const FORMAT: DeviceFormat = DeviceFormat {
    rate: 8000,
    channels: 1,
};

/// What the samples reach the device as.
pub(super) const SAMPLES: [f32; 2] = [0.5, -0.5];

/// What the recording sink saw.
#[derive(Debug, Default)]
pub(super) struct Record {
    /// How many times the device was opened.
    pub(super) opened: usize,
    /// Every sound started, in order.
    pub(super) started: Vec<Vec<f32>>,
    /// How many stops it was asked for.
    pub(super) stops: usize,
}

/// The audio device's double, whose sounds never end by themselves.
struct Recording(Arc<Mutex<Record>>);

impl Sink for Recording {
    fn format(&self) -> DeviceFormat {
        FORMAT
    }

    fn start(&mut self, samples: Vec<f32>, _ended: Box<dyn FnOnce() + Send>) -> Result<(), String> {
        record(&self.0).started.push(samples);
        Ok(())
    }

    fn stop(&mut self) {
        let mut record = record(&self.0);
        record.stops = record.stops.saturating_add(1);
    }
}

/// An opener of the recording sink, and the record it writes.
pub(super) fn recording() -> (OpenSink, Arc<Mutex<Record>>) {
    let kept = Arc::new(Mutex::new(Record::default()));
    let shared = Arc::clone(&kept);
    let open: OpenSink = Box::new(move || {
        let mut record = record(&shared);
        record.opened = record.opened.saturating_add(1);
        Ok(Box::new(Recording(Arc::clone(&shared))) as Box<dyn Sink>)
    });
    (open, kept)
}

/// The record, read.
pub(super) fn record(record: &Arc<Mutex<Record>>) -> std::sync::MutexGuard<'_, Record> {
    match record.lock() {
        Ok(record) => record,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// A page with a sound annotation (object 4) whose sound object (object 5) holds two 8-bit
/// `Signed` samples at 8000 Hz, mono: `0x40` and `0xC0`.
#[must_use]
pub(super) fn a_sounding_document() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Sound /Rect [0 0 9 9] /Sound 5 0 R >>",
        "<< /Type /Sound /R 8000 /E /Signed /Filter /ASCIIHexDecode /Length 5 >>\nstream\n\
         40C0>\nendstream",
    ];
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(
            format!("{} 0 obj\n{body}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}
