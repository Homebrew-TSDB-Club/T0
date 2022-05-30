use croaring::Bitmap;
use hashbrown::HashMap;
use std::hash::Hash;

#[derive(Debug, PartialEq, Clone)]
pub enum IndexType<Key> {
    Inverted(Key),
}

pub type IndexImpl<K> = IndexType<HashMap<K, Bitmap>>;

impl<K> IndexImpl<K>
where
    K: Hash + Eq,
{
    pub fn new(data_type: IndexType<()>) -> Self {
        match data_type {
            IndexType::Inverted(_) => IndexImpl::Inverted(HashMap::new()),
        }
    }
}
