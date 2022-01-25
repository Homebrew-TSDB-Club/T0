use hashbrown::HashMap;
use std::borrow::Borrow;
use std::hash::Hash;
use std::ops::Index;
use std::slice::{Iter, IterMut};
use std::vec::IntoIter;

#[derive(Debug)]
pub struct IndexMap<K, V> {
    value: Vec<V>,
    index: HashMap<K, usize>,
}

impl<K, V> Default for IndexMap<K, V> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            index: Default::default(),
        }
    }
}

impl<K, V> IndexMap<K, V>
where
    K: Hash + Eq,
{
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn insert(&mut self, k: K, v: V) {
        self.value.push(v);
        self.index.insert(k, self.value.len() - 1);
    }

    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        Some(&self.value[*self.index.get(k)?])
    }

    #[inline]
    pub fn get_id<Q: ?Sized>(&self, k: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.index.get(k).cloned()
    }

    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        Some(&mut self.value[*self.index.get(k)?])
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, V> {
        self.value.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        self.value.iter_mut()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.value.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

impl<K, V> Index<usize> for IndexMap<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.value[index]
    }
}

impl<K, V> IntoIterator for IndexMap<K, V> {
    type Item = V;
    type IntoIter = IntoIter<V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}
