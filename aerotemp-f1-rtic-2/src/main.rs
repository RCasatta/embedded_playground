#![deny(unsafe_code)]
//#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_rtt_target as _;
use rtic::app;
use stm32f1xx_hal::gpio::gpioc::PC13;
use stm32f1xx_hal::gpio::{Edge, ExtiPin, Output, PinState, PushPull, Pin, Input, CRL, PullUp};
use stm32f1xx_hal::prelude::*;
use systick_monotonic::{fugit::Duration, Systick};

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [SPI1])]
mod app {
    use super::*;

    const ONE_SEC: Duration<u64, 1, 1000> = Duration::<u64, 1, 1000>::from_ticks(1000);

    #[shared]
    struct Shared {
        // last: [Temp; 2] // temperature read last second
    // temps: [RingBuffer<Temp, 128>; 2]
    // unit
    // screen_type
    

    }

    #[local]
    struct Local {
        led: PC13<Output<PushPull>>,
        state: bool,
        counter: u32,
        pa0: Pin<Input<PullUp>, CRL, 'A', 0_u8>,
        pa1: Pin<Input<PullUp>, CRL, 'A', 1_u8>,

        pa2: Pin<Input<PullUp>, CRL, 'A', 2_u8>,
        pa3: Pin<Input<PullUp>, CRL, 'A', 3_u8>,

        pa4: Pin<Input<PullUp>, CRL, 'A', 4_u8>,
        // queu prod
        // queue cons
        // screen
    }

    #[monotonic(binds = SysTick, default = true)]
    type MonoTimer = Systick<1000>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::debug!("init");

        // Setup clocks
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();
        let mut afio = cx.device.AFIO.constrain();

        let mono = Systick::new(cx.core.SYST, 36_000_000);

        //defmt::info!("Starting! (eighty={=u32})", 80u32);

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

        // Setup Buttons
        let mut gpioa = cx.device.GPIOA.split();

        let mut pa0 = gpioa.pa0.into_pull_up_input(&mut gpioa.crl);
        pa0.make_interrupt_source(&mut afio);
        pa0.trigger_on_edge(&cx.device.EXTI, Edge::Rising);
        pa0.enable_interrupt(&cx.device.EXTI);

        let mut pa1 = gpioa.pa1.into_pull_up_input(&mut gpioa.crl);
        pa1.make_interrupt_source(&mut afio);
        pa1.trigger_on_edge(&cx.device.EXTI, Edge::Rising);
        pa1.enable_interrupt(&cx.device.EXTI);

        let mut pa2 = gpioa.pa2.into_pull_up_input(&mut gpioa.crl);
        pa2.make_interrupt_source(&mut afio);
        pa2.trigger_on_edge(&cx.device.EXTI, Edge::Rising);
        pa2.enable_interrupt(&cx.device.EXTI);

        let mut pa3 = gpioa.pa3.into_pull_up_input(&mut gpioa.crl);
        pa3.make_interrupt_source(&mut afio);
        pa3.trigger_on_edge(&cx.device.EXTI, Edge::Rising);
        pa3.enable_interrupt(&cx.device.EXTI);

        let mut pa4 = gpioa.pa4.into_pull_up_input(&mut gpioa.crl);
        pa4.make_interrupt_source(&mut afio);
        pa4.trigger_on_edge(&cx.device.EXTI, Edge::Rising);
        pa4.enable_interrupt(&cx.device.EXTI);

        // Schedule the every_seconding task
        every_second::spawn_after(ONE_SEC).unwrap();

        (
            Shared {},
            Local {
                led,
                state: false,
                counter: 0,
                pa0,
                pa1,
                pa2,
                pa3,
                pa4,
                
            },
            init::Monotonics(mono),
        )
    }

    //#[task(local = [led, state, counter])]
    //fn every_period(cx: every_second::Context) {
    //consume the queue, insert in array
    // call draw(true)
    //}

    #[task(local = [led, state, counter])]
    fn every_second(cx: every_second::Context) {
        defmt::debug!("every_second {=u32}", cx.local.counter);

        // shared last
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

        //call draw(true)
        every_second::spawn_after(ONE_SEC).unwrap();
    }

    #[task(binds = EXTI0, local = [pa0])]
    fn exti0(cx: exti0::Context) {
        defmt::debug!("exti0 {=bool}", cx.local.pa0.is_high());
        cx.local.pa0.clear_interrupt_pending_bit();
    }

    #[task(binds = EXTI1, local = [pa1])]
    fn exti1(cx: exti1::Context) {
        defmt::debug!("exti1 {=bool}", cx.local.pa1.is_high());
        cx.local.pa1.clear_interrupt_pending_bit();   
    }

    #[task(binds = EXTI2, local = [pa2])]
    fn exti2(cx: exti2::Context) {
        defmt::debug!("exti2 {=bool}", cx.local.pa2.is_high());
        cx.local.pa2.clear_interrupt_pending_bit();   
    }

    #[task(binds = EXTI3, local = [pa3])]
    fn exti3(cx: exti3::Context) {
        defmt::debug!("exti3 {=bool}", cx.local.pa3.is_high());
        cx.local.pa3.clear_interrupt_pending_bit();   
    }

    #[task(binds = EXTI4, local = [pa4])]
    fn exti4(cx: exti4::Context) {
        defmt::debug!("exti4 {=bool}", cx.local.pa4.is_high());
        cx.local.pa4.clear_interrupt_pending_bit();   
    }

    //fn exti, higher priority
    // detect button press
    // change screen_type and unit

    // fn draw
    // exclusive access to screen,
    // shared access to last, temps, screen_type, unit,
    // parameter/ reset (true when end period, false when end second)
}
