#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f1xx_hal::{adc, delay, pac, prelude::*, stm32};

use dht_sensor::{dht22, DhtReading};
use embedded_hal::digital::v2::OutputPin;

use ssd1306::{prelude::*, Builder, I2CDIBuilder};
use stm32f1xx_hal::i2c::{BlockingI2c, DutyCycle, Mode};

use core::fmt::Write;
use e_ring::hist::Hist;
use e_ring::Ring;
use e_write_buffer::WriteBuffer;
use embedded_graphics::fonts::{Font6x8, Text};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::style::TextStyleBuilder;
use stm32f1xx_hal::time::{Instant, MonoTimer};

const EVERY: u32 = 500;

#[entry]
fn main() -> ! {
    // Acquire peripherals
    let p = pac::Peripherals::take().unwrap();
    let cp = stm32::CorePeripherals::take().unwrap();

    let mut flash = p.FLASH.constrain();
    let mut rcc = p.RCC.constrain();

    // Configure ADC clocks
    // Default value is the slowest possible ADC clock: PCLK2 / 8. Meanwhile ADC
    // clock is configurable. So its frequency may be tweaked to meet certain
    // practical needs. User specified value is be approximated using supported
    // prescaler values 2/4/6/8.
    let clocks = rcc
        .cfgr
        .adcclk(2.mhz())
        .use_hse(8.mhz())
        .freeze(&mut flash.acr);
    //hprintln!("adc freq: {}", clocks.adcclk().0).unwrap();

    // Setup ADC
    let mut adc1 = adc::Adc::adc1(p.ADC1, &mut rcc.apb2, clocks);
    let mut adc2 = adc::Adc::adc2(p.ADC2, &mut rcc.apb2, clocks);

    // Setup GPIOB
    let mut gpiob = p.GPIOB.split(&mut rcc.apb2);

    // Configure pb0 as an analog input
    let mut ch0 = gpiob.pb0.into_analog(&mut gpiob.crl);
    let mut ch1 = gpiob.pb1.into_analog(&mut gpiob.crl);

    // This is used by `dht-sensor` to wait for signals
    let mut delay = delay::Delay::new(cp.SYST, clocks);

    let mut pb5 = gpiob.pb5.into_open_drain_output(&mut gpiob.crl);

    pb5.set_high().unwrap();

    let scl = gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh);
    let sda = gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh);

    let i2c = BlockingI2c::i2c2(
        p.I2C2,
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
    let mut disp: GraphicsMode<_, _> = Builder::new().connect(interface).into();
    disp.init().unwrap();

    let text_style = TextStyleBuilder::new(Font6x8)
        .text_color(BinaryColor::On)
        .background_color(BinaryColor::Off)
        .build();

    let mut buffer: WriteBuffer<20> = WriteBuffer::new();

    let mut ring_battery: Ring<u16, 32> = Ring::new();
    let mut ring_moisture: Ring<u16, 32> = Ring::new();

    let mut battery_chart: Ring<u16, 128> = Ring::new();
    let mut moisture_chart: Ring<i16, 128> = Ring::new();
    let hist = Hist::new(Point::new(0, 44), Point::new(128, 64)).unwrap();

    let mut cycle_count = 1u32;
    let mut cycle_time = 0u32;

    // The DHT11 datasheet suggests 1 second
    //hprintln!("Waiting on the sensor...").unwrap();
    delay.delay_ms(1000_u16);

    let timer = MonoTimer::new(cp.DWT, cp.DCB, clocks);
    let mut last_dht = timer.now();

    loop {
        let start = timer.now();
        let data: u16 = adc1.read(&mut ch0).unwrap();
        ring_moisture.append(data);

        write!(buffer, "Moisture {:>9}", ring_moisture.avg() as u32).unwrap();
        Text::new(&buffer.as_str().unwrap(), Point::zero())
            .into_styled(text_style)
            .draw(&mut disp)
            .unwrap();
        buffer.reset();

        let data2: u16 = adc2.read(&mut ch1).unwrap();
        ring_battery.append(data2);
        write!(buffer, "Battery {:>10}", ring_battery.avg() as u32).unwrap();
        Text::new(&buffer.as_str().unwrap(), Point::new(0, 12))
            .into_styled(text_style)
            .draw(&mut disp)
            .unwrap();
        buffer.reset();

        if timer.milliseconds_elapsed(last_dht) > 1000 {
            // dht must not be read more than once a sec
            last_dht = timer.now();
            if let Ok(dht22::Reading {
                          temperature,
                          relative_humidity,
                      }) = dht22::Reading::read(&mut delay, &mut pb5)
            {
                write!(
                    buffer,
                    "Temp {:>2}° RH {:>2}%",
                    temperature as u32, relative_humidity as u32
                )
                    .unwrap();
                Text::new(&buffer.as_str().unwrap(), Point::new(0, 24))
                    .into_styled(text_style)
                    .draw(&mut disp)
                    .unwrap();
                buffer.reset();
            }
        }

        write!(buffer, "{} {}", cycle_count, cycle_time).unwrap();
        Text::new(&buffer.as_str().unwrap(), Point::new(0, 36))
            .into_styled(text_style)
            .draw(&mut disp)
            .unwrap();
        buffer.reset();

        if cycle_count % EVERY == 0 {
            battery_chart.append(ring_battery.avg() as u16);
            moisture_chart.append(ring_moisture.avg() as i16);
        }

        hist.draw(
            &moisture_chart,
            &mut disp,
            BinaryColor::On,
            BinaryColor::Off,
        )
        .unwrap();

        disp.flush().unwrap();

        cycle_time = timer.milliseconds_elapsed(start);
        cycle_count += 1;
    }
}

trait Elapsed {
    fn seconds_elapsed(&self, start: Instant) -> f32;
    fn milliseconds_elapsed(&self, start: Instant) -> u32;
}

impl Elapsed for MonoTimer {
    fn seconds_elapsed(&self, start: Instant) -> f32 {
        start.elapsed() as f32 / self.frequency().0 as f32
    }
    fn milliseconds_elapsed(&self, start: Instant) -> u32 {
        (self.seconds_elapsed(start) * 1000.0) as u32
    }
}
