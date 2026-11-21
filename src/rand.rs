use crate::nums::MathRandom;
use std::cell::RefCell;

pub trait RandomValue {
    fn from_random(value: u128) -> Self;
}

macro_rules! impl_random_value {
        ($($ty:ty),*) => {
            $(impl RandomValue for $ty { fn from_random(value: u128) -> Self { value as $ty }})*
        };
    }

impl_random_value!(u8, u16, u32, u64, u128, usize);
impl_random_value!(i8, i16, i32, i64, i128, isize);

thread_local! {
    static MATH_RANDOM: RefCell<MathRandom> =
        RefCell::new(MathRandom::initialize(Some(::rand::random())));
}

pub fn random<T>() -> T
where
    T: RandomValue,
{
    MATH_RANDOM.with_borrow_mut(|mr| {
        let lhs = mr.random_u64() as u128;
        let rhs = mr.random_u64() as u128;
        let n = (lhs << 64) | rhs;
        T::from_random(n)
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nums_rand_foo() {
        for _ in 0..10 {
            let m: u128 = random();
            println!("{m}");
        }
    }
}
