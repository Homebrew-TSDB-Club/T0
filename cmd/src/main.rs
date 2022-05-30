#![feature(generic_associated_types)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

trait Array {
    type ElementRef<'a>;

    fn slice(&self) -> Self::ElementRef<'_>;
}

struct PrimitiveArray<P> {
    data: Vec<P>,
}

#[derive(Debug, Default)]
pub(crate) struct BitmapRef<'a> {
    buffer: &'a [u8],
    length: usize,
}

impl<P> Array for PrimitiveArray<P> {
    type ElementRef<'a> = BitmapRef<'a>;

    fn slice(&self) -> Self::ElementRef<'_> {
        todo!()
    }
}
