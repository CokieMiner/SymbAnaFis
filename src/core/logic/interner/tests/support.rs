use core::{hash::Hash, ptr};

use alloc::vec::Vec;

use super::super::Interner;

impl<Value: ?Sized> Interner<Value> {
    pub fn index_capacity(&self) -> usize {
        self.shards
            .iter()
            .map(|shard| shard.lock().capacity())
            .sum()
    }

    pub fn contains_weak(&self, value: &Value, pointer: *const Value) -> bool
    where
        Value: Hash + Eq,
    {
        self.shard(Self::fingerprint(value))
            .lock()
            .get(&Self::fingerprint(value))
            .is_some_and(|bucket| {
                bucket
                    .iter()
                    .any(|entry| ptr::addr_eq(entry.as_ptr(), pointer))
            })
    }

    pub fn entry_count(&self) -> usize {
        self.shards
            .iter()
            .map(|shard| shard.lock().values().map(Vec::len).sum::<usize>())
            .sum()
    }

    pub fn bucket_capacity(&self, value: &Value) -> usize
    where
        Value: Hash + Eq,
    {
        self.shard(Self::fingerprint(value))
            .lock()
            .get(&Self::fingerprint(value))
            .map_or(0, Vec::capacity)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Collision(pub u8);

impl Hash for Collision {
    fn hash<HasherType: core::hash::Hasher>(&self, state: &mut HasherType) {
        0_u8.hash(state);
    }
}
