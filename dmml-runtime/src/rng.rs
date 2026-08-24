//! A tiny deterministic PRNG (xorshift64*) so a frontier point generates
//! the same content from the same seed. No external dependency needed for
//! a prototype this size, and it keeps the engine crate portable to wasm
//! without pulling in `getrandom` and its platform shims.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn gen_range(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    pub fn chance(&mut self, pct_out_of_100: u8) -> bool {
        self.gen_range(100) < pct_out_of_100 as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.gen_range(items.len())]
    }

    /// A value in `[0.0, 1.0)`, at 1/10000 granularity -- enough resolution
    /// for a graded ground fact like dampness or decay without pulling in
    /// a real float-uniform algorithm for a prototype this size.
    pub fn gen_float(&mut self) -> f32 {
        self.gen_range(10_000) as f32 / 10_000.0
    }
}

/// Deterministic seed for a frontier point, so exploring the same
/// coordinate from the same origin always proposes the same content —
/// stability of the proposal, not just of the commit.
pub fn seed_for(world_seed: u64, origin_room: u32, dir_index: u8) -> u64 {
    world_seed
        .wrapping_mul(1_000_003)
        .wrapping_add(origin_room as u64 * 31 + dir_index as u64)
}
