use std::num::Wrapping;

pub mod dictionary;
pub mod string;

#[inline]
pub fn hash_combine(left: u64, right: u64) -> u64 {
    let left = Wrapping(left);
    let right = Wrapping(right);
    (left ^ (right + Wrapping(0x9e3779b9) + (left << 6) + (left >> 2))).0
}

pub struct HashReduce {
    hash: Wrapping<u64>,
}

impl HashReduce {
    #[inline]
    pub fn new(v: u64) -> Self {
        Self { hash: Wrapping(v) }
    }

    #[inline]
    pub fn add(&mut self, v: u64) {
        self.hash = self.hash * Wrapping(31) + Wrapping(v);
    }

    #[inline]
    pub fn finish(self) -> u64 {
        self.hash.0
    }
}

const JUMP: u64 = 1 << 31;

pub fn jump_consistent_hash(key: u64, buckets: usize) -> u32 {
    let mut k = key;
    let (mut b, mut j) = (-1, 0);
    while j < buckets as i64 {
        b = j;
        k = k.wrapping_mul(2862933555777941757) + 1;
        j = ((b + 1) as f64 * (JUMP as f64 / ((k >> 33) + 1) as f64)) as i64;
    }
    b as u32
}

#[cfg(test)]
mod tests {
    use crate::hash_combine;

    #[test]
    fn test_hash_combine() {
        let left = fxhash::hash64("hello");
        let right = fxhash::hash64("world");
        hash_combine(left, right);
    }
}
