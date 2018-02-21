use std::cmp;
use std::marker::PhantomData;
use std::io::Read as IoRead;
use std;

use serde::de::{Deserializer as SerdeDeserializer, DeserializeSeed, Visitor, Deserialize,
    SeqAccess, MapAccess, EnumAccess, VariantAccess, IntoDeserializer};

mod read;

use error::*;
use self::read::*;


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
// TODO put Type into error for expecting
#[allow(dead_code)] // Most types are never 'constructed', but they are through transmute
pub enum Type {
    Uint,
    Int,
    Misc,
    Variant,
    Seq,
    Bytes,
    Map,
    Reserved,
    Any, // Should never be constructed except for for error debugging
    Char, // Should never be constructed except for for error debugging
}

#[inline]
// Interpret the first three bits of the byte into an enum
fn ty(byte: u8) -> Type {
    *unsafe { &mut *(&mut (byte >> 5) as *mut u8 as *mut Type) }
}

#[inline]
fn val(byte: u8) -> u8 {
    byte & 0b00011111
}


pub fn from_reader<'de, R: IoRead + 'de, T>(r: R) -> Result<T>
where
    T: Deserialize<'de>
{
    let mut deserializer = Deserializer::from_reader(r);
    let t = T::deserialize(&mut deserializer)?;

    if deserializer.input.finished() {
        Ok(t)
    } else {
        Err(Error::TrailingBytes)
    }
}

pub fn from_slice<'de, S: AsRef<[u8]> + 'de, T>(bytes: &'de S) -> Result<T>
where
    T: Deserialize<'de>
{
    let mut deserializer = Deserializer::from_slice(bytes);
    let t = T::deserialize(&mut deserializer)?;

    if deserializer.input.finished() {
        Ok(t)
    } else {
        Err(Error::TrailingBytes)
    }
}


pub struct Deserializer<'de, R: Read<'de> + 'de> {
    input: R,
    phantom: PhantomData<&'de ()>,
}

impl<'de> Deserializer<'de, SliceReader<'de>> {
    pub fn from_slice<S: AsRef<[u8]> + 'de>(bytes: &'de S) -> Self {
        Self {
            input: SliceReader::new(bytes),
            phantom: PhantomData
        }
    }
}

impl<'de, R: IoRead> Deserializer<'de, BufferedReader<R>> {
    pub fn from_reader(reader: R) -> Self {
        Self {
            input: BufferedReader::new(reader),
            phantom: PhantomData
        }
    }
}

impl<'de, R: Read<'de>> Deserializer<'de, R> {
    #[inline]
    fn next(&mut self) -> Result<u8> {
        self.input.next().ok_or(Error::Eof)
    }

    #[inline]
    fn peek_next(&mut self) -> Result<u8> {
        self.input.peek_next().ok_or(Error::Eof)
    }

    #[inline]
    fn read<'a>(&'a mut self, bytes: usize) -> Result<Borrowed<'a, 'de>> {
        self.input.read(bytes).ok_or(Error::Eof)
    }

    #[inline]
    fn must_read<'a>(&'a mut self, bytes: usize) -> Result<Borrowed<'a, 'de>> {
        let borrowed = self.read(bytes)?;

        if borrowed.len() != bytes {
            Err(Error::Eof)
        } else {
            Ok(borrowed)
        }
    }

    // #[inline]
    // fn peek<'a>(&'a mut self, bytes: usize) -> Result<Borrowed<'a, 'de>> {
    //     self.input.peek(bytes).ok_or(Error::Eof)
    // }

    // #[inline]
    // fn consume(&mut self, bytes: usize) -> Result<usize> {
    //     self.input.consume(bytes).ok_or(Error::Eof)
    // }

    #[inline]
    fn must_consume(&mut self, bytes: usize) -> Result<()> {
        let mut total_consumed = 0;

        while total_consumed < bytes {
            match self.input.consume(bytes - total_consumed) {
                Some(consumed) => total_consumed += consumed,
                None => {
                    return Err(Error::Eof)
                },
            }
        }

        Ok(())
    }

    #[inline]
    fn get_param(&mut self, value: u8) -> Result<usize> {
        match value {
            0...23 => Ok(value as usize),
            24 => Ok(self.next()? as usize),
            25 => Ok(
                (self.next()? as usize) << 8 |
                (self.next()? as usize)
            ),
            // 25 => Ok(unsafe {
            //     *(self.must_read(2)?.as_slice().as_ptr() as *const u16) as usize
            // }),
            26 => Ok(
                (self.next()? as usize) << 24 |
                (self.next()? as usize) << 16 |
                (self.next()? as usize) << 8 |
                (self.next()? as usize)
            ),
            #[cfg(target_pointer_width = "64")]
            27 => {
                let num = (self.next()? as u64) << 56 |
                (self.next()? as u64) << 48 |
                (self.next()? as u64) << 40 |
                (self.next()? as u64) << 32 |
                (self.next()? as u64) << 24 |
                (self.next()? as u64) << 16 |
                (self.next()? as u64) << 8 |
                (self.next()? as u64);

                if num > usize::max_value() as u64 {
                    Err(Error::UsizeOverflow)
                } else {
                    Ok(num as usize)
                }
            }
            #[cfg(not(target_pointer_width = "64"))]
            27 => Err(Error::UsizeOverflow),
            _ => Err(Error::UnexpectedValue(Type::Any, value)),
        }
    }


    #[inline]
    fn parse_uint<V>(&mut self, visitor: V, value: u8) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        match value {
            0...23 => visitor.visit_u8(value as u8),
            24 => visitor.visit_u8(self.next()? as u8),
            25 => visitor.visit_u16(
                (self.next()? as u16) << 8 |
                (self.next()? as u16)
            ),
            26 => visitor.visit_u32(
                (self.next()? as u32) << 24 |
                (self.next()? as u32) << 16 |
                (self.next()? as u32) << 8 |
                (self.next()? as u32)
            ),
            27 => visitor.visit_u64(
                (self.next()? as u64) << 56 |
                (self.next()? as u64) << 48 |
                (self.next()? as u64) << 40 |
                (self.next()? as u64) << 32 |
                (self.next()? as u64) << 24 |
                (self.next()? as u64) << 16 |
                (self.next()? as u64) << 8 |
                (self.next()? as u64)
            ),
            _ => Err(Error::UnexpectedValue(Type::Uint, value)),
        }
    }

    // Note: << can change the sign bit, while >> can't
    #[inline]
    fn parse_int<V>(&mut self, visitor: V, value: u8) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        match value {
            0...15 => visitor.visit_i8(value as i8),
            16...23 => visitor.visit_i8(value as i8 - 24),
            24 => visitor.visit_i8(
                    // Reinterpret the u8 as an i8, through long, complex type punning
                    *unsafe { &mut *(&mut self.next()? as *mut u8 as *mut i8) }
                ),
            25 => visitor.visit_i16(
                    (self.next()? as i16) << 8 |
                    (self.next()? as i16)
                ),
            26 => visitor.visit_i32(
                    (self.next()? as i32) << 24 |
                    (self.next()? as i32) << 16 |
                    (self.next()? as i32) << 8 |
                    (self.next()? as i32)
                ),
            27 => visitor.visit_i64(
                    (self.next()? as i64) << 56 |
                    (self.next()? as i64) << 48 |
                    (self.next()? as i64) << 40 |
                    (self.next()? as i64) << 32 |
                    (self.next()? as i64) << 24 |
                    (self.next()? as i64) << 16 |
                    (self.next()? as i64) << 8 |
                    (self.next()? as i64)
                ),
            _ => Err(Error::UnexpectedValue(Type::Int, value)),
        }
    }

    #[inline]
    fn parse_float<V>(&mut self, visitor: V, value: u8) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        match value {
            4 => visitor.visit_f32(
                f32::from_bits(
                    (self.next()? as u32) << 24 |
                    (self.next()? as u32) << 16 |
                    (self.next()? as u32) << 8 |
                    (self.next()? as u32)
                )
            ),
            5 => visitor.visit_f64(
                f64::from_bits(
                    (self.next()? as u64) << 56 |
                    (self.next()? as u64) << 48 |
                    (self.next()? as u64) << 40 |
                    (self.next()? as u64) << 32 |
                    (self.next()? as u64) << 24 |
                    (self.next()? as u64) << 16 |
                    (self.next()? as u64) << 8 |
                    (self.next()? as u64)
                )
            ),
            _ => Err(Error::UnexpectedValue(Type::Misc, value)),
        }
    }

    // #[inline]
    fn ignore_value(&mut self) -> Result<()> {
        let byte = self.next()?;

        match ty(byte) {
            Type::Uint |
            Type::Int => match val(byte) {
                0...23 => {}, // Self-contained byte
                value @ 24...27 => {
                    // 24 => 1
                    // 25 => 2
                    // 26 => 4
                    // 27 => 8
                    let to_read = 2 << (value - 24);

                    // Don't have to worry about recursive reading because we will never read more
                    //   than the max buffer size
                    self.must_consume(to_read)?;
                }
                value => return Err(Error::UnexpectedValue(ty(byte), value)),
            }
            Type::Misc => match val(byte) {
                0...3 => {},
                value @ 4...5 => {
                    // 4 => 4
                    // 5 => 8
                    let to_read = 2 << (value - 2);

                    // Don't have to worry about recursive reading because we will never read more
                    //   than the max buffer size
                    self.must_consume(to_read)?;
                }
                value => return Err(Error::UnexpectedValue(Type::Misc, value)),
            }
            Type::Variant => {
                match val(byte) {
                    0...23 => {}
                    value @ 24...26 => {
                        // 24 => 1
                        // 25 => 2
                        // 26 => 4
                        let to_read = 2 << (value - 24);

                        // Don't have to worry about recursive reading because we will never read more
                        //   than the max buffer size
                        self.must_consume(to_read)?;
                    }
                    27 => self.ignore_value()?, // Ignore the string variant name
                    value => return Err(Error::UnexpectedValue(Type::Variant, value)),
                }

                // Ignore the variant content
                self.ignore_value()?;
            }
            Type::Seq => {
                let len = self.get_param(val(byte))?;

                for _ in 0..len {
                    self.ignore_value()?;
                }
            }
            Type::Bytes => {
                let len = self.get_param(val(byte))?;
                let mut bytes_to_parse = len;

                while bytes_to_parse > 0 {
                    let bytes_to_read = cmp::min(self.input.max_instant_read(), bytes_to_parse);

                    bytes_to_parse -= self.read(bytes_to_read)?.as_slice().len();
                }
            }
            Type::Map => {
                let len = self.get_param(val(byte))?;

                for _ in 0..len {
                    self.ignore_value()?; // key
                    self.ignore_value()?; // value
                }
            }
            Type::Reserved => return Err(Error::ExpectedType(vec![
                Type::Uint,
                Type::Int,
                Type::Misc,
                Type::Variant,
                Type::Seq,
                Type::Bytes,
                Type::Map
            ], byte)),
            _ => return Err(Error::NotAType),
        }

        Ok(())
    }
}

macro_rules! forward_num {
    () => {};
    ($fn:ident $($more:tt)*) => {
        #[inline]
        fn $fn <V>(self, visitor: V) -> Result<V::Value>
            where V: Visitor<'de>
        {
            let byte = self.next()?;

            match ty(byte) {
                Type::Uint => self.parse_uint(visitor, val(byte)),
                Type::Int => self.parse_int(visitor, val(byte)),
                Type::Misc => self.parse_float(visitor, val(byte)),
                _ => Err(Error::ExpectedType(vec![
                    Type::Uint,
                    Type::Int,
                    Type::Misc,
                ], byte)),
            }
        }

        forward_num!($($more)*);
    };
}
macro_rules! forward_to {
    () => {};
    ($fn:ident => $fn2:ident $($more:tt)*) => {
        #[inline]
        fn $fn <V>(self, visitor: V) -> Result<V::Value>
            where V: Visitor<'de>
        {
            self. $fn2 (visitor)
        }

        forward_to!($($more)*);
    };
}

impl<'de, 'a, R: Read<'de>> SerdeDeserializer<'de> for &'a mut Deserializer<'de, R> {
    type Error = Error;

    // #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.peek_next()?;

        if ty(byte) == Type::Variant {
            return visitor.visit_enum(VariantVisitor::new(&mut self));
        }

        self.must_consume(1)?;

        match ty(byte) {
            Type::Uint => self.parse_uint(visitor, val(byte)),
            Type::Int => self.parse_int(visitor, val(byte)),
            Type::Misc => match val(byte) {
                0 => visitor.visit_bool(false),
                1 => visitor.visit_bool(true),
                2 => visitor.visit_unit(),
                3 => visitor.visit_none(),
                4 => visitor.visit_f32(
                    f32::from_bits(
                        (self.next()? as u32) << 24 |
                        (self.next()? as u32) << 16 |
                        (self.next()? as u32) << 8 |
                        (self.next()? as u32)
                    )
                ),
                5 => visitor.visit_f64(
                    f64::from_bits(
                        (self.next()? as u64) << 56 |
                        (self.next()? as u64) << 48 |
                        (self.next()? as u64) << 40 |
                        (self.next()? as u64) << 32 |
                        (self.next()? as u64) << 24 |
                        (self.next()? as u64) << 16 |
                        (self.next()? as u64) << 8 |
                        (self.next()? as u64)
                    )
                ),
                _ => Err(Error::UnexpectedValue(Type::Misc, val(byte))),
            }
            Type::Variant => unreachable!(),
            Type::Seq => {
                let len = self.get_param(val(byte))?;

                visitor.visit_seq(SeqVisitor::new(&mut self, len))
            }
            Type::Bytes => {
                let len = self.get_param(val(byte))?;

                if self.input.max_instant_read() < len {
                    // need multiple reads to get full buffer

                    let mut buf = Vec::new();
                    let mut bytes_to_parse = len;

                    while bytes_to_parse > 0 {
                        let bytes_to_read = cmp::min(self.input.max_instant_read(), bytes_to_parse);

                        let bytes = self.read(bytes_to_read)?.as_slice();

                        bytes_to_parse -= bytes.len();
                        buf.extend_from_slice(bytes);
                    }

                    visitor.visit_byte_buf(buf)
                } else {
                    match self.read(len)? {
                        Borrowed::Transient(bytes) => visitor.visit_bytes(bytes),
                        Borrowed::Permanent(bytes) => visitor.visit_borrowed_bytes(bytes),
                    }
                }
            }
            Type::Map => {
                let len = self.get_param(val(byte))?;

                visitor.visit_map(SeqVisitor::new(&mut self, len))
            }
            Type::Reserved => Err(Error::ExpectedType(vec![
                Type::Int,
                Type::Uint,
                Type::Misc,
                Type::Variant,
                Type::Seq,
                Type::Bytes,
                Type::Map,
            ], byte)),
            _ => return Err(Error::NotAType),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.next()?;

        if ty(byte) != Type::Misc {
            Err(Error::ExpectedType(vec![Type::Misc], byte))
        } else {
            match val(byte) {
                0 => visitor.visit_bool(false),
                1 => visitor.visit_bool(true),
                2...5 => Err(Error::UnexpectedValue(Type::Misc, val(byte))),
                _ => Err(Error::UnexpectedValue(Type::Misc, val(byte))),
            }
        }
    }

    forward_num! {
        deserialize_i8
        deserialize_i16
        deserialize_i32
        deserialize_i64
        deserialize_u8
        deserialize_u16
        deserialize_u32
        deserialize_u64
        deserialize_f32
        deserialize_f64
    }
    forward_to! {
        deserialize_str => deserialize_bytes
        deserialize_string => deserialize_bytes
        deserialize_byte_buf => deserialize_bytes
    }

    // #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.next()?;

        match ty(byte) {
            Type::Uint => visitor.visit_char(
                std::char::from_u32(
                    match val(byte) {
                        value @ 0...23 => value as u32,
                        24 => self.next()? as u32,
                        25 => (self.next()? as u32) << 8 |
                            (self.next()? as u32),
                        26 => (self.next()? as u32) << 24 |
                            (self.next()? as u32) << 16 |
                            (self.next()? as u32) << 8 |
                            (self.next()? as u32),
                        27 => return Err(Error::UnexpectedValue(Type::Char, 27)),
                        value => return Err(Error::UnexpectedValue(Type::Char, value)), // Unexpected value
                    }
                ).ok_or(Error::FailedToParseChar)?
            ),
            Type::Bytes => {
                match val(byte) {
                    bytes @ 1...4 => {
                        match String::from_utf8(self.read(bytes as usize)?.into_vec()) {
                            Ok(s) => {
                                let mut chars = s.chars();

                                let first_char = match chars.next() {
                                    Some(ch) => ch,
                                    None => return Err(Error::FailedToParseChar),
                                };

                                if chars.next() != None {
                                    Err(Error::FailedToParseChar)
                                } else {
                                    visitor.visit_char(first_char)
                                }
                            },
                            Err(_) => Err(Error::FailedToParseChar),
                        }
                    }
                    value => Err(Error::UnexpectedValue(Type::Char, value)),
                }
            }
            _ => Err(Error::ExpectedType(vec![Type::Bytes, Type::Uint], byte)),
        }
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.next()?;

        match ty(byte) {
            Type::Bytes => {
                let len = self.get_param(val(byte))?;

                if self.input.max_instant_read() < len {
                    // need multiple reads to get full buffer

                    let mut buf = Vec::new();
                    let mut bytes_to_parse = len;

                    while bytes_to_parse > 0 {
                        let bytes_to_read = cmp::min(self.input.max_instant_read(), bytes_to_parse);

                        let bytes = self.read(bytes_to_read)?.as_slice();

                        bytes_to_parse -= bytes.len();
                        buf.extend_from_slice(bytes);
                    }

                    visitor.visit_byte_buf(buf)
                } else {
                    match self.read(len)? {
                        Borrowed::Transient(bytes) => visitor.visit_bytes(bytes),
                        Borrowed::Permanent(bytes) => visitor.visit_borrowed_bytes(bytes),
                    }
                }
            }
            // Type::Seq => {
            //     let len = self.get_param(val(byte))?;
            //     let mut buf = Vec::new();
            //
            //     for _ in 0..len {
            //         let key = self.next()?;
            //
            //         if ty(key) != Type::Uint {
            //             return Err(Error::ExpectedType(vec![Type::Uint], key));
            //         }
            //         match val(key) {
            //             value @ 0...23 => buf.push(value),
            //             24 => buf.push(self.next()?),
            //             value => return Err(Error::UnexpectedValue(Type::Uint, value)),
            //         }
            //     }
            //
            //     visitor.visit_bytes(&buf[..])
            // }
            _ => Err(Error::ExpectedType(vec![Type::Bytes/*, Type::Seq*/], byte)),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.peek_next()?;

        if byte == (Type::Misc as u8) << 5 | 3 { // misc - None
            self.must_consume(1)?;
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.next()?;

        if byte == (Type::Misc as u8) << 5 | 2 { // misc - ()
            visitor.visit_unit()
        } else {
            if ty(byte) == Type::Misc {
                Err(Error::UnexpectedValue(Type::Misc, val(byte)))
            } else {
                Err(Error::ExpectedType(vec![Type::Misc], byte))
            }
        }
    }

    #[inline]
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.next()?;

        match ty(byte) {
            Type::Seq => {
                let len = self.get_param(val(byte))?;

                visitor.visit_seq(SeqVisitor::new(&mut self, len))
            }
            _ => Err(Error::ExpectedType(vec![Type::Seq], byte))
        }
    }

    #[inline]
    fn deserialize_tuple<V>(mut self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.next()?;

        match ty(byte) {
            Type::Seq => {
                let seq_len = self.get_param(val(byte))?;

                if seq_len != len {
                    Err(Error::UnexpectedValue(Type::Seq, val(byte)))
                } else {
                    visitor.visit_seq(SeqVisitor::new(&mut self, len))
                }
            }
            _ => Err(Error::ExpectedType(vec![Type::Seq], byte)),
        }
    }

    #[inline]
    fn deserialize_tuple_struct<V>(self, _name: &'static str, len: usize, visitor: V)
        -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        self.deserialize_tuple(len, visitor)
    }

    #[inline]
    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let byte = self.next()?;

        match ty(byte) {
            Type::Map => {
                let len = self.get_param(val(byte))?;

                visitor.visit_map(SeqVisitor::new(&mut self, len))
            }
            _ => Err(Error::ExpectedType(vec![Type::Map], byte))
        }
    }

    #[inline]
    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str],
        visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_enum<V>(mut self, _name: &'static str, _variants: &'static [&'static str],
        visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.peek_next()?;

        match ty(byte) {
            Type::Uint => {
                self.must_consume(1)?;

                match val(byte) {
                    value @ 0...23 => visitor.visit_enum((value as u32).into_deserializer()),
                    24 => visitor.visit_enum((self.next()? as u32).into_deserializer()),
                    25 => visitor.visit_enum(
                        (
                            (self.next()? as u32) << 8 |
                            (self.next()? as u32)
                        ).into_deserializer()
                    ),
                    26 => visitor.visit_enum(
                        (
                            (self.next()? as u32) << 24 |
                            (self.next()? as u32) << 16 |
                            (self.next()? as u32) << 8 |
                            (self.next()? as u32)
                        ).into_deserializer()
                    ),
                    27 => Err(Error::UsizeOverflow),
                    value => Err(Error::UnexpectedValue(Type::Uint, value)),
                }
            }
            Type::Variant => visitor.visit_enum(VariantVisitor::new(&mut self)),
            _ => Err(Error::ExpectedType(vec![Type::Uint, Type::Variant], byte)),
        }
    }

    // #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        let byte = self.next()?;

        match ty(byte) {
            Type::Variant => match val(byte) {
                value @ 0...23 => visitor.visit_u32(value as u32),
                24 => visitor.visit_u32(self.next()? as u32),
                25 => visitor.visit_u32(
                        (self.next()? as u32) << 8 |
                        (self.next()? as u32)
                ),
                26 => visitor.visit_u32(
                        (self.next()? as u32) << 24 |
                        (self.next()? as u32) << 16 |
                        (self.next()? as u32) << 8 |
                        (self.next()? as u32)
                ),
                27 => {
                    let len = match self.next()? {
                        value @ 0...251 => value as usize,
                        252 => self.next()? as usize,
                        253 => (
                            (self.next()? as usize) << 8 |
                            (self.next()? as usize)
                        ),
                        254 => (
                            (self.next()? as usize) << 24 |
                            (self.next()? as usize) << 16 |
                            (self.next()? as usize) << 8 |
                            (self.next()? as usize)
                        ),
                        value => return Err(Error::UnexpectedValue(Type::Variant, value)),
                    };

                    if self.input.max_instant_read() < len {
                        // need multiple reads to get full buffer

                        let mut buf = Vec::new();
                        let mut bytes_to_parse = len;

                        while bytes_to_parse > 0 {
                            let bytes_to_read = cmp::min(self.input.max_instant_read(), bytes_to_parse);

                            let bytes = self.read(bytes_to_read)?.as_slice();

                            bytes_to_parse -= bytes.len();
                            buf.extend_from_slice(bytes);
                        }

                        visitor.visit_byte_buf(buf)
                    } else {
                        match self.read(len)? {
                            Borrowed::Transient(bytes) => visitor.visit_bytes(bytes),
                            Borrowed::Permanent(bytes) => visitor.visit_borrowed_bytes(bytes),
                        }
                    }
                }
                value => Err(Error::UnexpectedValue(Type::Variant, value)),
            }
            _ => Err(Error::ExpectedType(vec![Type::Variant], byte)), // Unexpected type
        }
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        self.ignore_value()?;

        visitor.visit_unit()
    }
}

struct SeqVisitor<'a, 'de: 'a, R: Read<'de> + 'de> {
    de: &'a mut Deserializer<'de, R>,
    index: usize,
    len: usize,
}

impl<'a, 'de, R: Read<'de>> SeqVisitor<'a, 'de, R> {
    #[inline]
    pub fn new(de: &'a mut Deserializer<'de, R>, len: usize) -> Self {
        Self {
            de,
            len,
            index: 0,
        }
    }
}

impl<'a, 'de, R: Read<'de>> SeqAccess<'de> for SeqVisitor<'a, 'de, R> {
    type Error = Error;

    #[inline]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>
    {
        if self.index >= self.len {
            return Ok(None);
        }

        self.index += 1;

        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'a, 'de, R: Read<'de>> MapAccess<'de> for SeqVisitor<'a, 'de, R> {
    type Error = Error;

    #[inline]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>
    {
        if self.index >= self.len {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>
    {
        self.index += 1;

        seed.deserialize(&mut *self.de)
    }
}


struct VariantVisitor<'a, 'de: 'a, R: Read<'de> + 'de> {
    de: &'a mut Deserializer<'de, R>,
}

impl<'a, 'de, R: Read<'de>> VariantVisitor<'a, 'de, R> {
    #[inline]
    fn new(de: &'a mut Deserializer<'de, R>) -> Self {
        Self {
            de
        }
    }
}

impl<'a, 'de, R: Read<'de>> EnumAccess<'de> for VariantVisitor<'a, 'de, R> {
    type Error = Error;
    type Variant = Self;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>
    {
        // Seed is going to call deserialize_identifier, which is where we get the variant
        Ok((seed.deserialize(&mut *self.de)?, self))
    }
}

impl<'de, 'a, R: Read<'de>> VariantAccess<'de> for VariantVisitor<'a, 'de, R> {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<()> {
        let byte = self.de.next()?;

        if byte == (Type::Misc as u8) << 5 | 2 {
            Ok(())
        } else if ty(byte) == Type::Misc {
            Err(Error::UnexpectedValue(Type::Misc, val(byte)))
        } else {
            Err(Error::ExpectedType(vec![Type::Misc], byte))
        }
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>
    {
        seed.deserialize(self.de)
    }

    #[inline]
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        self.de.deserialize_tuple(len, visitor)
    }

    #[inline]
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>
    {
        self.de.deserialize_seq(visitor)
    }
}
