#![deny(unsafe_code)]
// #![deny(warnings)]
#![no_main]
#![no_std]

mod button;
mod hist;
mod screen;
mod temp;
mod types;
mod unit;

use defmt_rtt as _;
use panic_rtt_target as _;
use rtic::app;

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [TAMPER])]
mod app {

    use max31865::FilterMode::Filter50Hz;
    use max31865::SensorType::TwoOrFourWire;
    use max31865::{temp_conversion, Max31865};
    use ssd1351::builder::Builder;
    use stm32f1xx_hal::gpio::{Edge, ExtiPin};
    use stm32f1xx_hal::prelude::*;
    use stm32f1xx_hal::spi::Spi;
    use systick_monotonic::Systick;

    use crate::button::Button;
    use crate::hist::Hist;
    use crate::screen::{
        draw_titles, text_small_white, text_temperature, Model, ModelChange, ScreenType,
    };
    use crate::types::*;
    use crate::unit::Unit;
    use embedded_graphics::geometry::{Point, Size};
    use embedded_graphics::image::Image;
    use embedded_graphics::prelude::RgbColor;
    use embedded_graphics::Drawable;

    use heapless::String;
    use ssd1351::mode::GraphicsMode;
    use ssd1351::prelude::SSD1351_SPI_MODE;
    use ssd1351::properties::DisplayRotation;
    use tinytga::DynamicTga;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        seconds: usize,
        pa0: Button<PA0>,
        pa1: Button<PA1>,
        display: Display,
        latest_period: [Temps; PERIOD],

        unit: Unit,
        screen: ScreenType,

        model: Model,
        temps: SharedBusResources<BusType>,
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

        // Setup sensors

        let mut gpiob = cx.device.GPIOB.split();

        let pins = (
            gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh), // sck
            gpiob.pb14.into_floating_input(&mut gpiob.crh),      // miso
            gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh), // mosi
        );

        let spi = Spi::spi2(cx.device.SPI2, pins, max31865::MODE, 2.MHz(), clocks);

        let manager = shared_bus_rtic::new!(spi, BusType);

        let rdy_1 = gpiob.pb1;
        let nss_1 = gpiob.pb0.into_push_pull_output(&mut gpiob.crl);

        let rdy_2 = gpiob.pb10;
        let nss_2 = gpiob.pb11.into_push_pull_output(&mut gpiob.crh);

        let mut t1 = Max31865::new(manager.acquire(), nss_1, rdy_1).unwrap();
        t1.configure(true, true, false, TwoOrFourWire, Filter50Hz)
            .unwrap();
        t1.set_calibration(430_000);

        let mut t2 = Max31865::new(manager.acquire(), nss_2, rdy_2).unwrap();
        t2.configure(true, true, false, TwoOrFourWire, Filter50Hz)
            .unwrap();
        t2.set_calibration(430_000);
        let temps = SharedBusResources { t1, t2 };

        // Schedule the every_second task
        every_second::spawn_after(ONE_SEC).unwrap();

        (
            Shared {},
            Local {
                seconds: 0,
                latest_period: [[0.into(), 0.into()]; PERIOD],
                pa0: Button {
                    pin: pa0,
                    last: ZERO_INSTANT,
                },
                pa1: Button {
                    pin: pa1,
                    last: ZERO_INSTANT,
                },
                display,

                unit: Unit::Celsius,
                screen: ScreenType::Both,
                model: Model::default(),
                temps,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [seconds, latest_period, temps])]
    fn every_second(cx: every_second::Context) {
        every_second::spawn_after(ONE_SEC).unwrap();

        let current = *cx.local.seconds;
        defmt::debug!("every_second {=usize}", current);
        if current == 0 {
            draw::spawn(ModelChange::Clear).unwrap();
        }

        //DEBUG mock read for sensors
        //let temps = [(current as i16).into(), (-(current as i16)).into()];

        let ohms1 = cx.local.temps.t1.read_ohms().unwrap();
        let t1 = temp_conversion::LOOKUP_VEC_PT1000.lookup_temperature(ohms1 as i32) as i16;

        let ohms2 = cx.local.temps.t2.read_ohms().unwrap();
        let t2 = temp_conversion::LOOKUP_VEC_PT1000.lookup_temperature(ohms2 as i32) as i16;

        let temps = [t1.into(), t2.into()];

        cx.local.latest_period[current % PERIOD] = temps.clone();
        let change = if ((current + 1) % PERIOD) == 0 {
            let acc = cx.local.latest_period.iter().fold([0i32; 2], |acc, x| {
                [acc[0] + x[0].0 as i32, acc[1] + x[1].0 as i32]
            });
            let average = [
                ((acc[0] / PERIOD as i32) as i16).into(),
                ((acc[1] / PERIOD as i32) as i16).into(),
            ];
            ModelChange::LastAndAverage(temps, average)
        } else {
            ModelChange::Last(temps)
        };

        draw::spawn(change).unwrap();
        *cx.local.seconds += 1;
    }

    #[task(capacity = 3, local = [display, model, buffer: String<32> = String::new()])]
    fn draw(cx: draw::Context, changes: ModelChange) {
        defmt::debug!("draw {}", changes);

        let model = cx.local.model;
        let mut display = cx.local.display;
        let mut buffer = cx.local.buffer;

        model.apply(changes);

        if model.changed {
            if model.clear {
                display.clear();
                draw_titles(&mut display, model.screen_type);
            }
            // draw temp
            let last = model.last;
            match model.screen_type {
                ScreenType::Both => {
                    for i in 0..2 {
                        text_temperature(
                            display,
                            &mut buffer,
                            32,
                            6 + i as i32 * 64,
                            false,
                            last[i],
                            model.unit,
                        );
                        let hist = Hist::new(
                            Point::new(0, 30 + i as i32 * 64),
                            Size::new(SCREEN_WIDTH as u32, 30),
                        );
                        hist.draw(
                            &model.history[i as usize],
                            display,
                            RgbColor::GREEN,
                            RgbColor::BLACK,
                        )
                        .unwrap();
                    }
                }
                ScreenType::Single(i) => {
                    let i = i as usize;
                    text_temperature(display, &mut buffer, 0, 20, true, last[i], model.unit);
                    let hist = Hist::new(Point::new(0, 53), Size::new(SCREEN_WIDTH as u32, 45));
                    hist.draw(&model.history[i], display, RgbColor::GREEN, RgbColor::BLACK)
                        .unwrap();
                    for b in 0..2 {
                        buffer.push_str(MIN_OR_MAX[b]).unwrap();
                        model
                            .min_or_max(b != 0, i)
                            .write_buffer(model.unit, false, &mut buffer);
                        text_small_white(display, buffer, b as i32 * 68, 110);
                    }
                }
            }
        }
    }

    #[task(binds = EXTI0, priority = 2, local = [pa0, screen])]
    fn exti0(cx: exti0::Context) {
        if cx.local.pa0.pressed(monotonics::now()) {
            let new = cx.local.screen.next();
            defmt::debug!("screen {}", new);
            draw::spawn(ModelChange::ScreenType(new)).unwrap();
        }
    }

    #[task(binds = EXTI1, priority = 2, local = [pa1, unit])]
    fn exti1(cx: exti1::Context) {
        if cx.local.pa1.pressed(monotonics::now()) {
            let new = cx.local.unit.next();
            defmt::debug!("unit {}", new);
            draw::spawn(ModelChange::Unit(new)).unwrap();
        }
    }
}
