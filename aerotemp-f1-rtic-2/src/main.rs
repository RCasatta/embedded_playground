#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_rtt_target as _;
use rtic::app;
use stm32f1xx_hal::gpio::PinState;
use stm32f1xx_hal::gpio::{gpioc::PC13, Output, PushPull};
use stm32f1xx_hal::prelude::*;
use systick_monotonic::{fugit::Duration, Systick};

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [SPI1])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        //last: [Temp; 2] // temperature read last second
    // temps: [RingBuffer<Temp, 128>; 2]
    }

    #[local]
    struct Local {
        led: PC13<Output<PushPull>>,
        state: bool,
        counter: u32,
        // queu prod
        // queue cons
        // screen
    }

    #[monotonic(binds = SysTick, default = true)]
    type MonoTimer = Systick<1000>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Setup clocks
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();

        let mono = Systick::new(cx.core.SYST, 36_000_000);

        defmt::info!("Starting! (eighty={=u32})", 80u32);

        let _clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(36.MHz())
            .pclk1(36.MHz())
            .freeze(&mut flash.acr);

        // Setup LED
        let mut gpioc = cx.device.GPIOC.split();
        let led = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, PinState::Low);

        // Schedule the every_seconding task
        every_second::spawn_after(Duration::<u64, 1, 1000>::from_ticks(1)).unwrap();

        (
            Shared {},
            Local {
                led,
                state: false,
                counter: 0,
            },
            init::Monotonics(mono),
        )
    }

    //#[task(local = [led, state, counter])]
    //fn every_period(cx: every_second::Context) {
    //consume the queue, insert in array
    //}

    #[task(local = [led, state, counter])]
    fn every_second(cx: every_second::Context) {
        // run every second
        // save temps in queue https://rtic.rs/1/book/en/by-example/tips_static_lifetimes.html
        *cx.local.counter += 1;
        if *cx.local.state {
            cx.local.led.set_high();
            *cx.local.state = false;
        } else {
            cx.local.led.set_low();
            *cx.local.state = true;
        }
        defmt::info!("Long message with a counter {=u32})", cx.local.counter);

        every_second::spawn_after(Duration::<u64, 1, 1000>::from_ticks(1)).unwrap();
    }

    //fn exti
    // detect button press
    // change screen type and degrees/farenhait

    // fn draw
    // exclusive access to screen
}
