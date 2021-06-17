use dht_sensor::{dht22, DhtReading};
use e_ring::Ring;
use embedded_hal::adc::OneShot;
use stm32f1xx_hal::adc::Adc;
use stm32f1xx_hal::delay::Delay;
use stm32f1xx_hal::gpio::gpioa::{PA6, PA7};
use stm32f1xx_hal::gpio::gpiob::{PB0, PB1, PB5};
use stm32f1xx_hal::gpio::{Analog, Floating, Input, OpenDrain, Output};
use stm32f1xx_hal::pac::{ADC1, ADC2};
use stm32f1xx_hal::time::Instant;

// seconds in a minute, seconds in an hour
const INTERVALS: [u32; 2] = [60, 3600];

pub struct Battery {
    pub adc: Adc<ADC2>,
    pub channel: PB1<Analog>,
    pub values: [Ring<i16, 128>; 3],
}

impl Battery {
    pub fn read_and_store(&mut self, seconds: u32) {
        let value: u16 = self.adc.read(&mut self.channel).unwrap();
        self.values[0].append(value as i16);
        for (i, interval) in INTERVALS.iter().enumerate() {
            if seconds % interval == 0 {
                let len = self.values[i].len();
                let value = self.values[i]
                    .iter()
                    .skip(len - 60)
                    .map(|e| e as f32)
                    .sum::<f32>()
                    / 60.0;
                self.values[i + 1].append(value as i16);
            }
        }
    }
}

pub struct Moisture {
    pub adc: Adc<ADC1>,
    pub channel: PB0<Analog>,
    pub values: [Ring<i16, 128>; 3],
}

impl Moisture {
    pub fn read_and_store(&mut self, seconds: u32) {
        let value: u16 = self.adc.read(&mut self.channel).unwrap();
        self.values[0].append(value as i16);
        for (i, interval) in INTERVALS.iter().enumerate() {
            if seconds % interval == 0 {
                let len = self.values[i].len();
                let value = self.values[i]
                    .iter()
                    .skip(len - 60)
                    .map(|e| e as f32)
                    .sum::<f32>()
                    / 60.0;
                self.values[i + 1].append(value as i16);
            }
        }
    }
}

pub struct TempHumidity {
    pub delay: Delay,
    pub dht_pin: PB5<Output<OpenDrain>>,
    pub temp_values: [Ring<i16, 128>; 3],
    pub humidity_values: [Ring<i16, 128>; 3],
}

impl TempHumidity {
    pub fn read_and_store(&mut self, seconds: u32) {
        let dht22::Reading {
            temperature,
            relative_humidity,
        } = dht22::Reading::read(&mut self.delay, &mut self.dht_pin).unwrap();
        self.temp_values[0].append((temperature * 10.0) as i16);
        self.humidity_values[0].append((relative_humidity * 10.0) as i16);

        for (i, interval) in INTERVALS.iter().enumerate() {
            if seconds % interval == 0 {
                let len = self.temp_values[i].len();
                let value = self.temp_values[i]
                    .iter()
                    .skip(len - 60)
                    .map(|e| e as f32)
                    .sum::<f32>()
                    / 60.0;
                self.temp_values[i + 1].append(value as i16);

                let len = self.humidity_values[i].len();
                let value = self.humidity_values[i]
                    .iter()
                    .skip(len - 60)
                    .map(|e| e as f32)
                    .sum::<f32>()
                    / 60.0;
                self.humidity_values[i + 1].append(value as i16);
            }
        }
    }
}

macro_rules! impl_button {
    ( $button_struct:ident, $pin_type:ty ) => {
        pub struct $button_struct {
            pub pin: $pin_type,
            pub last: Instant,
        }
    };
}

impl_button!(ButtonA, PA7<Input<Floating>>);
impl_button!(ButtonB, PA6<Input<Floating>>);
