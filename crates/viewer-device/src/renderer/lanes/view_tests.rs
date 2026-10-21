//! View changes must not present an old frame's timing as a measurement of the next one.
use super::tests::placed;
use super::{Costs, EstimateBasis, Lane, View};
use raster_gpu::Coverage::{Compute, Cpu};
use std::time::Duration;

const COMMANDS: usize = 3_000_000;
const SIZE: Option<(u32, u32)> = Some((800, 1000));

fn record(costs: &mut Costs, pages: &[crate::stale::Placed], lane: Lane, millis: u64) {
    costs.record(
        Costs::key(pages),
        View::of(pages, SIZE),
        COMMANDS,
        lane,
        Duration::from_millis(millis),
        Compute,
    );
}

#[test]
fn fit_zoom_pan_fit_preserves_samples_without_extrapolating_them() {
    let fit = placed(0, 1.0, 0.0);
    let zoom = placed(0, 8.0, 0.0);
    let pan = placed(0, 8.0, 120.0);
    let key = Costs::key(&fit);
    let mut costs = Costs::default();
    record(&mut costs, &fit, Lane::Device { scene_kept: false }, 8000);
    record(&mut costs, &fit, Lane::Processor, 2000);
    let forecast = |costs: &Costs, pages: &[crate::stale::Placed]| {
        costs.forecast_at(&key, COMMANDS, &View::of(pages, SIZE), Compute)
    };
    assert!(
        forecast(&costs, &fit).1,
        "the measured fit prefers the processor"
    );
    for pages in [&zoom, &pan] {
        let (prediction, processor) = forecast(&costs, pages);
        assert_eq!(prediction.device_basis, EstimateBasis::OtherView);
        assert_eq!(prediction.processor_basis, EstimateBasis::OtherView);
        assert_eq!(prediction.device, Costs::cost(Costs::DEVICE_SEED, COMMANDS));
        assert!(
            !processor,
            "the graphics seed is inside WARN; the stale eight seconds cannot select CPU"
        );
    }
    record(&mut costs, &pan, Lane::Processor, 40);
    let (prediction, _) = forecast(&costs, &pan);
    assert_eq!(prediction.processor_basis, EstimateBasis::Measured);
    assert_eq!(prediction.device_basis, EstimateBasis::OtherView);
    let (prediction, _) = forecast(&costs, &fit);
    assert_eq!(prediction.device_basis, EstimateBasis::Measured);
    assert_eq!(prediction.device, Duration::from_secs(8));
    assert_eq!(prediction.processor_basis, EstimateBasis::OtherView);
    record(&mut costs, &zoom, Lane::Device { scene_kept: false }, 80);
    let (prediction, _) = forecast(&costs, &zoom);
    assert_eq!(prediction.device_basis, EstimateBasis::Measured);
    assert_eq!(prediction.device, Duration::from_millis(80));
}

#[test]
fn coverage_extent_page_document_and_ink_are_not_interchangeable_samples() {
    let fit = placed(0, 1.0, 0.0);
    let key = Costs::key(&fit);
    let view = View::of(&fit, SIZE);
    let mut costs = Costs::default();
    record(&mut costs, &fit, Lane::Device { scene_kept: false }, 8000);
    record(&mut costs, &fit, Lane::Processor, 2000);
    let (other_lane, _) = costs.forecast_at(&key, COMMANDS, &view, Cpu);
    assert_eq!(other_lane.device_basis, EstimateBasis::OtherCoverage);
    assert_eq!(other_lane.processor_basis, EstimateBasis::Measured);
    let resized = View::of(&fit, Some((1600, 2000)));
    assert_eq!(
        costs
            .forecast_at(&key, COMMANDS, &resized, Compute)
            .0
            .device_basis,
        EstimateBasis::OtherView
    );
    let mut target_changed = fit.clone();
    target_changed[0].target.width = 801;
    assert_eq!(
        costs
            .forecast_at(&key, COMMANDS, &View::of(&target_changed, SIZE), Compute)
            .0
            .device_basis,
        EstimateBasis::OtherView
    );
    for identity in [
        crate::stale::Picture::new(viewer_core::DocumentId(1), 1, 0),
        crate::stale::Picture::new(viewer_core::DocumentId(2), 0, 0),
        crate::stale::Picture::new(viewer_core::DocumentId(1), 0, 1),
    ] {
        assert_eq!(
            costs
                .forecast_at(&[identity], COMMANDS, &view, Compute)
                .0
                .device_basis,
            EstimateBasis::Seed
        );
    }
    assert_eq!(
        costs
            .forecast_at(&key, COMMANDS + 1, &view, Compute)
            .0
            .device_basis,
        EstimateBasis::OtherView
    );
}

#[test]
fn incompatible_view_does_not_force_another_gpu_trial() {
    let fit = placed(0, 1.0, 0.0);
    let zoom = placed(0, 8.0, 0.0);
    let key = Costs::key(&fit);
    let commands = 8_000_000;
    let mut costs = Costs::default();
    costs.record(
        key.clone(),
        View::of(&fit, SIZE),
        commands,
        Lane::Device { scene_kept: false },
        Duration::from_secs(8),
        Compute,
    );
    costs.drawn_by_the_processor(&zoom);
    let (prediction, processor) =
        costs.forecast_at(&key, commands, &View::of(&zoom, SIZE), Compute);
    assert_eq!(prediction.device_basis, EstimateBasis::OtherView);
    assert!(
        processor,
        "these seeds still prefer CPU; a new view must not reset the once-per-arrangement trial"
    );
}

#[test]
fn held_scene_and_background_pricing_use_compatible_samples() {
    let fit = placed(0, 1.0, 0.0);
    let zoom = placed(0, 8.0, 0.0);
    let key = Costs::key(&fit);
    let mut costs = Costs::default();
    record(&mut costs, &fit, Lane::Device { scene_kept: true }, 8000);
    record(&mut costs, &fit, Lane::Processor, 2000);
    assert_eq!(
        costs.build_ahead_at(&key, COMMANDS, &View::of(&fit, SIZE), Compute),
        None
    );
    assert_eq!(
        costs.build_ahead_at(&key, COMMANDS, &View::of(&zoom, SIZE), Compute),
        Some(Costs::cost(
            Costs::DEVICE_SEED - Costs::DEVICE_KEPT_SEED,
            COMMANDS
        ))
    );
    costs.scene_of(Some(&fit));
    let (prediction, _) = costs.forecast_at(&key, COMMANDS, &View::of(&zoom, SIZE), Compute);
    assert!(
        prediction.scene_kept,
        "zoom does not discard the held scene"
    );
    assert_eq!(prediction.device_basis, EstimateBasis::OtherView);
    assert_eq!(
        prediction.device,
        Costs::cost(Costs::DEVICE_KEPT_SEED, COMMANDS)
    );
    assert_eq!(
        costs.build_ahead_at(&key, COMMANDS, &View::of(&zoom, SIZE), Compute),
        None
    );
}

#[test]
fn displayed_provenance_names_why_a_fixed_seed_is_used() {
    let fit = placed(0, 1.0, 0.0);
    let zoom = placed(0, 8.0, 0.0);
    let key = Costs::key(&fit);
    let mut costs = Costs::default();
    record(&mut costs, &fit, Lane::Device { scene_kept: false }, 8000);
    let prediction = costs
        .forecast_at(&key, COMMANDS, &View::of(&zoom, SIZE), Compute)
        .0;
    assert!(
        prediction
            .said()
            .contains("seed; the last measurement was for another view")
    );
    let other = costs
        .forecast_at(&key, COMMANDS, &View::of(&fit, SIZE), Cpu)
        .0;
    assert!(
        other
            .said()
            .contains("seed; the last measurement used another coverage lane")
    );
}

#[test]
fn transition_frames_cannot_replace_or_borrow_a_page_measurement() {
    let page = placed(0, 1.0, 0.0);
    let mut transition = placed(0, 1.0, 0.0);
    transition[0].depicts = crate::stale::Depicts::Transition;
    let key = Costs::key(&page);
    let mut costs = Costs::default();
    record(&mut costs, &page, Lane::Device { scene_kept: false }, 80);
    for millis in [20, 200] {
        record(
            &mut costs,
            &transition,
            Lane::Device { scene_kept: false },
            millis,
        );
        let (moving, _) = costs.forecast_at(&key, COMMANDS, &View::of(&transition, SIZE), Compute);
        assert_eq!(moving.device_basis, EstimateBasis::OtherView);
        let (settled, _) = costs.forecast_at(&key, COMMANDS, &View::of(&page, SIZE), Compute);
        assert_eq!(settled.device_basis, EstimateBasis::Measured);
        assert_eq!(settled.device, Duration::from_millis(80));
    }
}
