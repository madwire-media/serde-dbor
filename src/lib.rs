//! # DBOR - Dq's Binary Object Representation
//!
//! DBOR is a serialization format based on CBOR, designed for Rust, and optimized for speed and file size. It uses buffered reading and writing systems when interacting with io streams for maximum efficiency.
//!
//! I created this because I needed to save and load a large 23MB CBOR file containing a huge tree structure, and it was taking 6 seconds to load and 60 seconds to save. However, now with DBOR, both the save and load times went down to 0.3 seconds, and the file size went down to 19MB. (that's a difference of 1:20 in read speed and 1:200 in write speed!)
//!
//!
//! # Example Usage
//! (derived from [serde_json's tutorial](https://github.com/serde-rs/json#parsing-json-as-strongly-typed-data-structures))
//!
//! ### `Cargo.toml`
//! ```toml
//! [dependencies]
//! serde = "*"
//! serde_derive = "*"
//! serde_dbor = "*"
//! ```
//!
//! ### `main.rs`
//! ```rust
//! extern crate serde;
//! extern crate serde_dbor;
//!
//! #[macro_use]
//! extern crate serde_derive;
//!
//! use serde_dbor::Error;
//!
//! #[derive(Serialize, Deserialize)]
//! struct Person {
//!     name: String,
//!     age: u8,
//!     phones: Vec<String>
//! }
//!
//! fn example<'a>(data: &'a [u8]) => Result<(), Error> {
//!     // Parse the data into a Person object.
//!     let p: Person = serde_dbor::from_slice(data)?;
//!
//!     // Do things just like with any other Rust data structure.
//!     println!("Please call {} at the number {}", p.name, p.phones[0]);
//!
//!     Ok(())
//! }
//! ```

#[macro_use]
extern crate lazy_static;
extern crate serde;

/// When serializing or deserializing DBOR goes wrong
mod error;
/// Deserialize DBOR data to a Rust data structure
mod de;
/// Serialize Rust data structure into DBOR data
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
        first_byte == 0xff
    };
}
