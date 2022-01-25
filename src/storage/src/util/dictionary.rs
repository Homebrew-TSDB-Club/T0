use super::string::StringArray;
use ahash::RandomState;
use hashbrown::hash_map::RawEntryMut;
use hashbrown::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Debug, Default)]
pub struct StringDictionary {
    hash_state: RandomState,
    dedup: HashMap<usize, (), ()>,
    data: StringArray,
}

impl StringDictionary {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn lookup_or_insert(&mut self, value: &str) -> usize {
        let hash = Self::hash_str(&self.hash_state, value);
        let entry = self
            .dedup
            .raw_entry_mut()
            .from_hash(hash, |key| value == self.data.get(*key).unwrap());
        let storage = &mut self.data;

        return match entry {
            RawEntryMut::Occupied(entry) => *entry.into_key(),
            RawEntryMut::Vacant(entry) => {
                let index = storage.append(value);
                *entry
                    .insert_with_hasher(hash, index, (), |index| {
                        let string = self.data.get(*index).unwrap();
                        Self::hash_str(&self.hash_state, string)
                    })
                    .0
            }
        } + 1;
    }

    pub fn lookup(&self, value: &str) -> Option<usize> {
        return self
            .dedup
            .raw_entry()
            .from_hash(Self::hash_str(&self.hash_state, value), |key| {
                value == self.data.get(*key).unwrap()
            })
            .map(|(&symbol, &())| symbol + 1);
    }

    pub fn get(&self, id: usize) -> Option<&str> {
        if id == 0 {
            None
        } else {
            self.data.get(id - 1)
        }
    }

    fn hash_str(state: &RandomState, value: &str) -> u64 {
        let mut hasher = state.build_hasher();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::StringDictionary;

    #[test]
    fn test_storage() {
        let mut dict = StringDictionary::new();
        let id = dict.lookup_or_insert("hello, world");
        let id2 = dict.lookup_or_insert("hello, world");
        assert_eq!(id, id2);
        let id3 = dict.lookup_or_insert("hello world");
        assert_ne!(id, id3);
        let v1 = dict.get(id);
        let v2 = dict.get(id2);
        assert_eq!(v1, v2);
    }
}
