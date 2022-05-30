use super::Array;
use ahash::RandomState;
use hashbrown::hash_map::RawEntryMut;
use hashbrown::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

#[inline]
fn hash_with_state<H: Hash>(state: &RandomState, value: &H) -> u64 {
    let mut hasher = state.build_hasher();
    value.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
pub struct ListDictionary<A: Array> {
    hash_state: RandomState,
    dedup: HashMap<usize, (), ()>,
    data: A,
}

impl<A: Array + Default> ListDictionary<A>
where
    for<'a, 'b> A::ElementRef<'a>: PartialEq<A::ElementRef<'b>> + Hash,
{
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            hash_state: RandomState::new(),
            dedup: Default::default(),
            data: Default::default(),
        }
    }

    #[inline]
    pub(crate) fn lookup_or_insert(&mut self, value: A::ElementRef<'_>) -> usize {
        let hash = hash_with_state(&self.hash_state, &value);
        let entry = self
            .dedup
            .raw_entry_mut()
            .from_hash(hash, |key| value == self.data.get_unchecked(*key));

        return match entry {
            RawEntryMut::Occupied(entry) => *entry.into_key(),
            RawEntryMut::Vacant(entry) => {
                self.data.push(value);
                *entry
                    .insert_with_hasher(hash, self.data.len() - 1, (), |index| {
                        let list = self.data.get(*index).unwrap();
                        hash_with_state(&self.hash_state, &list)
                    })
                    .0
            }
        } + 1;
    }

    #[inline]
    pub(crate) fn lookup(&self, value: A::ElementRef<'_>) -> Option<usize> {
        return self
            .dedup
            .raw_entry()
            .from_hash(hash_with_state(&self.hash_state, &value), |key| {
                self.data.get_unchecked(*key) == value
            })
            .map(|(&symbol, &())| symbol + 1);
    }

    #[inline]
    pub(crate) fn get(&self, id: usize) -> Option<Option<A::ElementRef<'_>>> {
        if id == 0 {
            Some(None)
        } else {
            Some(self.data.get(id - 1))
        }
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, id: usize) -> Option<Option<A::ElementRefMut<'_>>> {
        if id == 0 {
            Some(None)
        } else {
            Some(self.data.get_mut(id - 1))
        }
    }

    #[inline]
    pub(crate) fn get_unchecked(&self, id: usize) -> Option<A::ElementRef<'_>> {
        if id == 0 {
            None
        } else {
            self.data.get(id - 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ListDictionary;
    use crate::array::ListArray;

    #[test]
    fn test_storage() {
        let mut dict = ListDictionary::<ListArray<u8>>::new();
        let id = dict.lookup_or_insert("hello, world".as_ref());
        let id2 = dict.lookup_or_insert("hello, world".as_ref());
        assert_eq!(id, id2);
        let id3 = dict.lookup_or_insert("hello world".as_ref());
        assert_ne!(id, id3);
        let v1 = dict.get(id);
        let v2 = dict.get(id2);
        assert_eq!(v1, v2);
    }
}
