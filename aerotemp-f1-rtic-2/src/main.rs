#![deny(unsafe_code)]
// #![deny(warnings)]
#![no_main]
#![no_std]

mod button;
mod screen;
mod types;
mod unit;

use defmt_rtt as _;
use panic_rtt_target as _;
use rtic::app;

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [TAMPER])]
mod app {

    use ssd1351::builder::Builder;
    use stm32f1xx_hal::gpio::gpioc::PC13;
    use stm32f1xx_hal::gpio::{Edge, ExtiPin, Output, PinState, PushPull};
    use stm32f1xx_hal::prelude::*;
    use stm32f1xx_hal::spi::Spi;
    use systick_monotonic::Systick;

    use crate::button::Button;
    use crate::screen::Screen;
    use crate::types::*;
    use crate::unit::Unit;
    use embedded_graphics::geometry::Point;
    use embedded_graphics::image::Image;
    use embedded_graphics::Drawable;
    use ssd1351::mode::GraphicsMode;
    use ssd1351::prelude::SSD1351_SPI_MODE;
    use ssd1351::properties::DisplayRotation;
    use tinytga::DynamicTga;

    #[shared]
    struct Shared {
        unit: Unit,
        screen: Screen,
    }

    #[local]
    struct Local {
        led: PC13<Output<PushPull>>,
        state: bool,
        seconds: usize,
        pa0: Button<PA0>,
        pa1: Button<PA1>,
        display: Display,
        latest_period: [[Temp; 2]; PERIOD],
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

        let clocks = rcc
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

        // Setup display
        defmt::debug!("Setup display");
        let mut nss = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
        nss.set_low();
        let pins = (
            gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl), // sck
            gpioa.pa6.into_floating_input(&mut gpioa.crl),      // miso
            gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl), // mosi
        );
        let dc = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
        let mut rst = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);

        let spi = Spi::spi1(
            cx.device.SPI1,
            pins,
            &mut afio.mapr,
            SSD1351_SPI_MODE,
            2_000_000.Hz(),
            clocks,
        );

        let mut display: GraphicsMode<_> = Builder::new().connect_spi(spi, dc).into();

        let mono = Systick::new(cx.core.SYST, 36_000_000);
        let mut delay = cx.device.TIM2.delay::<1000>(&clocks);
        display.reset(&mut rst, &mut delay).unwrap();
        defmt::debug!("display reset");
        display.init().unwrap();
        defmt::debug!("display init");
        display.set_rotation(DisplayRotation::Rotate180).unwrap();

        let image_data = include_bytes!("../assets/logo_groppo_aviazione_128x128.tga");
        let tga = DynamicTga::from_slice(image_data).unwrap();
        defmt::debug!("loading dynamic image");

        let image = Image::new(&tga, Point::zero());
        image.draw(&mut display).unwrap();
        defmt::debug!("draw image");

        // Schedule the every_second task
        every_second::spawn_after(ONE_SEC).unwrap();

        (
            Shared {
                unit: Unit::Celsius,
                screen: Screen::Both,
            },
            Local {
                led,
                state: false,
                seconds: 0,
                latest_period: [[0,0]; PERIOD],
                pa0: Button {
                    pin: pa0,
                    last: ZERO_INSTANT,
                },
                pa1: Button {
                    pin: pa1,
                    last: ZERO_INSTANT,
                },
                display,
            },
            init::Monotonics(mono),
        )
    }


    #[task(local = [display])]
    fn draw(_cx: draw::Context, _last: Temps, _average: Option<Temps> ) {
        defmt::debug!("draw");
    }

    #[task(local = [led, state, seconds, latest_period])]
    fn every_second(cx: every_second::Context) {
        every_second::spawn_after(ONE_SEC).unwrap();

        let current = *cx.local.seconds;
        defmt::debug!("every_second {=u32}", current);

        if *cx.local.state {
            cx.local.led.set_high();
            *cx.local.state = false;
        } else {
            cx.local.led.set_low();
            *cx.local.state = true;
        }

        //TODO read from sensors
        let temps = [current as i16, current as i16];

        cx.local.latest_period[current % PERIOD] = temps.clone();
        let average = if ((current + 1) % PERIOD) == 0 {
            let acc = cx.local.latest_period.iter().fold([0i32; 2], |acc, x| {
                [acc[0] + x[0] as i32, acc[1] + x[1] as i32]
            });
            Some([
                (acc[0] / PERIOD as i32) as i16,
                (acc[1] / PERIOD as i32) as i16,
            ])
        } else {
            None
        };

        draw::spawn(temps, average).unwrap();
    }

    #[task(binds = EXTI0, local = [pa0], shared=[screen])]
    fn exti0(mut cx: exti0::Context) {
        if cx.local.pa0.pressed(monotonics::now()) {
            cx.shared.screen.lock(|s| {
                s.next();
                defmt::debug!("screen {}", s);
            });
        }
    }

    #[task(binds = EXTI1, local = [pa1], shared=[unit])]
    fn exti1(mut cx: exti1::Context) {
        if cx.local.pa1.pressed(monotonics::now()) {
            cx.shared.unit.lock(|u| {
                u.next();
                defmt::debug!("unit {}", u);
            });
        }
    }

    // fn draw
    // exclusive access to screen,
    // shared access to last, temps, screen_type, unit,
    // parameter/ reset (true when end period, false when end second)
}
