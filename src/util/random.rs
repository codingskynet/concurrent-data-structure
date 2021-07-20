use rand::{distributions::Alphanumeric, prelude::ThreadRng, Rng};
pub trait Random {
    fn gen(rng: &mut ThreadRng) -> Self;
}

const RANDOM_STRING_MIN: usize = 0;
const RANDOM_STRING_MAX: usize = 10;

impl Random for String {
    // get random string whose length is in [RANDOM_STRING_MIN, RANDOM_STRING_MAX)
    fn gen(rng: &mut ThreadRng) -> Self {
        let length: usize = rng.gen_range(RANDOM_STRING_MIN..RANDOM_STRING_MAX);

        rng.sample_iter(&Alphanumeric)
            .map(char::from)
            .take(length)
            .collect()
    }
}

impl Random for u64 {
    fn gen(rng: &mut ThreadRng) -> Self {
        rng.gen()
    }
}
