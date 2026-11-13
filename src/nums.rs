// Although the logic is the same, this is a vastly simplified port of the Math.random
// implementation in V8 15.4.80.20, which is used in Chrome 154.0.8037.98 (Official Build) (x86_64).
#![allow(dead_code)]

pub mod math_random {
    use crate::nums::utils::RandomNumberGenerator;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub trait RandomProvider {
        fn random(&mut self) -> u64;
    }

    #[derive(Default, Debug)]
    pub struct State {
        pub s0: u64,
        pub s1: u64,
    }

    #[derive(Debug)]
    pub struct MathRandom {
        state: State,
        cache_index: usize,
        cache: [u64; Self::K_CACHE_SIZE as usize],
    }

    impl RandomProvider for MathRandom {
        fn random(&mut self) -> u64 {
            self.random_u64()
        }
    }

    impl MathRandom {
        const K_INT64_SIZE: i64 = std::mem::size_of::<i64>() as i64;
        const K_CACHE_SIZE: i64 = 64;
        const K_STATE_SIZE: i64 = 2 * Self::K_INT64_SIZE;

        pub fn initialize(seed: Option<u64>) -> Self {
            let seed = match seed {
                Some(s) => s,
                None => SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            let mut this = Self {
                state: State::default(),
                cache_index: 0,
                cache: [0; Self::K_CACHE_SIZE as usize],
            };

            this.state.s0 = RandomNumberGenerator::murmurhash3(seed);
            this.state.s1 = RandomNumberGenerator::murmurhash3(!seed);
            this.refill_cache();

            this
        }

        pub fn random_double(&mut self) -> f64 {
            if self.cache_index == 0 {
                self.refill_cache();
            }
            self.cache_index -= 1;
            RandomNumberGenerator::to_double(self.cache[self.cache_index])
        }

        pub fn random_u64(&mut self) -> u64 {
            if self.cache_index == 0 {
                self.refill_cache();
            }
            self.cache_index -= 1;
            self.cache[self.cache_index]
        }

        fn refill_cache(&mut self) {
            for i in (0usize..=(Self::K_CACHE_SIZE - 1) as usize).rev() {
                let random =
                    RandomNumberGenerator::xorshift128(&mut self.state.s0, &mut self.state.s1);
                self.cache[i] = random;
            }
            self.cache_index = Self::K_CACHE_SIZE as usize;
        }

        fn reverse_to_double(dbl: f64) -> u64 {
            ((dbl * (1_u64 << 53) as f64) as u64) << 11
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn matches_chrome() {
            let mut mr = MathRandom::initialize(Some(1337));
            // During startup Chrome seems to call Math.random 8 times, so we also need to call it 8 times.
            for _ in 0..8 {
                _ = mr.random_double();
            }

            let mut rands = [0.0; 10];
            (0..10).for_each(|i| rands[i] = mr.random_double());

            let expected = [
                0.1623809348128048,
                0.803143096276664,
                0.6202810541608373,
                0.4509504712894983,
                0.44412613159342396,
                0.7323221103230801,
                0.7364364137826004,
                0.35209346258118945,
                0.9414014356908873,
                0.6226657071438221,
            ];

            assert_eq!(expected, rands);
        }
    }
}

pub mod utils {
    #[derive(Default, Debug)]
    pub struct RandomNumberGenerator {
        state0: u64,
        state1: u64,
    }

    impl RandomNumberGenerator {
        pub fn new(seed: u64) -> Self {
            let mut this = Self::default();
            this.set_seed(seed);
            this
        }

        pub fn set_seed(&mut self, seed: u64) {
            let initial_seed = seed;
            self.state0 = Self::murmurhash3(initial_seed);
            self.state1 = Self::murmurhash3(!initial_seed);
        }

        pub fn next_u64(&mut self) -> u64 {
            Self::xorshift128(&mut self.state0, &mut self.state1)
        }

        pub fn next_double(&mut self) -> f64 {
            Self::to_double(Self::xorshift128(&mut self.state0, &mut self.state1))
        }

        pub(crate) fn state0(&self) -> u64 {
            self.state0
        }

        pub(crate) fn state1(&self) -> u64 {
            self.state1
        }

        pub fn to_double(random: u64) -> f64 {
            // 11 bits of state
            let random_0_to_2_53: f64 = (random >> 11) as f64;
            // Map this to [0, 1) by division with 2**53
            let k2_53: f64 = (1_u64 << 53) as f64;
            random_0_to_2_53 / k2_53
        }

        pub fn xorshift128(state0: &mut u64, state1: &mut u64) -> u64 {
            let mut s1 = *state0;
            let s0 = *state1;
            *state0 = s0;
            s1 ^= s1 << 23;
            s1 ^= s1 >> 17;
            s1 ^= s0;
            s1 ^= s0 >> 26;
            *state1 = s1;

            s0.wrapping_add(s1)
        }

        pub fn murmurhash3(mut h: u64) -> u64 {
            h ^= h >> 33;
            h = h.wrapping_mul(0xFF51AFD7ED558CCDu64);
            h ^= h >> 33;
            h = h.wrapping_mul(0xC4CEB9FE1A85EC53u64);
            h ^= h >> 33;
            h
        }
    }
}
