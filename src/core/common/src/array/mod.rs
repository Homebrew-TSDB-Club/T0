mod bitmap;
mod dictionary;
pub mod primitive;

use crate::array::bitmap::BitmapRefMut;
use bitmap::{Bitmap, BitmapRef};
use dictionary::ListDictionary;
use primitive::Primitive;
use std::fmt::Debug;
use std::hash::Hash;

pub trait Array: 'static + Debug + Send + Sync {
    type Element: 'static;
    type ElementRef<'a>
    where
        Self: 'a;
    type ElementRefMut<'a>
    where
        Self: 'a;

    fn get(&self, id: usize) -> Option<Self::ElementRef<'_>>;
    fn get_unchecked(&self, id: usize) -> Self::ElementRef<'_>;
    fn get_mut(&mut self, id: usize) -> Option<Self::ElementRefMut<'_>>;
    fn push(&mut self, value: Self::ElementRef<'_>);
    fn push_zero(&mut self);
    fn len(&self) -> usize;

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub trait IndexableArray: Array {
    type ID: Hash + PartialEq;
}

#[derive(Debug)]
pub struct ConstFixedSizeListArray<P: Primitive, const SIZE: usize> {
    array: FixedSizeListArray<P>,
}

impl<P: Primitive, const SIZE: usize> Default for ConstFixedSizeListArray<P, SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Primitive, const SIZE: usize> ConstFixedSizeListArray<P, SIZE> {
    #[inline]
    pub fn new() -> Self {
        Self {
            array: FixedSizeListArray::new(SIZE),
        }
    }
}

impl<P: Primitive, const SIZE: usize> Array for ConstFixedSizeListArray<P, SIZE> {
    type Element = <FixedSizeListArray<P> as Array>::Element;
    type ElementRef<'a> = <FixedSizeListArray<P> as Array>::ElementRef<'a>;
    type ElementRefMut<'a> = <FixedSizeListArray<P> as Array>::ElementRefMut<'a>;

    fn get(&self, id: usize) -> Option<Self::ElementRef<'_>> {
        self.array.get(id)
    }

    fn get_unchecked(&self, id: usize) -> Self::ElementRef<'_> {
        self.array.get_unchecked(id)
    }

    fn get_mut(&mut self, id: usize) -> Option<Self::ElementRefMut<'_>> {
        self.array.get_mut(id)
    }

    fn push(&mut self, value: Self::ElementRef<'_>) {
        self.array.push(value)
    }

    fn push_zero(&mut self) {
        self.array.push_zero()
    }

    fn len(&self) -> usize {
        self.array.len()
    }
}

#[derive(Debug)]
pub struct FixedSizeListArray<P: Primitive> {
    data: Vec<P>,
    list_size: usize,
}

impl<P: Primitive> FixedSizeListArray<P> {
    #[inline]
    fn new(list_size: usize) -> Self {
        Self {
            list_size,
            data: Vec::new(),
        }
    }

    #[inline]
    fn slice_raw_mut(&mut self, start: usize, end: usize) -> &mut [P] {
        &mut self.data[start..end]
    }

    #[inline]
    fn slice_raw(&self, start: usize, end: usize) -> &[P] {
        &self.data[start..end]
    }
}

impl<P: Primitive> Array for FixedSizeListArray<P> {
    type Element = Vec<P>;
    type ElementRef<'a> = &'a [P];
    type ElementRefMut<'a> = &'a mut [P];

    #[inline]
    fn get(&self, id: usize) -> Option<Self::ElementRef<'_>> {
        if id * self.list_size > self.data.len() {
            None
        } else {
            Some(self.get_unchecked(id))
        }
    }

    #[inline]
    fn get_unchecked(&self, id: usize) -> Self::ElementRef<'_> {
        self.slice_raw(id * self.list_size, (id + 1) * self.list_size)
    }

    #[inline]
    fn get_mut(&mut self, id: usize) -> Option<Self::ElementRefMut<'_>> {
        if id * self.list_size > self.data.len() {
            None
        } else {
            Some(self.slice_raw_mut(id * self.list_size, (id + 1) * self.list_size))
        }
    }

    #[inline]
    fn push(&mut self, value: Self::ElementRef<'_>) {
        self.data.extend_from_slice(value);
    }

    #[inline]
    fn push_zero(&mut self) {
        let empty = vec![Default::default(); self.list_size];
        self.push(&empty);
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.len() / self.list_size
    }
}

#[derive(Debug, PartialEq)]
pub struct NullableFixedSizeList<'a, P: Primitive> {
    validity: BitmapRef<'a>,
    data: &'a [P],
}

#[derive(Debug, PartialEq)]
pub struct NullableFixedSizeListMut<'a, P: Primitive> {
    validity: BitmapRefMut<'a>,
    data: &'a mut [P],
}

#[derive(Debug)]
pub struct NullableFixedSizeListArray<P: Primitive> {
    validity: Bitmap,
    data: FixedSizeListArray<P>,
}

impl<P: Primitive> NullableFixedSizeListArray<P> {
    #[inline]
    pub fn new(list_size: usize) -> Self {
        Self {
            data: FixedSizeListArray::<P>::new(list_size),
            validity: Bitmap::new(),
        }
    }
}

impl<P: Primitive> Array for NullableFixedSizeListArray<P> {
    type Element = Vec<P>;
    type ElementRef<'a> = NullableFixedSizeList<'a, P>;
    type ElementRefMut<'a> = NullableFixedSizeListMut<'a, P>;

    #[inline]
    fn get(&self, id: usize) -> Option<Self::ElementRef<'_>> {
        if id * self.data.list_size > self.data.data.len() {
            None
        } else {
            Some(self.get_unchecked(id))
        }
    }

    #[inline]
    fn get_unchecked(&self, id: usize) -> Self::ElementRef<'_> {
        let (start, end) = (id * self.data.list_size, (id + 1) * self.data.list_size);
        NullableFixedSizeList {
            validity: self.validity.slice(start, end),
            data: self.data.slice_raw(start, end),
        }
    }

    #[inline]
    fn get_mut(&mut self, offset: usize) -> Option<Self::ElementRefMut<'_>> {
        let (start, end) = (
            offset * self.data.list_size,
            (offset + 1) * self.data.list_size,
        );
        Some(NullableFixedSizeListMut {
            validity: self.validity.slice_mut(start, end),
            data: self.data.slice_raw_mut(start, end),
        })
    }

    #[inline]
    fn push(&mut self, value: Self::ElementRef<'_>) {
        self.validity.add(value.validity);
        self.data.data.extend_from_slice(value.data);
    }

    #[inline]
    fn push_zero(&mut self) {
        for _ in 0..self.data.list_size {
            self.validity.push(false);
        }
        self.data.push_zero();
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }
}

impl<P: Primitive + Hash> IndexableArray for NullableFixedSizeListArray<P> {
    type ID = P;
}

#[derive(Debug)]
pub struct ListArray<P: Primitive> {
    offsets: Vec<usize>,
    data: Vec<P>,
}

impl<P: Primitive> ListArray<P> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<P: Primitive> Default for ListArray<P> {
    #[inline]
    fn default() -> Self {
        Self {
            offsets: vec![0],
            data: Vec::<P>::new(),
        }
    }
}

impl<P: 'static + Primitive> Array for ListArray<P> {
    type Element = Vec<P>;
    type ElementRef<'a> = &'a [P];
    type ElementRefMut<'a> = &'a mut [P];

    #[inline]
    fn get(&self, id: usize) -> Option<Self::ElementRef<'_>> {
        let offset = self.offsets.get(id)?;
        let end = self.offsets.get(id + 1)?;
        Some(&self.data[*offset..*end])
    }

    #[inline]
    fn get_unchecked(&self, id: usize) -> Self::ElementRef<'_> {
        let offset = self.offsets[id];
        let end = self.offsets[id + 1];
        &self.data[offset..end]
    }

    #[inline]
    fn get_mut(&mut self, id: usize) -> Option<Self::ElementRefMut<'_>> {
        let offset = self.offsets.get(id)?;
        let end = self.offsets.get(id + 1)?;
        Some(&mut self.data[*offset..*end])
    }

    #[inline]
    fn push(&mut self, value: Self::ElementRef<'_>) {
        let id = self.offsets.len() - 1;
        let end = self.offsets[id] + value.len();
        self.offsets.push(end);
        self.data.extend_from_slice(value);
    }

    #[inline]
    fn push_zero(&mut self) {
        self.offsets.push(self.offsets[self.offsets.len() - 1]);
    }

    fn len(&self) -> usize {
        self.offsets.len() - 1
    }
}

#[derive(Debug)]
pub struct IdListArray<A: Array> {
    values: ListDictionary<A>,
    data: Vec<usize>,
}

impl<A: Array + Default> Default for IdListArray<A>
where
    for<'a, 'b> A::ElementRef<'a>: PartialEq<A::ElementRef<'b>> + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Array + Default> IdListArray<A>
where
    for<'a, 'b> A::ElementRef<'a>: PartialEq<A::ElementRef<'b>> + Hash,
{
    pub fn new() -> Self {
        Self {
            values: ListDictionary::new(),
            data: Vec::new(),
        }
    }
}

impl<A: Array + Default> Array for IdListArray<A>
where
    for<'a, 'b> A::ElementRef<'a>: PartialEq<A::ElementRef<'b>> + Hash,
{
    type Element = Option<A::Element>;
    type ElementRef<'a> = Option<A::ElementRef<'a>>;
    type ElementRefMut<'a> = Option<A::ElementRefMut<'a>>;

    #[inline]
    fn get(&self, id: usize) -> Option<Self::ElementRef<'_>> {
        let vid = self.data.get(id)?;
        self.values.get(*vid)
    }

    #[inline]
    fn get_unchecked(&self, id: usize) -> Self::ElementRef<'_> {
        self.values.get_unchecked(self.data[id])
    }

    #[inline]
    fn get_mut(&mut self, id: usize) -> Option<Self::ElementRefMut<'_>> {
        let vid = self.data.get(id)?;
        self.values.get_mut(*vid)
    }

    #[inline]
    fn push(&mut self, value: Self::ElementRef<'_>) {
        match value {
            Some(value) => {
                self.data.push(self.values.lookup_or_insert(value));
            }
            None => {
                self.push_zero();
            }
        }
    }

    #[inline]
    fn push_zero(&mut self) {
        self.data.push(0);
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }
}

impl<A: Array + Default> IndexableArray for IdListArray<A>
where
    for<'a, 'b> A::ElementRef<'a>: PartialEq<A::ElementRef<'b>> + Hash,
{
    type ID = usize;
}

#[derive(Debug, Default)]
pub struct PrimitiveArray<P: Primitive> {
    data: Vec<P>,
}

impl<P: Primitive> PrimitiveArray<P> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<P: Primitive> Array for PrimitiveArray<P> {
    type Element = P;
    type ElementRef<'a> = &'a P;
    type ElementRefMut<'a> = &'a mut P;

    #[inline]
    fn get(&self, id: usize) -> Option<Self::ElementRef<'_>> {
        self.data.get(id)
    }

    #[inline]
    fn get_unchecked(&self, id: usize) -> Self::ElementRef<'_> {
        unsafe { self.data.get_unchecked(id) }
    }

    #[inline]
    fn get_mut(&mut self, id: usize) -> Option<Self::ElementRefMut<'_>> {
        self.data.get_mut(id)
    }

    #[inline]
    fn push(&mut self, value: Self::ElementRef<'_>) {
        self.data.push(value.clone())
    }

    #[inline]
    fn push_zero(&mut self) {
        self.data.push(P::default())
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::array::{Array, IdListArray, ListArray};

    #[test]
    fn test_id_list_array() {
        let mut array = IdListArray::<ListArray<u8>>::new();
        array.push(Some("test".as_ref()));
        assert_eq!(array.get(0), Some(Some("test".as_ref())));
        assert_eq!(array.get(1), None);
        array.push(None);
        assert_eq!(array.get(1), Some(None));
    }
}
