use super::render::WORLD_SIZE;
use std::ops::{Index, IndexMut};

pub struct World([usize; WORLD_SIZE * WORLD_SIZE * WORLD_SIZE]);

impl Default for World {
    fn default() -> Self {
        Self([0; WORLD_SIZE * WORLD_SIZE * WORLD_SIZE])
    }
}

impl Index<[usize; 3]> for World {
    type Output = usize;
    fn index(&self, idx: [usize; 3]) -> &usize {
        &self.0[(idx[0] * WORLD_SIZE + idx[1]) * WORLD_SIZE + idx[2]]
    }
}
impl IndexMut<[usize; 3]> for World {
    fn index_mut(&mut self, idx: [usize; 3]) -> &mut usize {
        &mut self.0[(idx[0] * WORLD_SIZE + idx[1]) * WORLD_SIZE + idx[2]]
    }
}

impl World {
    pub fn add_sand(&mut self, mut todo: Vec<([usize; 3], usize)>) {
        while let Some((loc, num_grains)) = todo.pop() {
            if loc.iter().all(|&x| 0 < x && x < WORLD_SIZE - 1) {
                let pile = &mut self[loc];

                *pile += num_grains;
                let num_topples = *pile / 6;

                if num_topples > 0 {
                    *pile -= 6 * num_topples;

                    #[rustfmt::skip]
					todo.extend_from_slice(&[
						{let mut loc = loc; loc[0] += 1; (loc, num_topples)},
						{let mut loc = loc; loc[0] -= 1; (loc, num_topples)},
						{let mut loc = loc; loc[1] += 1; (loc, num_topples)},
						{let mut loc = loc; loc[1] -= 1; (loc, num_topples)},
						{let mut loc = loc; loc[2] += 1; (loc, num_topples)},
						{let mut loc = loc; loc[2] -= 1; (loc, num_topples)},
					]);
                }
            }
        }
    }

    pub fn to_color_array(&self) -> Vec<u8> {
        self.0
            .iter()
            .flat_map(|n| match n {
                0 => &[0x00, 0x00, 0x00, 0x00],
                1 => &[0x00, 0x00, 0xFF, 0x00],
                2 => &[0x00, 0xC0, 0xC0, 0x00],
                3 => &[0x00, 0xFF, 0x00, 0x00],
                4 => &[0xC0, 0xC0, 0x00, 0x00],
                5 => &[0xFF, 0x00, 0x00, 0x00],
                _ => &[0xFF, 0xFF, 0xFF, 0x00],
            })
            .copied()
            .collect()
    }
}
