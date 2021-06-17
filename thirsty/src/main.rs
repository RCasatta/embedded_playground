#![no_std]
#![no_main]

mod sensors;
mod types;

// you can put a breakpoint on `rust_begin_unwind` to catch panics
use panic_halt as _;

use rtic::app;

use crate::sensors::{Battery, ButtonA, ButtonB, Moisture, TempHumidity};
use crate::types::{OnScreen, TimeSlice};
use core::fmt::Write;
use e_ring::hist::Hist;
use e_ring::Ring;
use e_write_buffer::WriteBuffer;
use embedded_graphics::drawable::Drawable;
use embedded_graphics::fonts::{Font6x8, Text};
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::style::TextStyleBuilder;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use ssd1306::displaysize::DisplaySize128x64;
use ssd1306::mode::GraphicsMode;
use ssd1306::prelude::I2CInterface;
use ssd1306::{Builder, I2CDIBuilder};
use stm32f1xx_hal::adc::Adc;
use stm32f1xx_hal::gpio::gpiob::{PB10, PB11};
use stm32f1xx_hal::gpio::{Alternate, Edge, ExtiPin, OpenDrain};
use stm32f1xx_hal::i2c::BlockingI2c;
use stm32f1xx_hal::i2c::{DutyCycle, Mode};
use stm32f1xx_hal::pac::I2C2;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::time::MonoTimer;
use stm32f1xx_hal::timer::{CountDownTimer, Event, Timer};
use stm32f1xx_hal::{delay, pac};

const RECENTLY: u32 = 2_000_000;

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

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        timer_handler: CountDownTimer<pac::TIM1>,
        mono_timer: MonoTimer,
        seconds: u32,

        battery: Battery,
        moisture: Moisture,
        temp_humidity: TempHumidity,
        button_a: ButtonA,
        button_b: ButtonB,

        on_screen: OnScreen,
        time_slice: TimeSlice,

        display: GraphicsMode<
            I2CInterface<
                BlockingI2c<I2C2, (PB10<Alternate<OpenDrain>>, PB11<Alternate<OpenDrain>>)>,
            >,
            DisplaySize128x64,
        >,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        hprintln!("{}", "init");
        let mut flash = cx.device.FLASH.constrain();
        let mut rcc = cx.device.RCC.constrain();
        let mut afio = cx.device.AFIO.constrain(&mut rcc.apb2);

        let clocks = rcc
            .cfgr
            .adcclk(2.mhz())
            .use_hse(8.mhz())
            .freeze(&mut flash.acr);

        // Acquire the GPIO peripherals
        let mut gpioa = cx.device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = cx.device.GPIOB.split(&mut rcc.apb2);

        // Setup Moisture
        let moisture_adc = Adc::adc1(cx.device.ADC1, &mut rcc.apb2, clocks);
        let ch0 = gpiob.pb0.into_analog(&mut gpiob.crl);
        let moisture = Moisture {
            adc: moisture_adc,
            channel: ch0,
            values: [Ring::new(), Ring::new(), Ring::new()],
        };

        // Setup Battery
        let battery_adc = Adc::adc2(cx.device.ADC2, &mut rcc.apb2, clocks);
        let ch1 = gpiob.pb1.into_analog(&mut gpiob.crl);
        let battery = Battery {
            adc: battery_adc,
            channel: ch1,
            values: [Ring::new(), Ring::new(), Ring::new()],
        };

        // Setup Temp and Humidity
        let delay = delay::Delay::new(cx.core.SYST, clocks);
        let mut dht_pin = gpiob.pb5.into_open_drain_output(&mut gpiob.crl);
        dht_pin.set_high().unwrap();
        let temp_humidity = TempHumidity {
            delay,
            dht_pin,
            temp_values: [Ring::new(), Ring::new(), Ring::new()],
            humidity_values: [Ring::new(), Ring::new(), Ring::new()],
        };

        // Setup button
        let mono_timer = MonoTimer::new(cx.core.DWT, cx.core.DCB, clocks);
        let mut button_a_pin = gpioa.pa7.into_floating_input(&mut gpioa.crl);
        button_a_pin.make_interrupt_source(&mut afio);
        button_a_pin.trigger_on_edge(&cx.device.EXTI, Edge::FALLING);
        button_a_pin.enable_interrupt(&cx.device.EXTI);
        let mut button_b_pin = gpioa.pa6.into_floating_input(&mut gpioa.crl);
        button_b_pin.make_interrupt_source(&mut afio);
        button_b_pin.trigger_on_edge(&cx.device.EXTI, Edge::FALLING);
        button_b_pin.enable_interrupt(&cx.device.EXTI);

        let button_a = ButtonA {
            pin: button_a_pin,
            last: mono_timer.now(),
        };

        let button_b = ButtonB {
            pin: button_b_pin,
            last: mono_timer.now(),
        };

        // Setup display
        let scl = gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh);
        let sda = gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh);
        let i2c = BlockingI2c::i2c2(
            cx.device.I2C2,
            (scl, sda),
            Mode::Fast {
                frequency: 400_000.hz(),
                duty_cycle: DutyCycle::Ratio2to1,
            },
            clocks,
            &mut rcc.apb1,
            1000,
            10,
            1000,
            1000,
        );
        let interface = I2CDIBuilder::new().init(i2c);
        let mut display: GraphicsMode<_, _> = Builder::new().connect(interface).into();
        display.init().unwrap();

        // Configure the syst timer to trigger an update every second and enables interrupt
        let mut timer =
            Timer::tim1(cx.device.TIM1, &clocks, &mut rcc.apb2).start_count_down(1.hz());
        timer.listen(Event::Update);

        // Init the static resources to use them later through RTIC
        init::LateResources {
            timer_handler: timer,
            mono_timer,
            seconds: 1,
            time_slice: TimeSlice::Second,
            battery,
            moisture,
            temp_humidity,
            on_screen: OnScreen::Battery,
            button_a,
            button_b,
            display,
        }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = TIM1_UP, priority = 1, spawn = [screen], resources = [timer_handler, battery, moisture, temp_humidity, seconds])]
    fn tick(cx: tick::Context) {
        cx.resources.battery.read_and_store(*cx.resources.seconds);
        cx.resources.moisture.read_and_store(*cx.resources.seconds);
        cx.resources
            .temp_humidity
            .read_and_store(*cx.resources.seconds);
        *cx.resources.seconds += 1;

        cx.spawn.screen().unwrap();

        // Clears the update flag
        cx.resources.timer_handler.clear_update_interrupt_flag();
    }

    #[task(binds = EXTI9_5, priority = 1, spawn = [screen], resources = [on_screen, button_a, button_b, mono_timer, time_slice])]
    fn button(cx: button::Context) {
        if cx.resources.button_a.pin.is_low().unwrap() {
            if cx.resources.button_a.last.elapsed() > RECENTLY {
                cx.resources.button_a.last = cx.resources.mono_timer.now();
                cx.resources.on_screen.next();
                cx.spawn.screen().unwrap();
            }
        }

        if cx.resources.button_b.pin.is_low().unwrap() {
            if cx.resources.button_b.last.elapsed() > RECENTLY {
                cx.resources.button_a.last = cx.resources.mono_timer.now();
                cx.resources.time_slice.next();
                cx.spawn.screen().unwrap();
            }
        }

        // Clears the update flag
        cx.resources.button_a.pin.clear_interrupt_pending_bit();
        cx.resources.button_b.pin.clear_interrupt_pending_bit();
    }

    #[task(resources = [battery, moisture, temp_humidity, display, on_screen, time_slice])]
    fn screen(cx: screen::Context) {
        let mut title: WriteBuffer<20> = WriteBuffer::new();
        let mut buffer: WriteBuffer<20> = WriteBuffer::new();
        let time_slice = *cx.resources.time_slice;
        let display = cx.resources.display;
        display.clear();

        write!(title, "{:?}", cx.resources.on_screen).unwrap();
        let ring = match cx.resources.on_screen {
            OnScreen::Battery => &cx.resources.battery.values,
            OnScreen::Moisture => &cx.resources.moisture.values,
            OnScreen::Humidity => &cx.resources.temp_humidity.humidity_values,
            OnScreen::Temperature => &cx.resources.temp_humidity.temp_values,
        };

        let text_style = TextStyleBuilder::new(Font6x8)
            .text_color(BinaryColor::On)
            .background_color(BinaryColor::Off)
            .build();

        if let Some(last) = ring[time_slice as usize].last() {
            write!(
                buffer,
                "{} {:>width$}",
                title,
                last as u32,
                width = 19 - title.len()
            )
            .unwrap();
            Text::new(&buffer.as_str().unwrap(), Point::zero())
                .into_styled(text_style)
                .draw(display)
                .unwrap();
            buffer.reset();

            write!(buffer, "{:?}", cx.resources.time_slice).unwrap();
            Text::new(&buffer.as_str().unwrap(), Point::new(0, 12))
                .into_styled(text_style)
                .draw(display)
                .unwrap();
            buffer.reset();

            let hist = Hist::new(Point::new(0, 28), Size::new(128, 36));
            hist.draw(
                &ring[time_slice as usize],
                display,
                BinaryColor::On,
                BinaryColor::Off,
            )
            .unwrap();
        } else {
            Text::new(&title.as_str().unwrap(), Point::zero())
                .into_styled(text_style)
                .draw(display)
                .unwrap();

            write!(buffer, "{:?}", cx.resources.time_slice).unwrap();
            Text::new(&buffer.as_str().unwrap(), Point::new(0, 12))
                .into_styled(text_style)
                .draw(display)
                .unwrap();
            buffer.reset();
        }
        display.flush().unwrap();
    }

    extern "C" {
        fn TAMPER();
    }
};
