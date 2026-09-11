use etyloom_core::{Error, Recipe, Result};

pub struct Random {
    key: [u8; 32],
    counter: u64,
}

impl Random {
    pub fn new(recipe: &Recipe, address: &str) -> Self {
        let mut hash = blake3::Hasher::new_derive_key("etyloom.addressed-randomness.v1");
        hash.update(&(recipe.seed.len() as u64).to_le_bytes());
        hash.update(recipe.seed.as_bytes());
        hash.update(&(address.len() as u64).to_le_bytes());
        hash.update(address.as_bytes());
        hash.update(
            &recipe
                .rerolls
                .get(address)
                .copied()
                .unwrap_or_default()
                .to_le_bytes(),
        );
        Self {
            key: *hash.finalize().as_bytes(),
            counter: 0,
        }
    }

    fn next(&mut self) -> u64 {
        let hash = blake3::keyed_hash(&self.key, &self.counter.to_le_bytes());
        self.counter = self.counter.wrapping_add(1);
        let mut bytes = [0; 8];
        bytes.copy_from_slice(&hash.as_bytes()[..8]);
        u64::from_le_bytes(bytes)
    }

    pub fn below(&mut self, bound: usize) -> Result<usize> {
        if bound == 0 {
            return Err(Error::Invalid("Cannot sample an empty domain".into()));
        }
        let bound = bound as u64;
        let threshold = bound.wrapping_neg() % bound;
        for _ in 0..128 {
            let value = self.next();
            if value >= threshold {
                return Ok((value % bound) as usize);
            }
        }
        Err(Error::Budget)
    }

    pub fn pick<T: Clone>(&mut self, domain: &[T]) -> Result<T> {
        domain
            .get(self.below(domain.len())?)
            .cloned()
            .ok_or(Error::Budget)
    }

    pub fn weighted<T: Clone>(&mut self, choices: &[(T, usize)]) -> Result<T> {
        let total = choices
            .iter()
            .try_fold(0usize, |n, (_, w)| n.checked_add(*w))
            .ok_or(Error::Budget)?;
        let mut roll = self.below(total)?;
        for (choice, weight) in choices {
            if roll < *weight {
                return Ok(choice.clone());
            }
            roll -= weight;
        }
        Err(Error::Budget)
    }
}
