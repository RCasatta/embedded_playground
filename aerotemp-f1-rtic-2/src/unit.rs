use core::fmt;

#[derive(Copy, Clone, Debug)]
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

impl Default for Unit {
    fn default() -> Self {
        Unit::Celsius
    }
}

impl Unit {
    pub fn next(&mut self) -> Self {
        *self = match self {
            Unit::Celsius => Unit::Fahrenheit,
            Unit::Fahrenheit => Unit::Celsius,
        };
        *self
    }
}
