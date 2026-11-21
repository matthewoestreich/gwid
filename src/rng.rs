use rand::{RngExt, rngs::ThreadRng};

use crate::nums::MathRandom;

pub trait RandomProvider {
    fn random_u128(&mut self) -> u128;
}

impl RandomProvider for MathRandom {
    fn random_u128(&mut self) -> u128 {
        ((self.random_u64() << 64) | self.random_u64()) as u128
    }
}

impl RandomProvider for ThreadRng {
    fn random_u128(&mut self) -> u128 {
        self.random()
    }
}
