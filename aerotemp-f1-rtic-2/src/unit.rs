use core::fmt;

use defmt::Format;

#[derive(Copy, Clone, Format)]
pub enum Unit {
    Celsius,
    Fahrenheit,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::Celsius => write!(f, "°C"),
            Unit::Fahrenheit => write!(f, "°F"),
        }
    }
}

impl Unit {
    pub fn next(&mut self) {
        *self = match self {
            Unit::Celsius => Unit::Fahrenheit,
            Unit::Fahrenheit => Unit::Celsius,
        }
    }
}
