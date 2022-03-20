use crate::types::{Instant, ENOUGH_TIME_BUTTON_PRESSED};
use stm32f1xx_hal::gpio::{ExtiPin, PinExt};

pub struct Button<T: ExtiPin + PinExt> {
    pub pin: T,
    pub last: Instant,
}

impl<T: ExtiPin + PinExt> Button<T> {
    /// update last time is pressed, return if it is passed enough time from last time
    pub fn pressed(&mut self, instant: Instant) -> bool {
        let enough_time_passed = (instant - self.last) > ENOUGH_TIME_BUTTON_PRESSED;
        defmt::debug!(
            "pin{=u8} pressed at {=u64} last {=u64} enough time passed:{=bool}",
            self.pin.pin_id(),
            instant.ticks(),
            self.last.ticks(),
            enough_time_passed
        );
        self.last = instant;
        self.pin.clear_interrupt_pending_bit();
        enough_time_passed
    }
}
