#![no_std]
#![no_main]

mod temps;

use panic_halt as _;

use rtic::app;

use crate::temps::TempsValues;
use core::fmt::{self, Formatter, Write};
use e_ring::hist::Hist;
use e_write_buffer::WriteBuffer;
use embedded_graphics::drawable::Drawable;
use embedded_graphics::fonts::{Font12x16, Font6x8, Text};
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::pixelcolor::{Rgb565, RgbColor};
use embedded_graphics::style::TextStyleBuilder;
use embedded_hal::digital::v2::OutputPin;
use max31865::FilterMode::Filter50Hz;
use max31865::SensorType::TwoOrFourWire;
use max31865::{temp_conversion, Max31865};
use shared_bus_rtic::SharedBus;
use ssd1351::builder::Builder;
use ssd1351::interface::SpiInterface;
use ssd1351::mode::GraphicsMode;
use ssd1351::prelude::SSD1351_SPI_MODE;
use ssd1351::properties::DisplayRotation;
use stm32f1xx_hal::delay::Delay;
use stm32f1xx_hal::gpio::gpioa::{PA0, PA1, PA3, PA5, PA6, PA7};
use stm32f1xx_hal::gpio::gpiob::{PB0, PB1, PB10, PB11, PB13, PB14, PB15};
use stm32f1xx_hal::gpio::{Alternate, Edge};
use stm32f1xx_hal::gpio::{ExtiPin, Floating, Input, Output, PushPull};
use stm32f1xx_hal::pac;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::spi::{Spi, Spi1NoRemap, Spi2NoRemap};
use stm32f1xx_hal::time::{Instant, MonoTimer};
use stm32f1xx_hal::timer::{CountDownTimer, Event, Timer};

const RECENTLY: u32 = 2_000_000;

#[rustfmt::skip]
type BusType = Spi<stm32f1xx_hal::pac::SPI2, Spi2NoRemap, (PB13<Alternate<PushPull>>, PB14<Input<Floating>>, PB15<Alternate<PushPull>>), u8>;
#[rustfmt::skip]
type Display = GraphicsMode<SpiInterface<Spi<stm32f1xx_hal::pac::SPI1, Spi1NoRemap, (PA5<Alternate<PushPull>>, PA6<Input<Floating>>, PA7<Alternate<PushPull>>, ), u8, >, PA3<Output<PushPull>>, >, >;

pub struct SharedBusResources<T: 'static> {
    t1: Max31865<SharedBus<T>, PB0<Output<PushPull>>, PB1<Input<Floating>>>,
    t2: Max31865<SharedBus<T>, PB11<Output<PushPull>>, PB10<Input<Floating>>>,
}

pub enum Unit {
    Degrees,
    Fahrenheit,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::Degrees => write!(f, "°C"),
            Unit::Fahrenheit => write!(f, "°F"),
        }
    }
}

impl Unit {
    fn next(&mut self) {
        *self = match self {
            Unit::Degrees => Unit::Fahrenheit,
            Unit::Fahrenheit => Unit::Degrees,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Scale {
    Seconds,
    TenSeconds,
    Minute,
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Scale::Seconds => write!(f, "1 second"),
            Scale::TenSeconds => write!(f, "10 seconds"),
            Scale::Minute => write!(f, "1 minute"),
        }
    }
}

impl Scale {
    fn next(&mut self) {
        *self = match self {
            Scale::Seconds => Scale::TenSeconds,
            Scale::TenSeconds => Scale::Minute,
            Scale::Minute => Scale::Seconds,
        };
    }
    fn seconds(&self) -> u32 {
        match self {
            Scale::Seconds => 1,
            Scale::TenSeconds => 10,
            Scale::Minute => 60,
        }
    }
}

pub struct Button<T> {
    pin: T,
    last: Instant,
}

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        timer_handler: CountDownTimer<pac::TIM1>,
        seconds: u32,
        mono_timer: MonoTimer,

        display: Display,
        reset_display: bool,

        pa0: Button<PA0<Input<Floating>>>,
        pa1: Button<PA1<Input<Floating>>>,

        temps: SharedBusResources<BusType>,
        temps_values: TempsValues,

        unit: Unit,
        scale: Scale,
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
        let mut timer =
            Timer::tim1(cx.device.TIM1, &clocks, &mut rcc.apb2).start_count_down(1.hz());
        timer.listen(Event::Update);

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

        init::LateResources {
            timer_handler: timer,
            temps_values: TempsValues::default(),
            mono_timer,
            seconds: 1,
            display,
            reset_display: true,
            pa0,
            pa1,
            temps,
            unit: Unit::Degrees,
            scale: Scale::Seconds,
        }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = TIM1_UP, priority = 1, spawn = [screen], resources = [timer_handler, seconds, temps, temps_values])]
    fn tick(cx: tick::Context) {
        let seconds = cx.resources.seconds;

        let ohms1 = cx.resources.temps.t1.read_ohms().unwrap();
        let t1 = temp_conversion::LOOKUP_VEC_PT1000.lookup_temperature(ohms1 as i32);
        cx.resources.temps_values.store(t1 as i16, *seconds, 0);

        let ohms2 = cx.resources.temps.t2.read_ohms().unwrap();
        let t2 = temp_conversion::LOOKUP_VEC_PT1000.lookup_temperature(ohms2 as i32);
        cx.resources.temps_values.store(t2 as i16, *seconds, 1);

        cx.spawn.screen().unwrap();

        *seconds += 1;
        // Clears the update flag
        cx.resources.timer_handler.clear_update_interrupt_flag();
    }

    #[task(binds = EXTI0, priority = 1, resources = [pa0, unit, mono_timer])]
    fn exti0(cx: exti0::Context) {
        let button = cx.resources.pa0;
        if button.last.elapsed() > RECENTLY {
            button.last = cx.resources.mono_timer.now();
            cx.resources.unit.next();
        }
        button.pin.clear_interrupt_pending_bit();
    }

    #[task(binds = EXTI1, priority = 1, resources = [pa1, scale, reset_display, mono_timer])]
    fn exti1(cx: exti1::Context) {
        let button = cx.resources.pa1;
        if button.last.elapsed() > RECENTLY {
            button.last = cx.resources.mono_timer.now();
            cx.resources.scale.next();
            *cx.resources.reset_display = true;
        }
        button.pin.clear_interrupt_pending_bit();
    }

    #[task(resources = [display, reset_display, temps_values, scale, unit])]
    fn screen(cx: screen::Context) {
        let mut buffer: WriteBuffer<20> = WriteBuffer::new();
        let mut pad_buffer: WriteBuffer<20> = WriteBuffer::new();

        let display = cx.resources.display;
        let scale = cx.resources.scale;
        let unit = cx.resources.unit;
        let reset_display = cx.resources.reset_display;

        if *reset_display {
            display.clear();
            draw_titles(display, *scale, &mut buffer);
            *reset_display = false;
        }

        let last_degrees = match (
            cx.resources.temps_values.last(0),
            cx.resources.temps_values.last(1),
        ) {
            (Some(t1), Some(t2)) => [t1, t2],
            _ => return,
        };

        let last = match unit {
            Unit::Degrees => last_degrees,
            Unit::Fahrenheit => [fahrenheit(last_degrees[0]), fahrenheit(last_degrees[1])],
        };
        let color = [color(last_degrees[0]), color(last_degrees[1])];
        let temp_position = [Point::new(0, 10), Point::new(0, 70)];
        let hist_position = [Point::new(0, 25), Point::new(0, 85)];
        let hist_size = Size::new(128, 30);

        for i in 0..=1usize {
            let text_style_big = TextStyleBuilder::new(Font12x16)
                .text_color(color[i])
                .background_color(RgbColor::BLACK)
                .build();

            format_100(last[i], &mut buffer);
            write!(buffer, " {}", unit).unwrap();
            write!(pad_buffer, "{:>10}", buffer.as_str().unwrap()).unwrap();
            Text::new(pad_buffer.as_str().unwrap(), temp_position[i])
                .into_styled(text_style_big)
                .draw(display)
                .unwrap();
            buffer.reset();
            pad_buffer.reset();

            let hist = Hist::new(hist_position[i].clone(), hist_size.clone());
            hist.draw(
                cx.resources.temps_values.series(i, *scale),
                display,
                RgbColor::GREEN,
                RgbColor::BLACK,
            )
            .unwrap();
        }
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
        RgbColor::WHITE
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

fn draw_titles<const N: usize>(display: &mut Display, scale: Scale, buffer: &mut WriteBuffer<N>) {
    let text_style_small = TextStyleBuilder::new(Font6x8)
        .text_color(RgbColor::YELLOW)
        .background_color(RgbColor::BLACK)
        .build();

    Text::new("OAT", Point::new(0, 2))
        .into_styled(text_style_small)
        .draw(display)
        .unwrap();

    Text::new("CAT", Point::new(0, 62))
        .into_styled(text_style_small)
        .draw(display)
        .unwrap();

    let text_style_small = TextStyleBuilder::new(Font6x8)
        .text_color(RgbColor::WHITE)
        .background_color(RgbColor::BLACK)
        .build();
    write!(buffer, "{}", scale).unwrap();
    Text::new(buffer.as_str().unwrap(), Point::new(0, 120))
        .into_styled(text_style_small)
        .draw(display)
        .unwrap();
    buffer.reset();
}
