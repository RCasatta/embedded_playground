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

use embedded_hal::digital::v2::{InputPin, OutputPin};
use stm32f1xx_hal::gpio::gpioa::{PA0, PA1, PA2, PA5, PA6, PA7};
use stm32f1xx_hal::gpio::{gpioc::PC13, Edge};
use stm32f1xx_hal::gpio::{ExtiPin, Floating, Input, Output, PushPull, State};
use stm32f1xx_hal::prelude::*;

pub struct Button<T> {
    pin: T,
    pressed: bool,
}

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        led: PC13<Output<PushPull>>,

        led0: PA0<Output<PushPull>>,
        led1: PA1<Output<PushPull>>,
        led2: PA2<Output<PushPull>>,

        pa5: Button<PA5<Input<Floating>>>,
        pa6: Button<PA6<Input<Floating>>>,
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

        // Setup Leds
        let led0 = gpioa
            .pa0
            .into_push_pull_output_with_state(&mut gpioa.crl, State::Low);
        let led1 = gpioa
            .pa1
            .into_push_pull_output_with_state(&mut gpioa.crl, State::Low);
        let led2 = gpioa
            .pa2
            .into_push_pull_output_with_state(&mut gpioa.crl, State::Low);

        // Setup Buttons
        let mut pa5 = gpioa.pa5.into_floating_input(&mut gpioa.crl);
        pa5.make_interrupt_source(&mut afio);
        pa5.trigger_on_edge(&cx.device.EXTI, Edge::RISING_FALLING);
        pa5.enable_interrupt(&cx.device.EXTI);
        let pa5 = Button {
            pin: pa5,
            pressed: false,
        };

        let mut pa6 = gpioa.pa6.into_floating_input(&mut gpioa.crl);
        pa6.make_interrupt_source(&mut afio);
        pa6.trigger_on_edge(&cx.device.EXTI, Edge::RISING_FALLING);
        pa6.enable_interrupt(&cx.device.EXTI);
        let pa6 = Button {
            pin: pa6,
            pressed: false,
        };

        // Setup Leds

        init::LateResources {
            led,
            led0,
            led1,
            led2,
            pa5,
            pa6,
        }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = EXTI9_5, priority = 1, resources = [led, led0, led1, led2, pa5, pa6])]
    fn pin(cx: pin::Context) {
        if !cx.resources.pa5.pressed && cx.resources.pa5.pin.is_low().unwrap() {
            // PA5 RISING
            cx.resources.pa5.pressed = true;
        } else if cx.resources.pa5.pressed && cx.resources.pa5.pin.is_high().unwrap() {
            // PA5 FALLING
            cx.resources.pa5.pressed = false;
            if cx.resources.pa6.pressed {
                // BOTH pressed
                cx.resources.pa6.pressed = false;
                cx.resources.led0.toggle().unwrap();
            } else {
                cx.resources.led1.toggle().unwrap();
            }
        } else if !cx.resources.pa6.pressed && cx.resources.pa6.pin.is_low().unwrap() {
            // PA6 RISING
            cx.resources.pa6.pressed = true;
        } else if cx.resources.pa6.pressed && cx.resources.pa6.pin.is_high().unwrap() {
            // PA6 FALLING
            cx.resources.pa6.pressed = false;
            if cx.resources.pa5.pressed {
                // BOTH pressed
                cx.resources.pa5.pressed = false;
                cx.resources.led0.toggle().unwrap();
            } else {
                cx.resources.led2.toggle().unwrap();
            }
        }

        cx.resources.pa5.pin.clear_interrupt_pending_bit();
        cx.resources.pa6.pin.clear_interrupt_pending_bit();
    }
};
