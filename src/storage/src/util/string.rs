use std::str::from_utf8;
use std::usize;

#[derive(Debug)]
pub struct StringArray {
    offsets: Vec<usize>,
    data: Vec<u8>,
}

impl Default for StringArray {
    fn default() -> Self {
        Self {
            offsets: vec![0],
            data: Vec::<u8>::new(),
        }
    }
}

impl StringArray {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn append(&mut self, value: &str) -> usize {
        let id = self.offsets.len() - 1;
        let end = self.offsets[id] + value.len();
        self.offsets.push(end);
        self.data.extend_from_slice(value.as_bytes());
        id
    }

    #[inline]
    pub fn get(&self, id: usize) -> Option<&str> {
        let offset = self.offsets.get(id)?;
        let end = self.offsets.get(id + 1)?;
        return Some(from_utf8(&self.data[*offset..*end]).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::StringArray;

    #[test]
    fn test_storage() {
        let mut array = StringArray::new();
        let id1 = array.append("hello, world");
        let id2 = array.append("one");
        assert_eq!(array.get(id1).unwrap(), "hello, world");
        assert_eq!(array.get(id2).unwrap(), "one");
        let empty_array = StringArray::new();
        assert_eq!(empty_array.get(0), None);
    }
}
