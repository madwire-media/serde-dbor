use std::io::Write as IoWrite;

use serde::ser::{self, Serializer as SerdeSerializer, Serialize};

mod write;

use error::*;
use self::write::*;
use super::WRONG_ENDIANNESS;


const TYPE_UINT: u8 = 0b00000000;
const TYPE_INT: u8 = 0b00100000;
const TYPE_MISC: u8 = 0b01000000;
const TYPE_VARIANT: u8 = 0b01100000;
const TYPE_SEQ: u8 = 0b10000000;
const TYPE_BYTES: u8 = 0b10100000;
const TYPE_MAP: u8 = 0b11000000;

const VALUE_MASK: u8 = 0b00011111;


pub struct Serializer<W: Write> {
    output: W,
}

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize
{
    let mut serializer = Serializer {
        output: VecWriter::new(),
    };
    value.serialize(&mut serializer)?;
    serializer.output.finish()
}

pub fn to_writer<T, W>(value: &T, writer: W) -> Result<W>
where
    T: Serialize,
    W: IoWrite,
{
    let mut serializer = Serializer {
        output: IoWriter::new(writer),
    };
    value.serialize(&mut serializer)?;
    serializer.output.finish()
}

impl<'a, W: Write> SerdeSerializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.put_byte(TYPE_MISC | (v as u8))
    }

    #[inline]
    fn serialize_i8(self, mut v: i8) -> Result<()> {
        match v {
            0...15 => self.output.put_byte(TYPE_INT | (v as u8 & VALUE_MASK)),
            -8...-1 => self.output.put_byte(TYPE_INT | ((v + 24) as u8 & VALUE_MASK)),
            -0x80...-8 | 16...0x7f | _ => {
                self.output.put_byte(TYPE_INT | 24)?;
                self.output.put_byte(unsafe { *(&mut v as *mut i8 as *mut u8) })
            }
        }
    }

    // #[inline]
    fn serialize_i16(self, mut v: i16) -> Result<()> {
        match v {
            0...15 => self.output.put_byte(TYPE_INT | (v as u8 & VALUE_MASK)),
            -8...-1 => self.output.put_byte(TYPE_INT | ((v + 24) as u8 & VALUE_MASK)),
            -0x80...-8 | 16...0x7f => {
                self.output.put_byte(TYPE_INT | 24)?;
                self.output.put_byte(unsafe { *(&mut (v as i8) as *mut i8 as *mut u8) })
            }
            -0x8000...-0x81 | 0x80...0x7fff | _ => {
                self.output.put_byte(TYPE_INT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut v as *mut i16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )
            }
        }
    }

    #[inline]
    fn serialize_i32(self, mut v: i32) -> Result<()> {
        match v {
            0...15 => self.output.put_byte(TYPE_INT | (v as u8 & VALUE_MASK)),
            -8...-1 => self.output.put_byte(TYPE_INT | ((v + 24) as u8 & VALUE_MASK)),
            -0x80...-8 | 16...0x7f => {
                self.output.put_byte(TYPE_INT | 24)?;
                self.output.put_byte(unsafe { *(&mut (v as i8) as *mut i8 as *mut u8) })
            }
            -0x8000...-0x81 | 0x80...0x7fff => {
                self.output.put_byte(TYPE_INT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (v as i16) as *mut i16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )
            }
            -0x80000000...-0x8001 | 0x8000...0x7fffffff | _ => {
                self.output.put_byte(TYPE_INT | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut v as *mut i32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )
            }
        }
    }

    // #[inline]
    fn serialize_i64(self, mut v: i64) -> Result<()> {
        match v {
            0...15 => self.output.put_byte(TYPE_INT | (v as u8 & VALUE_MASK)),
            -8...-1 => self.output.put_byte(TYPE_INT | ((v + 24) as u8 & VALUE_MASK)),
            -0x80...-8 | 16...0x7f => {
                self.output.put_byte(TYPE_INT | 24)?;
                self.output.put_byte(unsafe { *(&mut (v as i8) as *mut i8 as *mut u8) })
            }
            -0x8000...-0x81 | 0x80...0x7fff => {
                self.output.put_byte(TYPE_INT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (v as i16) as *mut i16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )
            }
            -0x80000000...-0x8001 | 0x8000...0x7fffffff => {
                self.output.put_byte(TYPE_INT | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (v as i32) as *mut i32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )
            }
            -0x8000000000000000...-0x80000001 | 0x80000000...0x7fffffffffffffff | _ => {
                self.output.put_byte(TYPE_INT | 27)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut v as *mut i64 as *mut [u8; 8]) },
                    *WRONG_ENDIANNESS
                )
            }
        }
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<()> {
        match v {
            0...23 => self.output.put_byte(TYPE_UINT | (v as u8 & VALUE_MASK)),
            24...0xff | _ => {
                self.output.put_byte(TYPE_UINT | 24)?;
                self.output.put_byte(v)
            }
        }
    }

    #[inline]
    fn serialize_u16(self, mut v: u16) -> Result<()> {
        match v {
            0...23 => self.output.put_byte(TYPE_UINT | (v as u8 & VALUE_MASK)),
            24...0xff => {
                self.output.put_byte(TYPE_UINT | 24)?;
                self.output.put_byte(v as u8)
            }
            0x100...0xffff | _ => {
                self.output.put_byte(TYPE_UINT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut v as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )
            }
        }
    }

    // #[inline]
    fn serialize_u32(self, mut v: u32) -> Result<()> {
        match v {
            0...23 => self.output.put_byte(TYPE_UINT | (v as u8 & VALUE_MASK)),
            24...0xff => {
                self.output.put_byte(TYPE_UINT | 24)?;
                self.output.put_byte(v as u8)
            }
            0x100...0xffff => {
                self.output.put_byte(TYPE_UINT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (v as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )
            }
            0x10000...0xffffffff | _ => {
                self.output.put_byte(TYPE_UINT | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut v as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )
            }
        }
    }

    // #[inline]
    fn serialize_u64(self, mut v: u64) -> Result<()> {
        match v {
            0...23 => self.output.put_byte(TYPE_UINT | (v as u8 & VALUE_MASK)),
            24...0xff => {
                self.output.put_byte(TYPE_UINT | 24)?;
                self.output.put_byte(v as u8)
            }
            0x100...0xffff => {
                self.output.put_byte(TYPE_UINT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (v as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )
            }
            0x10000...0xffffffff => {
                self.output.put_byte(TYPE_UINT | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (v as u32) as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )
            }
            0x100000000...0xffffffffffffffff | _ => {
                self.output.put_byte(TYPE_UINT | 27)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut v as *mut u64 as *mut [u8; 8]) },
                    *WRONG_ENDIANNESS
                )
            }
        }
    }

    #[inline]
    fn serialize_f32(self, mut v: f32) -> Result<()> {
        self.output.put_byte(TYPE_MISC | 4)?;
        self.output.put_bytes(
            unsafe { &mut *(&mut v as *mut f32 as *mut [u8; 4]) },
            *WRONG_ENDIANNESS
        )
    }

    #[inline]
    fn serialize_f64(self, mut v: f64) -> Result<()> {
        self.output.put_byte(TYPE_MISC | 5)?;
        self.output.put_bytes(
            unsafe { &mut *(&mut v as *mut f64 as *mut [u8; 8]) },
            *WRONG_ENDIANNESS
        )
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_u32(v as u32)

        // let mut buf = [0; 4];
        //
        // let slice = v.encode_utf8(&mut buf).as_bytes();
        //
        // self.output.put_byte(TYPE_BYTES | slice.len() as u8)?;
        // self.output.put_bytes(slice, false)
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }

    // #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        let len = v.len();

        match len {
            0...23 => {
                self.output.put_byte(TYPE_BYTES | len as u8)?;
            }
            24...0xff => {
                self.output.put_byte(TYPE_BYTES | 24)?;
                self.output.put_byte(len as u8)?;
            }
            0x0100...0xffff => {
                self.output.put_byte(TYPE_BYTES | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            0x00010000...0xffffffff => {
                self.output.put_byte(TYPE_BYTES | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u32) as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            #[cfg(target_pointer_width = "64")]
            0x0000000100000000...0xffffffffffffffff | _ => {
                self.output.put_byte(TYPE_BYTES | 27)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u64) as *mut u64 as *mut [u8; 8]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            #[cfg(not(target_pointer_width = "64"))]
            _ => Err(Error::TODO), // Too many bytes to load for this current machine
        }

        self.output.put_bytes(v, false)
    }

    #[inline]
    fn serialize_none(self) -> Result<()> {
        self.output.put_byte(TYPE_MISC | 3)
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<()> {
        self.output.put_byte(TYPE_MISC | 2)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(self, _name: &'static str, variant_index: u32,
        _variant: &'static str) -> Result<()>
    {
        self.serialize_u32(variant_index)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        value.serialize(self)
    }

    // #[inline]
    fn serialize_newtype_variant<T>(self, _name: &'static str, mut variant_index: u32,
        _variant: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        match variant_index {
            0...23 => {
                self.output.put_byte(TYPE_VARIANT | (variant_index as u8 & VALUE_MASK))?;
            }
            24...0xff => {
                self.output.put_byte(TYPE_VARIANT | 24)?;
                self.output.put_byte(variant_index as u8)?;
            }
            0x100...0xffff => {
                self.output.put_byte(TYPE_VARIANT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (variant_index as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            0x10000...0xffffffff | _ => {
                self.output.put_byte(TYPE_VARIANT | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut variant_index as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )?;
            }
        }

        value.serialize(&mut *self)?;

        Ok(())
    }

    // #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        match len {
            Some(len) => match len {
                0...23 => {
                    self.output.put_byte(TYPE_SEQ | len as u8)?;
                }
                24...0xff => {
                    self.output.put_byte(TYPE_SEQ | 24)?;
                    self.output.put_byte(len as u8)?;
                }
                0x0100...0xffff => {
                    self.output.put_byte(TYPE_SEQ | 25)?;
                    self.output.put_bytes(
                        unsafe { &mut *(&mut (len as u16) as *mut u16 as *mut [u8; 2]) },
                        *WRONG_ENDIANNESS
                    )?;
                }
                0x00010000...0xffffffff => {
                    self.output.put_byte(TYPE_SEQ | 26)?;
                    self.output.put_bytes(
                        unsafe { &mut *(&mut (len as u32) as *mut u32 as *mut [u8; 4]) },
                        *WRONG_ENDIANNESS
                    )?;
                }
                #[cfg(target_pointer_width = "64")]
                0x0000000100000000...0xffffffffffffffff | _ => {
                    self.output.put_byte(TYPE_SEQ | 27)?;
                    self.output.put_bytes(
                        unsafe { &mut *(&mut (len as u64) as *mut u64 as *mut [u8; 8]) },
                        *WRONG_ENDIANNESS
                    )?;
                }
                #[cfg(not(target_pointer_width = "64"))]
                _ => Err(Error::TODO), // Too many bytes to load for this current machine
            }
            None => return Err(Error::TODO), // Must know the size of an array ahead of time
        }

        Ok(self)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    // #[inline]
    fn serialize_tuple_variant(self, _name: &'static str, mut variant_index: u32,
        _variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant>
    {
        // Start variant
        match variant_index {
            0...23 => {
                self.output.put_byte(TYPE_VARIANT | (variant_index as u8 & VALUE_MASK))?;
            }
            24...0xff => {
                self.output.put_byte(TYPE_VARIANT | 24)?;
                self.output.put_byte(variant_index as u8)?;
            }
            0x100...0xffff => {
                self.output.put_byte(TYPE_VARIANT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (variant_index as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            0x10000...0xffffffff | _ => {
                self.output.put_byte(TYPE_VARIANT | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut variant_index as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )?;
            }
        }

        // Start seq
        match len {
            0...23 => {
                self.output.put_byte(TYPE_SEQ | len as u8)?;
            }
            24...0xff => {
                self.output.put_byte(TYPE_SEQ | 24)?;
                self.output.put_byte(len as u8)?;
            }
            0x0100...0xffff => {
                self.output.put_byte(TYPE_SEQ | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            0x00010000...0xffffffff => {
                self.output.put_byte(TYPE_SEQ | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u32) as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            #[cfg(target_pointer_width = "64")]
            0x0000000100000000...0xffffffffffffffff | _ => {
                self.output.put_byte(TYPE_SEQ | 27)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u64) as *mut u64 as *mut [u8; 8]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            #[cfg(not(target_pointer_width = "64"))]
            _ => Err(Error::TODO), // Too many bytes to load for this current machine
        }

        Ok(self)
    }

    // #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeTupleVariant>
    {
        match len {
            Some(len) => match len {
                0...23 => {
                    self.output.put_byte(TYPE_MAP | len as u8)?;
                }
                24...0xff => {
                    self.output.put_byte(TYPE_MAP | 24)?;
                    self.output.put_byte(len as u8)?;
                }
                0x0100...0xffff => {
                    self.output.put_byte(TYPE_MAP | 25)?;
                    self.output.put_bytes(
                        unsafe { &mut *(&mut (len as u16) as *mut u16 as *mut [u8; 2]) },
                        *WRONG_ENDIANNESS
                    )?;
                }
                0x00010000...0xffffffff => {
                    self.output.put_byte(TYPE_MAP | 26)?;
                    self.output.put_bytes(
                        unsafe { &mut *(&mut (len as u32) as *mut u32 as *mut [u8; 4]) },
                        *WRONG_ENDIANNESS
                    )?;
                }
                #[cfg(target_pointer_width = "64")]
                0x0000000100000000...0xffffffffffffffff | _ => {
                    self.output.put_byte(TYPE_MAP | 27)?;
                    self.output.put_bytes(
                        unsafe { &mut *(&mut (len as u64) as *mut u64 as *mut [u8; 8]) },
                        *WRONG_ENDIANNESS
                    )?;
                }
                #[cfg(not(target_pointer_width = "64"))]
                _ => Err(Error::TODO), // Too many bytes to load for this current machine
            }
            None => return Err(Error::TODO), // Must know the size of an array ahead of time
        }

        Ok(self)
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_seq(Some(len))
    }

    // #[inline]
    fn serialize_struct_variant(self, _name: &'static str, mut variant_index: u32,
        _variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant>
    {
        // Start variant
        match variant_index {
            0...23 => {
                self.output.put_byte(TYPE_VARIANT | (variant_index as u8 & VALUE_MASK))?;
            }
            24...0xff => {
                self.output.put_byte(TYPE_VARIANT | 24)?;
                self.output.put_byte(variant_index as u8)?;
            }
            0x100...0xffff => {
                self.output.put_byte(TYPE_VARIANT | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (variant_index as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            0x10000...0xffffffff | _ => {
                self.output.put_byte(TYPE_VARIANT | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut variant_index as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )?;
            }
        }

        // Start seq
        match len {
            0...23 => {
                self.output.put_byte(TYPE_SEQ | len as u8)?;
            }
            24...0xff => {
                self.output.put_byte(TYPE_SEQ | 24)?;
                self.output.put_byte(len as u8)?;
            }
            0x0100...0xffff => {
                self.output.put_byte(TYPE_SEQ | 25)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u16) as *mut u16 as *mut [u8; 2]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            0x00010000...0xffffffff => {
                self.output.put_byte(TYPE_SEQ | 26)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u32) as *mut u32 as *mut [u8; 4]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            #[cfg(target_pointer_width = "64")]
            0x0000000100000000...0xffffffffffffffff | _ => {
                self.output.put_byte(TYPE_SEQ | 27)?;
                self.output.put_bytes(
                    unsafe { &mut *(&mut (len as u64) as *mut u64 as *mut [u8; 8]) },
                    *WRONG_ENDIANNESS
                )?;
            }
            #[cfg(not(target_pointer_width = "64"))]
            _ => Err(Error::TODO), // Too many bytes to load for this current machine
        }

        Ok(self)
    }
}

impl<'a, W: Write> ser::SerializeSeq for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTuple for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTupleStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized +  Serialize
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTupleVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeMap for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        key.serialize(&mut **self)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        // Treat struct as a seq
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeStructVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize
    {
        // Treat struct as a seq
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}
