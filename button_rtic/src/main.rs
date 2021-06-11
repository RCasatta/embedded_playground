//! Uses the timer interrupt to blink a led with different frequencies.
//!
//! This assumes that a LED is connected to pc13 as is the case on the blue pill board.
//!
//! Note: Without additional hardware, PC13 should not be used to drive an LED, see page 5.1.2 of
//! the reference manual for an explanation. This is not an issue on the blue pill.

#![no_std]
#![no_main]

// you can put a breakpoint on `rust_begin_unwind` to catch panics
use panic_halt as _;

use rtic::app;

use stm32f1xx_hal::gpio::{gpioa::PA7, gpioc::PC13, Edge};
use stm32f1xx_hal::gpio::{ExtiPin, Floating, Input, Output, PushPull, State};
use stm32f1xx_hal::prelude::*;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        led: PC13<Output<PushPull>>,
        pin: PA7<Input<Floating>>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let mut rcc = cx.device.RCC.constrain();
        let mut afio = cx.device.AFIO.constrain(&mut rcc.apb2);

        let mut gpioc = cx.device.GPIOC.split(&mut rcc.apb2);

        let led = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, State::High);

        let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);

        let mut pin = gpioa.pa7.into_floating_input(&mut gpioa.crl);
        pin.make_interrupt_source(&mut afio);
        pin.trigger_on_edge(&cx.device.EXTI, Edge::RISING_FALLING);
        pin.enable_interrupt(&cx.device.EXTI);

        init::LateResources { led, pin }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = EXTI9_5, priority = 1, resources = [led, pin])]
    fn pin(cx: pin::Context) {
        cx.resources.led.toggle().unwrap();
        cx.resources.pin.clear_interrupt_pending_bit();
    }
};
