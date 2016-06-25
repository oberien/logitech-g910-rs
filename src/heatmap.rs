use std::collections::HashMap;
use color::{Color, KeyColor};
use keys::Key;

const GRADIENT: [Color; 6] = [
    Color { red: 0, green: 0, blue: 0 },
    Color { red: 0, green: 0, blue: 255 },
    Color { red: 0, green: 255, blue: 255 },
    Color { red: 0, green: 255, blue: 0 },
    Color { red: 255, green: 255, blue: 0 },
    Color { red: 255, green: 0, blue: 0 },
];

pub struct Heatmap {
    data: HashMap<Key, u64>,
}

impl Heatmap {
    pub fn new() -> Heatmap {
        let mut data = HashMap::new();
        for key in Key::values() {
            data.insert(key, 0);
        }
        Heatmap {
            data: data,
        }
    }

    pub fn increment(&mut self, key: &Key) {
        match self.data.get_mut(&key) {
            Some(mut count) => *count += 1,
            None => unreachable!()
        }
    }

    /// Six Color Gradient:
    /// (1) black, (2) blue, (3) cyan, (4) green, (5) yellow, (6) red
    /// (http://www.andrewnoske.com/wiki/Code_-_heatmaps_and_color_gradients)
    pub fn colors<'a>(&'a self) -> Vec<KeyColor> {
        let max = match self.data.iter().map(|(_, v)| v).max() {
            Some(max) => max,
            None => unreachable!()
        };
        self.data.iter().map(|(k, v)| {
            let color;
            let v_scaled = *v as f64 / *max as f64;
            if v_scaled <= 0f64 {
                color = GRADIENT[0];
            } else if v_scaled >= 1f64 {
                color = GRADIENT[GRADIENT.len()-1];
            } else {
                let idx = (v_scaled * (GRADIENT.len()-1) as f64) as usize;
                let diff = (v_scaled * (GRADIENT.len()-1) as f64) - idx as f64;
                color = Color::new(
                    ((((GRADIENT[idx+1].red as i16 - GRADIENT[idx].red as i16) as f64) * diff) as i16 + GRADIENT[idx].red as i16) as u8,
                    ((((GRADIENT[idx+1].green as i16 - GRADIENT[idx].green as i16) as f64) * diff) as i16 + GRADIENT[idx].green as i16) as u8,
                    ((((GRADIENT[idx+1].blue as i16 - GRADIENT[idx].blue as i16) as f64) * diff) as i16 + GRADIENT[idx].blue as i16) as u8,
                );
            }
            KeyColor::new(k.clone(), color)
        }).collect()
    }
}
