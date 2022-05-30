const BIT_MASK: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
const UNSET_BIT_MASK: [u8; 8] = [
    255 - 1,
    255 - 2,
    255 - 4,
    255 - 8,
    255 - 16,
    255 - 32,
    255 - 64,
    255 - 128,
];

#[inline]
fn set_bit(byte: u8, i: usize, value: bool) -> u8 {
    if value {
        byte | BIT_MASK[i]
    } else {
        byte & UNSET_BIT_MASK[i]
    }
}

#[inline]
fn is_set(byte: u8, i: usize) -> bool {
    (byte & BIT_MASK[i]) != 0
}

#[inline]
fn get_bit(data: &[u8], i: usize) -> bool {
    is_set(data[i / 8], i % 8)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct BitmapRefMut<'a> {
    buffer: &'a mut [u8],
    length: usize,
}

impl<'a> BitmapRefMut<'a> {
    #[inline]
    pub(crate) fn insert(&mut self, offset: usize, value: bool) {
        let byte = &mut self.buffer[offset / 8];
        *byte = set_bit(*byte, offset % 8, value);
    }

    #[inline]
    pub(crate) fn get_bit(&self, offset: usize) -> bool {
        get_bit(self.buffer, offset)
    }
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct BitmapRef<'a> {
    buffer: &'a [u8],
    length: usize,
}

impl<'a> BitmapRef<'a> {
    #[inline]
    pub(crate) fn get_bit(&self, offset: usize) -> bool {
        get_bit(self.buffer, offset)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Bitmap {
    buffer: Vec<u8>,
    length: usize,
}

impl Bitmap {
    #[inline]
    pub(crate) fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub(crate) fn push(&mut self, value: bool) {
        if self.length % 8 == 0 {
            self.buffer.push(0);
        }
        let byte = self.buffer.as_mut_slice().last_mut().unwrap();
        *byte = set_bit(*byte, self.length % 8, value);
        self.length += 1;
    }

    #[inline]
    pub(crate) fn slice(&self, start: usize, end: usize) -> BitmapRef<'_> {
        BitmapRef {
            buffer: &self.buffer[(start / 8)..((end / 8) + 1)],
            length: end - start,
        }
    }

    #[inline]
    pub(crate) fn slice_mut(&mut self, start: usize, end: usize) -> BitmapRefMut<'_> {
        BitmapRefMut {
            buffer: &mut self.buffer[(start / 8)..((end / 8) + 1)],
            length: end - start,
        }
    }

    #[inline]
    pub(crate) fn add(&mut self, another: BitmapRef<'_>) {
        for byte in another.buffer {
            for i in 0..8 {
                self.push(is_set(*byte, i))
            }
        }
    }
}
