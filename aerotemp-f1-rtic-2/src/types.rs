use crate::temp::Temp;
use ssd1351::{interface::SpiInterface, mode::GraphicsMode};
use stm32f1xx_hal::{
    gpio::{Alternate, Floating, Input, Output, Pin, PullUp, PushPull, CRL},
    spi::Spi1NoRemap,
};
use stm32f1xx_hal::{pac, spi};
use systick_monotonic::fugit;

pub type Instant = fugit::Instant<u64, 1, 1000>;
pub type Duration = fugit::Duration<u64, 1, 1000>;

pub type PA0 = Pin<Input<PullUp>, CRL, 'A', 0_u8>;
pub type PA1 = Pin<Input<PullUp>, CRL, 'A', 1_u8>;
pub type PA3 = Pin<Output<PushPull>, CRL, 'A', 3_u8>;
pub type PA5 = Pin<Alternate<PushPull>, CRL, 'A', 5_u8>;
pub type PA6 = Pin<Input<Floating>, CRL, 'A', 6_u8>;
pub type PA7 = Pin<Alternate<PushPull>, CRL, 'A', 7_u8>;

pub type SPI1 = spi::Spi<pac::SPI1, Spi1NoRemap, (PA5, PA6, PA7), u8>;

pub type Display = GraphicsMode<SpiInterface<SPI1, PA3>>;

pub type Temps = [Temp; 2];

pub const PERIOD: usize = 2;
pub const SCREEN_WIDTH: usize = 128;
pub const SCREEN_WIDTH_PLUS_1: usize = SCREEN_WIDTH + 1;
pub const ENOUGH_TIME_BUTTON_PRESSED: Duration = Duration::from_ticks(200);

pub const ONE_SEC: Duration = Duration::from_ticks(1_000);
pub const ZERO_INSTANT: Instant = Instant::from_ticks(0);
pub const TITLES: [&'static str; 2] = ["OAT", "CAT"];
pub const MIN_OR_MAX: [&'static str; 2] = ["min:", "max:"];
