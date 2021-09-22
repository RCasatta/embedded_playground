use core::fmt;
use max31865::Max31865;
use shared_bus_rtic::SharedBus;
use ssd1351::interface::SpiInterface;
use ssd1351::mode::GraphicsMode;
use stm32f1xx_hal::gpio::gpioa::{PA3, PA5, PA6, PA7};
use stm32f1xx_hal::gpio::gpiob::{PB0, PB1, PB10, PB11, PB13, PB14, PB15};
use stm32f1xx_hal::gpio::{Alternate, Floating, Input, Output, PushPull};
use stm32f1xx_hal::spi::{Spi, Spi1NoRemap, Spi2NoRemap};
use stm32f1xx_hal::time::Instant;

#[rustfmt::skip]
pub type BusType = Spi<stm32f1xx_hal::pac::SPI2, Spi2NoRemap, (PB13<Alternate<PushPull>>, PB14<Input<Floating>>, PB15<Alternate<PushPull>>), u8>;
#[rustfmt::skip]
pub type Display = GraphicsMode<SpiInterface<Spi<stm32f1xx_hal::pac::SPI1, Spi1NoRemap, (PA5<Alternate<PushPull>>, PA6<Input<Floating>>, PA7<Alternate<PushPull>>, ), u8, >, PA3<Output<PushPull>>, >, >;

pub struct SharedBusResources<T: 'static> {
    pub t1: Max31865<SharedBus<T>, PB0<Output<PushPull>>, PB1<Input<Floating>>>,
    pub t2: Max31865<SharedBus<T>, PB11<Output<PushPull>>, PB10<Input<Floating>>>,
}

#[derive(Copy, Clone)]
pub enum Unit {
    Degrees,
    Fahrenheit,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::Degrees => write!(f, "°C"),
            Unit::Fahrenheit => write!(f, "°F"),
        }
    }
}

impl Unit {
    pub fn next(&mut self) {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scale::Seconds => write!(f, "1 second"),
            Scale::TenSeconds => write!(f, "10 seconds"),
            Scale::Minute => write!(f, "1 minute"),
        }
    }
}

impl Scale {
    pub fn next(&mut self) {
        *self = match self {
            Scale::Seconds => Scale::TenSeconds,
            Scale::TenSeconds => Scale::Minute,
            Scale::Minute => Scale::Seconds,
        };
    }
    pub fn seconds(&self) -> u32 {
        match self {
            Scale::Seconds => 1,
            Scale::TenSeconds => 10,
            Scale::Minute => 60,
        }
    }
}

pub struct Button<T> {
    pub pin: T,
    pub last: Instant,
}
