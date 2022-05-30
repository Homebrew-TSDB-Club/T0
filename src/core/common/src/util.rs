use hashbrown::HashMap;
use std::borrow::Borrow;
use std::hash::Hash;
use std::num::Wrapping;
use std::ops::Index;
use std::slice::{Iter, IterMut};
use std::vec::IntoIter;

pub const NULL_HASH: u64 = 0xf90ec6875afe5257;

#[inline]
pub fn hash_combine(left: u64, right: u64) -> u64 {
    let left = Wrapping(left);
    let right = Wrapping(right);
    (left ^ (right + Wrapping(0x9e3779b9) + (left << 6) + (left >> 2))).0
}

#[derive(Debug)]
pub struct OrderedMap<K, V> {
    values: Vec<V>,
    index: HashMap<K, usize>,
}

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self {
            values: Default::default(),
            index: Default::default(),
        }
    }
}

impl<K, V> OrderedMap<K, V>
where
    K: Hash + Eq,
{
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn insert(&mut self, k: K, v: V) {
        self.values.push(v);
        self.index.insert(k, self.values.len() - 1);
    }

    #[inline]
    pub fn lookup<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        Some(&self.values[*self.index.get(k)?])
    }

    #[inline]
    pub fn lookup_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        Some(&mut self.values[*self.index.get(k)?])
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, V> {
        self.values.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        self.values.iter_mut()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[inline]
    pub fn first(&self) -> Option<&V> {
        self.values.first()
    }

    #[inline]
    pub fn get(&self, id: usize) -> Option<&V> {
        self.values.get(id)
    }
}

impl<K, V> Index<usize> for OrderedMap<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl<K, V> IntoIterator for OrderedMap<K, V> {
    type Item = V;
    type IntoIter = IntoIter<V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<K, V> FromIterator<(K, V)> for OrderedMap<K, V>
where
    K: Hash + Eq,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (k, v) in iter {
            map.insert(k, v)
        }
        map
    }
}
