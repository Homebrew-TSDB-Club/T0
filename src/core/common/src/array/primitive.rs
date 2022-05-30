use std::fmt::{Debug, Display};

pub trait Primitive:
    'static + Send + Sync + Sized + Debug + Display + PartialEq + Default + Clone
{
}

macro_rules! native_type {
    ($type:ty) => {
        impl Primitive for $type {}
    };
}

native_type!(bool);
native_type!(u8);
native_type!(u16);
native_type!(u32);
native_type!(u64);
native_type!(i8);
native_type!(i16);
native_type!(i32);
native_type!(i64);
native_type!(f32);
native_type!(f64);
native_type!(i128);
