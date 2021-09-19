use crate::Scale;
use e_ring::Ring;

pub struct TempsValues([[Ring<i16, 128>; 3]; 2]);
impl TempsValues {
    pub fn series(&self, t: usize, scale: Scale) -> &Ring<i16, 128> {
        &self.0[t][scale as usize]
    }

    pub fn last(&self, t: usize) -> Option<i16> {
        self.0[t][0].iter().last()
    }

    pub fn store(&mut self, value: i16, seconds: u32, t: usize) {
        let current = &mut self.0[t];
        current[0].append(value);
        for (i, interval) in [Scale::TenSeconds.seconds(), Scale::Minute.seconds()]
            .iter()
            .enumerate()
        {
            if seconds % interval == 0 {
                let len = current[0].len();
                let value = current[0]
                    .iter()
                    .skip(len - *interval as usize)
                    .map(|e| e as f32)
                    .sum::<f32>()
                    / *interval as f32;
                current[i + 1].append(value as i16);
            }
        }
    }
}

impl Default for TempsValues {
    fn default() -> Self {
        TempsValues([
            [Ring::new(), Ring::new(), Ring::new()],
            [Ring::new(), Ring::new(), Ring::new()],
        ])
    }
}
