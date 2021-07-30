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
use stm32f1xx_hal::gpio::gpioa::{PA0, PA1, PA5, PA6, PA7, PA3};
use stm32f1xx_hal::gpio::gpioc::PC13;
use stm32f1xx_hal::gpio::{Edge, Alternate, State};
use stm32f1xx_hal::gpio::{ExtiPin, Floating, Input, Output, PushPull};
use stm32f1xx_hal::prelude::*;
use ssd1351::prelude::SSD1351_SPI_MODE;
use ssd1351::mode::GraphicsMode;
use ssd1351::builder::Builder;
use stm32f1xx_hal::delay::Delay;
use stm32f1xx_hal::spi::{Spi, Spi1NoRemap};
use ssd1351::interface::SpiInterface;
use embedded_graphics::pixelcolor::{RgbColor};
use embedded_graphics::style::{PrimitiveStyleBuilder};
use embedded_graphics::geometry::Point;
use embedded_graphics::drawable::Drawable;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::prelude::Primitive;
use stm32f1xx_hal::time::{MonoTimer, Instant};

pub struct Button<T> {
    pin: T,
    pressed: bool,
}

#[cfg(feature = "semihosting")]
macro_rules! hprintln {
    ($s:expr, $($tt:tt)*) => {
        cortex_m_semihosting::export::hstdout_fmt(format_args!(concat!($s, "\n"), $($tt)*)).unwrap()
    };
}
#[cfg(not(feature = "semihosting"))]
macro_rules! hprintln {
    ($s:expr, $($tt:tt)*) => {};
}

type Display = GraphicsMode<SpiInterface<Spi<stm32f1xx_hal::pac::SPI1, Spi1NoRemap, (PA5<Alternate<PushPull>>, PA6<Input<Floating>>, PA7<Alternate<PushPull>>), u8>, PA3<Output<PushPull>>>>;

pub enum Last {
    None,
    Left,
    Right,
}

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        display: Display,

        pa0: Button<PA0<Input<Floating>>>,
        pa1: Button<PA1<Input<Floating>>>,

        led: PC13<Output<PushPull>>,

        start: Instant,

        last: Last,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let mut rcc = cx.device.RCC.constrain();
        let mut afio = cx.device.AFIO.constrain(&mut rcc.apb2);
        let mut flash = cx.device.FLASH.constrain();
        let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(8.mhz())
            .freeze(&mut flash.acr);
        hprintln!("sysclk: {:?}", clocks.sysclk());

        let timer = MonoTimer::new(cx.core.DWT, cx.core.DCB, clocks);
        let start = timer.now();

        let mut gpioc = cx.device.GPIOC.split(&mut rcc.apb2);

        let led = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, State::High);

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

        draw_square(&mut display, 0, 128, 0, 0);
        draw_square(&mut display, 10, 128, 0, 0);

        // Setup Buttons
        let mut pa0 = gpioa.pa0.into_floating_input(&mut gpioa.crl);
        pa0.make_interrupt_source(&mut afio);
        pa0.trigger_on_edge(&cx.device.EXTI, Edge::RISING_FALLING);
        pa0.enable_interrupt(&cx.device.EXTI);
        let pa0 = Button {
            pin: pa0,
            pressed: false,
        };

        let mut pa1 = gpioa.pa1.into_floating_input(&mut gpioa.crl);
        pa1.make_interrupt_source(&mut afio);
        pa1.trigger_on_edge(&cx.device.EXTI, Edge::RISING_FALLING);
        pa1.enable_interrupt(&cx.device.EXTI);
        let pa1 = Button {
            pin: pa1,
            pressed: false,
        };
        let last = Last::None;

        init::LateResources {
            display,
            pa0,
            pa1,
            led,
            start,
            last,
        }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = EXTI0, priority = 1, spawn = [buttons], resources = [pa0, led])]
    fn exti0(cx: exti0::Context) {
        hprintln!("exti0, p0 is low? {:?}", cx.resources.pa0.pin.is_low());
        cx.resources.pa0.pressed = cx.resources.pa0.pin.is_low().unwrap();
        cx.resources.pa0.pin.clear_interrupt_pending_bit();
        cx.spawn.buttons().unwrap();
        cx.resources.led.toggle().unwrap();
    }

    #[task(binds = EXTI1, priority = 1, spawn = [buttons], resources = [pa1, led])]
    fn exti1(cx: exti1::Context) {
        hprintln!("exti1, p1 is low? {:?}", cx.resources.pa1.pin.is_low());
        cx.resources.pa1.pressed = cx.resources.pa1.pin.is_low().unwrap();
        cx.resources.pa1.pin.clear_interrupt_pending_bit();
        cx.spawn.buttons().unwrap();
        cx.resources.led.toggle().unwrap();
    }

    #[task(resources = [display, pa0, pa1, last, start])]
    fn buttons(cx: buttons::Context) {
        let elapsed = cx.resources.start.elapsed();
        let x = (elapsed % 118) as i32;
        let y = ((elapsed / 118) % 118) as i32;

        let display = cx.resources.display;
        match (cx.resources.pa0.pressed, cx.resources.pa1.pressed) {
            (false, false) => {
                match cx.resources.last {
                    Last::Left => draw_square(display, 1, 10,x,y),
                    Last::Right => draw_square(display, 2, 10,x,y),
                    Last::None => (),
                }
                *cx.resources.last = Last::None;
            },
            (true, false) => *cx.resources.last = Last::Left,
            (false, true) => *cx.resources.last = Last::Right,
            (true, true) => {
                draw_square(display, 3, 10,x,y);
                *cx.resources.last = Last::None;
                cx.resources.pa0.pressed = false;
                cx.resources.pa1.pressed = false;
            },
        }
    }

    extern "C" {
        fn TAMPER();
    }

};

fn draw_square(display: &mut Display, color: u8, size: i32, x: i32, y: i32 ) {
    let color = match color {
        0u8 => RgbColor::WHITE,
        1 => RgbColor::RED,
        2 => RgbColor::GREEN,
        3 => RgbColor::BLUE,
        _ => RgbColor::BLACK,

    };
    let style = PrimitiveStyleBuilder::new()
        .fill_color(color)
        .build();
    Rectangle::new(Point::new(x, y), Point::new(x+size, y+size))
        .into_styled(style)
        .draw(display).unwrap();
}