#[derive(Debug)]
pub enum OnScreen {
    Temperature,
    Humidity,
    Battery,
    Moisture,
}

impl OnScreen {
    pub fn next(&mut self) {
        use OnScreen::*;
        let new = match self {
            Temperature => Humidity,
            Humidity => Battery,
            Battery => Moisture,
            Moisture => Temperature,
        };
        *self = new;
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TimeSlice {
    Second = 0,
    Minute = 1,
    Hour = 2,
}

impl TimeSlice {
    pub fn next(&mut self) {
        use TimeSlice::*;
        let new = match self {
            Second => Minute,
            Minute => Hour,
            Hour => Second,
        };
        *self = new;
    }
}
