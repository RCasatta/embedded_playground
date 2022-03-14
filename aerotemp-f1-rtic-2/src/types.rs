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

// pub type PA2 = Pin<Input<PullUp>, CRL, 'A', 2_u8>;
pub type PA3 = Pin<Output<PushPull>, CRL, 'A', 3_u8>;

// pub type PA4 = Pin<Input<PullUp>, CRL, 'A', 4_u8>;

pub type PA5 = Pin<Alternate<PushPull>, CRL, 'A', 5_u8>;
pub type PA6 = Pin<Input<Floating>, CRL, 'A', 6_u8>;
pub type PA7 = Pin<Alternate<PushPull>, CRL, 'A', 7_u8>;

pub type SPI1 = spi::Spi<pac::SPI1, Spi1NoRemap, (PA5, PA6, PA7), u8>;

pub type Display = GraphicsMode<SpiInterface<SPI1, PA3>>;
