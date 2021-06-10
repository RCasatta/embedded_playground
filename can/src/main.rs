//! Simple CAN example.
//! Requires a transceiver connected to PA11, PA12 (CAN1) or PB5 PB6 (CAN2).

#![no_main]
#![no_std]

use panic_halt as _;

use bxcan::filter::Mask32;
use cortex_m_rt::entry;
use nb::block;
use stm32f1xx_hal::{can::Can, pac, prelude::*};

#[cfg(feature = "sender")]
use cortex_m_semihosting::hprintln;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    // To meet CAN clock accuracy requirements an external crystal or ceramic
    // resonator must be used. The blue pill has a 8MHz external crystal.
    // Other boards might have a crystal with another frequency or none at all.
    rcc.cfgr.use_hse(8.mhz()).freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    let mut can1 = {
        let can = Can::new(dp.CAN1, &mut rcc.apb1, dp.USB);

        let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
        let rx = gpioa.pa11.into_floating_input(&mut gpioa.crh);
        let tx = gpioa.pa12.into_alternate_push_pull(&mut gpioa.crh);
        can.assign_pins((tx, rx), &mut afio.mapr);

        bxcan::Can::new(can)
    };

    // APB1 (PCLK1): 8MHz, Bit rate: 125kBit/s, Sample Point 87.5%
    // Value was calculated with http://www.bittiming.can-wiki.info/
    can1.modify_config().set_bit_timing(0x001c_0003);

    // Configure filters so that can frames can be received.
    let mut filters = can1.modify_filters();
    filters.enable_bank(0, Mask32::accept_all());

    // Drop filters to leave filter configuraiton mode.
    drop(filters);

    // Select the interface.
    let mut can = can1;
    //let mut can = _can2;

    // Split the peripheral into transmitter and receiver parts.
    block!(can.enable()).unwrap();

    #[cfg(feature = "sender")]
    {
        for _i in 0..5 {
            //delaying
            hprintln!("prepare to send").unwrap();
        }
        let frame = bxcan::Frame::new_data(bxcan::StandardId::new(0).unwrap(), [0u8]);
        block!(can.transmit(&frame)).unwrap();
        hprintln!("1 sent").unwrap();
    }

    // Echo back received packages in sequence.
    // See the `can-rtfm` example for an echo implementation that adheres to
    // correct frame ordering based on the transfer id.
    loop {
        if let Ok(frame) = block!(can.receive()) {

            #[cfg(feature = "sender")]
            hprintln!("received {:?}", frame).unwrap();

            let data = frame.data().unwrap();
            let value = data.as_ref()[0];
            let frame = bxcan::Frame::new_data(bxcan::StandardId::new(0).unwrap(), [value+1]);

            block!(can.transmit(&frame)).unwrap();
        }
    }
}
