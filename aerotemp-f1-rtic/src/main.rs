#![no_std]
#![no_main]

mod temps;
mod types;
use panic_halt as _;

use rtic::app;

use crate::temps::TempsValues;
use crate::types::{BusType, Button, Display, Scale, SharedBusResources, Unit};
use core::fmt::Write;
use e_ring::hist::Hist;
use e_write_buffer::WriteBuffer;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::image::Image;
use embedded_graphics::mono_font::ascii::{FONT_6X9, FONT_8X13};
use embedded_graphics::mono_font::{iso_8859_1, MonoTextStyle};
use embedded_graphics::pixelcolor::{Rgb565, RgbColor};
use embedded_graphics::text::renderer::CharacterStyle;
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use embedded_graphics::Drawable;
use embedded_hal::digital::v2::OutputPin;
use max31865::FilterMode::Filter50Hz;
use max31865::SensorType::TwoOrFourWire;
use max31865::{temp_conversion, Max31865};
use ssd1351::builder::Builder;
use ssd1351::mode::GraphicsMode;
use ssd1351::prelude::SSD1351_SPI_MODE;
use ssd1351::properties::DisplayRotation;
use stm32f1xx_hal::delay::Delay;
use stm32f1xx_hal::gpio::gpioa::{PA0, PA1};
use stm32f1xx_hal::gpio::Edge;
use stm32f1xx_hal::gpio::{ExtiPin, Floating, Input};
use stm32f1xx_hal::pac;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::spi::Spi;
use stm32f1xx_hal::time::MonoTimer;
use stm32f1xx_hal::timer::{CountDownTimer, Event, Timer};
use tinytga::DynamicTga;

const RECENTLY: u32 = 2_000_000;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        #[init(1)]
        seconds: u32,
        #[init(true)]
        reset_display: bool,
        #[init(Unit::Degrees)]
        unit: Unit,
        #[init(Scale::Seconds)]
        scale: Scale,

        timer_handler: CountDownTimer<pac::TIM1>,
        mono_timer: MonoTimer,

        display: Display,

        pa0: Button<PA0<Input<Floating>>>,
        pa1: Button<PA1<Input<Floating>>>,

        temps: SharedBusResources<BusType>,
        temps_values: TempsValues,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let mut rcc = cx.device.RCC.constrain();
        let mut afio = cx.device.AFIO.constrain(&mut rcc.apb2);
        let mut flash = cx.device.FLASH.constrain();
        let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);

        let clocks = rcc.cfgr.use_hse(8.mhz()).freeze(&mut flash.acr);

        // Setup display
        let mut nss = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
        nss.set_low().unwrap();
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
            2_000_000.hz(),
            clocks,
            &mut rcc.apb2,
        );

        let mut display: GraphicsMode<_> = Builder::new().connect_spi(spi, dc).into();

        let mut delay = Delay::new(cx.core.SYST, clocks);

        display.reset(&mut rst, &mut delay).unwrap();
        display.init().unwrap();
        display.set_rotation(DisplayRotation::Rotate180).unwrap();

        let mono_timer = MonoTimer::new(cx.core.DWT, cx.core.DCB, clocks);

        // Setup Buttons
        let mut pa0 = gpioa.pa0.into_floating_input(&mut gpioa.crl);
        pa0.make_interrupt_source(&mut afio);
        pa0.trigger_on_edge(&cx.device.EXTI, Edge::RISING);
        pa0.enable_interrupt(&cx.device.EXTI);
        let pa0 = Button {
            pin: pa0,
            last: mono_timer.now(),
        };

        let mut pa1 = gpioa.pa1.into_floating_input(&mut gpioa.crl);
        pa1.make_interrupt_source(&mut afio);
        pa1.trigger_on_edge(&cx.device.EXTI, Edge::RISING);
        pa1.enable_interrupt(&cx.device.EXTI);
        let pa1 = Button {
            pin: pa1,
            last: mono_timer.now(),
        };

        // Configure the syst timer to trigger an update every second and enables interrupt
        let mut timer_handler =
            Timer::tim1(cx.device.TIM1, &clocks, &mut rcc.apb2).start_count_down(1.hz());
        timer_handler.listen(Event::Update);

        let mut gpiob = cx.device.GPIOB.split(&mut rcc.apb2);

        let pins = (
            gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh), // sck
            gpiob.pb14.into_floating_input(&mut gpiob.crh),      // miso
            gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh), // mosi
        );

        let spi = Spi::spi2(
            cx.device.SPI2,
            pins,
            max31865::MODE,
            2_000_000.hz(),
            clocks,
            &mut rcc.apb1,
        );

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

        let image_data = include_bytes!("../assets/pegaso_avionics.tga");
        let tga = DynamicTga::from_slice(image_data).unwrap();
        let image = Image::new(&tga, Point::zero());
        image.draw(&mut display).unwrap();

        let text_style = MonoTextStyle::new(&FONT_8X13, Rgb565::WHITE);
        Text::new("Pegaso Avionics", Point::new(4, 118), text_style)
            .draw(&mut display)
            .unwrap();
        delay.delay_ms(2000u16);

        init::LateResources {
            temps_values: TempsValues::default(),
            timer_handler,
            mono_timer,
            display,
            pa0,
            pa1,
            temps,
        }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = TIM1_UP, priority = 1, resources = [timer_handler, seconds, temps, temps_values, scale, unit, display, reset_display])]
    fn tick(mut cx: tick::Context) {
        let seconds = cx.resources.seconds;
        let display = cx.resources.display;
        let temps_values = cx.resources.temps_values;
        let reset_display = cx.resources.reset_display.lock(|reset_display| {
            let temp = *reset_display;
            *reset_display = false;
            temp
        });
        let scale = cx.resources.scale.lock(|scale| *scale);
        let unit = cx.resources.unit.lock(|unit| *unit);

        let mut buffer: WriteBuffer<20> = WriteBuffer::new();

        let ohms1 = cx.resources.temps.t1.read_ohms().unwrap();
        let t1 = temp_conversion::LOOKUP_VEC_PT1000.lookup_temperature(ohms1 as i32);
        temps_values.store(t1 as i16, *seconds, 0);

        let ohms2 = cx.resources.temps.t2.read_ohms().unwrap();
        let t2 = temp_conversion::LOOKUP_VEC_PT1000.lookup_temperature(ohms2 as i32);
        temps_values.store(t2 as i16, *seconds, 1);

        if reset_display {
            display.clear();
            draw_titles(display, scale, &mut buffer);
        }

        let last_degrees = match (temps_values.last(0), temps_values.last(1)) {
            (Some(t1), Some(t2)) => [t1, t2],
            _ => return,
        };

        let last = match unit {
            Unit::Degrees => last_degrees,
            Unit::Fahrenheit => [fahrenheit(last_degrees[0]), fahrenheit(last_degrees[1])],
        };
        let color = [color(last_degrees[0]), color(last_degrees[1])];
        let temp_position = [Point::new(136, 0), Point::new(136, 60)];
        let hist_position = [Point::new(0, 25), Point::new(0, 85)];
        let hist_size = Size::new(128, 30);

        for i in 0..=1usize {
            let mut font = MonoTextStyle::new(&iso_8859_1::FONT_10X20, color[i]);
            font.set_background_color(Some(Rgb565::BLACK));
            let style = TextStyleBuilder::new()
                .alignment(Alignment::Right)
                .baseline(Baseline::Top)
                .build();

            write!(buffer, "  ").unwrap();
            format_100(last[i], &mut buffer);
            write!(buffer, "{}", unit).unwrap();
            Text::with_text_style(buffer.as_str().unwrap(), temp_position[i], font, style)
                .draw(display)
                .unwrap();
            buffer.reset();

            let hist = Hist::new(hist_position[i].clone(), hist_size.clone());
            hist.draw(
                temps_values.series(i, scale),
                display,
                RgbColor::GREEN,
                RgbColor::BLACK,
            )
            .unwrap();
        }
        *seconds += 1;

        // Clears the update flag
        cx.resources.timer_handler.clear_update_interrupt_flag();
    }

    #[task(binds = EXTI0, priority = 2, resources = [pa0, unit, mono_timer])]
    fn exti0(cx: exti0::Context) {
        let button = cx.resources.pa0;
        if button.last.elapsed() > RECENTLY {
            button.last = cx.resources.mono_timer.now();
            cx.resources.unit.next();
        }
        button.pin.clear_interrupt_pending_bit();
    }

    #[task(binds = EXTI1, priority = 2, resources = [pa1, scale, reset_display, mono_timer])]
    fn exti1(cx: exti1::Context) {
        let button = cx.resources.pa1;
        if button.last.elapsed() > RECENTLY {
            button.last = cx.resources.mono_timer.now();
            cx.resources.scale.next();
            *cx.resources.reset_display = true;
        }
        button.pin.clear_interrupt_pending_bit();
    }

    extern "C" {
        fn TAMPER();
    }
};

/// `degrees` is degrees multiplied by 100, eg 3.31 °C is 331
/// returned value is fahrenheit multiplied by 100, eg 22.41 °F is 2241
fn fahrenheit(degrees: i16) -> i16 {
    let f = degrees as i32;
    (f * 9 / 5 + 3200) as i16
}

/// color of the text printing degrees
fn color(degrees: i16) -> Rgb565 {
    if degrees < 0 {
        RgbColor::RED
    } else if degrees < 1500 {
        RgbColor::YELLOW
    } else {
        RgbColor::GREEN
    }
}

/// format a value multiplied by 100 into a decimal number with 1 digit after the dot
fn format_100<const N: usize>(val: i16, buf: &mut WriteBuffer<N>) {
    let sign = if val > 0 { "" } else { "-" };
    let abs_val = val.abs();
    let before_comma = abs_val / 100;
    let after_comma = (abs_val % 100) / 10;
    write!(buf, "{}{}.{}", sign, before_comma, after_comma).unwrap();
}

/// Draw the texts that needs only to be re-drawn only on reset
fn draw_titles<const N: usize>(display: &mut Display, scale: Scale, buffer: &mut WriteBuffer<N>) {
    let text_style_small = MonoTextStyle::new(&FONT_8X13, Rgb565::WHITE);

    Text::with_baseline("OAT", Point::new(0, 0), text_style_small, Baseline::Top)
        .draw(display)
        .unwrap();

    Text::with_baseline("CAT", Point::new(0, 60), text_style_small, Baseline::Top)
        .draw(display)
        .unwrap();

    let text_style_small = MonoTextStyle::new(&FONT_6X9, Rgb565::WHITE);

    write!(buffer, "{}", scale).unwrap();
    Text::with_baseline(
        buffer.as_str().unwrap(),
        Point::new(0, 119),
        text_style_small,
        Baseline::Top,
    )
    .draw(display)
    .unwrap();
    buffer.reset();
}
