use super::render::WORLD_SIZE;
use std::ops::{Index, IndexMut};

pub struct World(pub [u8; WORLD_SIZE * WORLD_SIZE * WORLD_SIZE]);

impl Default for World {
    fn default() -> Self {
        Self([0; WORLD_SIZE * WORLD_SIZE * WORLD_SIZE])
    }
}

impl Index<[usize; 3]> for World {
    type Output = u8;
    fn index(&self, idx: [usize; 3]) -> &u8 {
        &self.0[(idx[0] * WORLD_SIZE + idx[1]) * WORLD_SIZE + idx[2]]
    }
}
impl IndexMut<[usize; 3]> for World {
    fn index_mut(&mut self, idx: [usize; 3]) -> &mut u8 {
        &mut self.0[(idx[0] * WORLD_SIZE + idx[1]) * WORLD_SIZE + idx[2]]
    }
}

impl World {
    pub fn add_sand(&mut self, mut todo: Vec<([usize; 3], usize)>) {
        while let Some((loc, num_grains)) = todo.pop() {
            if loc.iter().all(|&x| 0 < x && x < WORLD_SIZE - 1) {
                let pile = &mut self[loc];

                let pile_grains = *pile as usize + num_grains;

                let num_topples = pile_grains / 6;
                *pile = (pile_grains % 6) as u8;

                if num_topples > 0 {
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

    pub fn to_color_array(&self) -> &[u8] {
        &self.0
    }
}
