use crate::temp::Temp;
use max31865::Max31865;
use shared_bus_rtic::SharedBus;
use ssd1351::{interface::SpiInterface, mode::GraphicsMode};
use stm32f1xx_hal::pac;
use stm32f1xx_hal::{
    gpio::{Alternate, Floating, Input, Output, Pin, PullUp, PushPull, CRH, CRL},
    spi::{Spi, Spi1NoRemap, Spi2NoRemap},
};
use systick_monotonic::fugit;

pub type Instant = fugit::Instant<u64, 1, 1000>;
pub type Duration = fugit::Duration<u64, 1, 1000>;

pub type PA0 = Pin<Input<PullUp>, CRL, 'A', 0_u8>;
pub type PA1 = Pin<Input<PullUp>, CRL, 'A', 1_u8>;
pub type PA3 = Pin<Output<PushPull>, CRL, 'A', 3_u8>;
pub type PA5 = Pin<Alternate<PushPull>, CRL, 'A', 5_u8>;
pub type PA6 = Pin<Input<Floating>, CRL, 'A', 6_u8>;
pub type PA7 = Pin<Alternate<PushPull>, CRL, 'A', 7_u8>;

pub type PB0 = Pin<Output<PushPull>, CRL, 'B', 0_u8>;
pub type PB1 = Pin<Input<Floating>, CRL, 'B', 1_u8>;

pub type PB11 = Pin<Output<PushPull>, CRH, 'B', 11_u8>;
pub type PB10 = Pin<Input<Floating>, CRH, 'B', 10_u8>;

pub type PB13 = Pin<Alternate<PushPull>, CRH, 'B', 13_u8>;
pub type PB14 = Pin<Input<Floating>, CRH, 'B', 14_u8>;
pub type PB15 = Pin<Alternate<PushPull>, CRH, 'B', 15_u8>;

pub type SPI1 = Spi<pac::SPI1, Spi1NoRemap, (PA5, PA6, PA7), u8>;

pub type BusType = Spi<pac::SPI2, Spi2NoRemap, (PB13, PB14, PB15), u8>;

pub type Display = GraphicsMode<SpiInterface<SPI1, PA3>>;

pub struct SharedBusResources<T: 'static> {
    pub t1: Max31865<SharedBus<T>, PB0, PB1>,
    pub t2: Max31865<SharedBus<T>, PB11, PB10>,
}

pub type Temps = [Temp; 2];

pub const PERIOD: usize = 2;
pub const SCREEN_WIDTH: usize = 128;
pub const SCREEN_WIDTH_PLUS_1: usize = SCREEN_WIDTH + 1;
pub const ENOUGH_TIME_BUTTON_PRESSED: Duration = Duration::from_ticks(200);

pub const ONE_SEC: Duration = Duration::from_ticks(1_000);
pub const ZERO_INSTANT: Instant = Instant::from_ticks(0);
pub const TITLES: [&'static str; 2] = ["OAT", "CAT"];
pub const MIN_OR_MAX: [&'static str; 2] = ["min:", "max:"];
