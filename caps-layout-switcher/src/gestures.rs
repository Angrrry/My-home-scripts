use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapsEvent {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapsGesture {
    SingleTap,
    DoubleTap,
    Hold,
}

#[derive(Debug)]
pub struct GestureDetector {
    double_tap_timeout: Duration,
    hold_timeout: Duration,
    press_started_at: Option<Duration>,
    pending_tap_released_at: Option<Duration>,
}

impl GestureDetector {
    pub fn new(double_tap_timeout: Duration, hold_timeout: Duration) -> Self {
        Self {
            double_tap_timeout,
            hold_timeout,
            press_started_at: None,
            pending_tap_released_at: None,
        }
    }

    pub fn handle(&mut self, event: CapsEvent, at: Duration) -> Option<CapsGesture> {
        match event {
            CapsEvent::Down => {
                self.press_started_at = Some(at);
                self.flush_expired_tap(at)
            }
            CapsEvent::Up => {
                let Some(press_started_at) = self.press_started_at.take() else {
                    return self.flush_expired_tap(at);
                };
                
                if at.saturating_sub(press_started_at) >= self.hold_timeout {
                    self.pending_tap_released_at = None;
                    return Some(CapsGesture::Hold);
                }

                if let Some(previous_tap_at) = self.pending_tap_released_at {
                    if at.saturating_sub(previous_tap_at) <= self.double_tap_timeout {
                        self.pending_tap_released_at = None;
                        return Some(CapsGesture::DoubleTap);
                    }
                }

                self.pending_tap_released_at = Some(at);
                None
            }
        }
    }

    pub fn poll(&mut self, at: Duration) -> Option<CapsGesture> {
        self.flush_expired_tap(at)
    }

    fn flush_expired_tap(&mut self, at: Duration) -> Option<CapsGesture> {
        let pending_tap_released_at = self.pending_tap_released_at?;
        if at.saturating_sub(pending_tap_released_at) <= self.double_tap_timeout {
            return None;
        }

        self.pending_tap_released_at = None;
        Some(CapsGesture::SingleTap)
    }
}
