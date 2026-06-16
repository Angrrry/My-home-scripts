use caps_layout_switcher::gestures::{CapsEvent, CapsGesture, GestureDetector};
use std::time::Duration;

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn short_tap_is_pending_until_double_tap_window_expires() {
    let mut detector = GestureDetector::new(ms(250), ms(400));

    assert_eq!(detector.handle(CapsEvent::Down, ms(1_000)), None);
    assert_eq!(detector.handle(CapsEvent::Up, ms(1_080)), None);
    assert_eq!(detector.poll(ms(1_331)), Some(CapsGesture::SingleTap));
}

#[test]
fn second_short_tap_inside_window_is_double_tap() {
    let mut detector = GestureDetector::new(ms(250), ms(400));

    assert_eq!(detector.handle(CapsEvent::Down, ms(1_000)), None);
    assert_eq!(detector.handle(CapsEvent::Up, ms(1_060)), None);
    assert_eq!(detector.handle(CapsEvent::Down, ms(1_180)), None);
    assert_eq!(
        detector.handle(CapsEvent::Up, ms(1_240)),
        Some(CapsGesture::DoubleTap)
    );
    assert_eq!(detector.poll(ms(1_500)), None);
}

#[test]
fn hold_is_reported_on_release_and_clears_pending_taps() {
    let mut detector = GestureDetector::new(ms(250), ms(400));

    assert_eq!(detector.handle(CapsEvent::Down, ms(1_000)), None);
    assert_eq!(
        detector.handle(CapsEvent::Up, ms(1_450)),
        Some(CapsGesture::Hold)
    );
    assert_eq!(detector.poll(ms(1_800)), None);
}

#[test]
fn delayed_second_tap_emits_single_then_starts_new_tap() {
    let mut detector = GestureDetector::new(ms(250), ms(400));

    assert_eq!(detector.handle(CapsEvent::Down, ms(1_000)), None);
    assert_eq!(detector.handle(CapsEvent::Up, ms(1_050)), None);
    assert_eq!(
        detector.handle(CapsEvent::Down, ms(1_400)),
        Some(CapsGesture::SingleTap)
    );
    assert_eq!(detector.handle(CapsEvent::Up, ms(1_460)), None);
    assert_eq!(detector.poll(ms(1_711)), Some(CapsGesture::SingleTap));
}
