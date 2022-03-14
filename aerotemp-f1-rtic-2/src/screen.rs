use defmt::Format;

#[derive(Copy, Clone, Format)]
pub enum Screen {
    Both,
    Single(bool),
}

impl Screen {
    pub fn next(&mut self) {
        *self = match self {
            Screen::Both => Screen::Single(true),
            Screen::Single(true) => Screen::Single(false),
            Screen::Single(false) => Screen::Both,
        }
    }
}
