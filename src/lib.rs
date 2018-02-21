#[macro_use]
extern crate lazy_static;
extern crate serde;

mod error;
mod de;
mod ser;

pub use de::*;
pub use ser::*;
pub use error::*;


lazy_static! {
    static ref WRONG_ENDIANNESS: bool = {
        let mut num: u16 = 0xff00;

        let first_byte: u8 = unsafe {
            *(&mut num as *mut u16 as *mut u8)
        };

        // Although sacrificing readability, x86 uses Little Endian, so it's faster to avoid
        //   flipping endianness all of the time
        first_byte != 0xff
    };
}
