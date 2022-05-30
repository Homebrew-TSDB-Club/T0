#![feature(generic_associated_types)]

pub mod array;
pub mod column;
pub mod index;
pub mod schema;
pub mod time;
pub mod util;

#[derive(Debug, PartialEq, Clone)]
pub enum ScalarType<
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Float128,
    Bool,
> {
    UInt8(UInt8),
    UInt16(UInt16),
    UInt32(UInt32),
    UInt64(UInt64),
    Int8(Int8),
    Int16(Int16),
    Int32(Int32),
    Int64(Int64),
    Float32(Float32),
    Float64(Float64),
    Float128(Float128),
    Bool(Bool),
}

#[derive(Debug, PartialEq, Clone)]
pub enum LabelType<String, IPv4, IPv6, Int, Bool> {
    String(String),
    IPv4(IPv4),
    IPv6(IPv6),
    Int(Int),
    Bool(Bool),
}
