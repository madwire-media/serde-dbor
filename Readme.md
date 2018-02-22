# DBOR - Dq's Binary Object Representation

DBOR is a serialization format based on CBOR, designed for Rust, and optimized for speed and file size. It uses buffered reading and writing systems when interacting with io streams for maximum efficiency.

I created this because I needed to save and load a large 23MB CBOR file containing a huge tree structure, and it was taking 6 seconds to load and 60 seconds to save. However, now with DBOR, both the save and load times went down to 0.3 seconds, and the file size went down to 19MB. (that's a difference of 1:20 in read speed and 1:200 in write speed!)


# Example Usage
(derived from [serde_json's tutorial](https://github.com/serde-rs/json#parsing-json-as-strongly-typed-data-structures))

### `Cargo.toml`
```toml
[dependencies]
serde = "*"
serde_derive = "*"
serde_dbor = "*"
```

### `main.rs`
```rust
extern crate serde;
extern crate serde_dbor;

#[macro_use]
extern crate serde_derive;

use serde_dbor::Error;

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,
    phones: Vec<String>
}

fn example<'a>(data: &'a [u8]) => Result<(), Error> {
    // Parse the data into a Person object.
    let p: Person = serde_dbor::from_slice(data)?;

    // Do things just like with any other Rust data structure.
    println!("Please call {} at the number {}", p.name, p.phones[0]);

    Ok(())
}
```

## Spec
DBOR, just like CBOR, is composed of instruction bytes and additional content bytes. However, in DBOR, every item needs to be described before its content, meaning that indefinite-length arrays, strings, or maps are not allowed because they would require a termination byte at the end of the item. An instruction byte is split up into two sections of 3 bits and 5 bits, respectively. The first 3 bits define the type of the item, and the last 5 are a parameter for that item, which in some cases can be the value of the item itself. For example, an unsigned integer with a value of 21 would be stored as `0x15`, or `0b000 10101`, because type 0 (`0b000`) is a uint and the byte has enough space left over to encode the number 21 (`0b10101`).

When an instruction byte indicates that the parameter is of a certain size `n`, the next `n` bytes will be used for that parameter, and then afterwards will be the content of the item described by the instruction byte. For example, a `u16` parameter takes up the two bytes immediately after the instruction byte. However, when serializing a `u16`, it may be shortened into a `u8` or into the instruction byte itself. Also, it should be noted that DBOR stores multi-byte integers and floats in little endian because it makes serialization/deserialization on most machines faster (x86 uses little endian).

### Instruction Bytes

<table>
  <tr>
    <th>Type ID</th>
    <th>Encoded Type</th>
    <th>Parameter Descriptions</th>
  </tr>
  <tr>
    <td><code>0b000</code> (<code>0</code>)</td>
    <td>uint</td>
    <td>
      <ul>
        <li><code>0-23</code> - values <code>0-23</code></li>
        <li><code>24</code> - <code>u8</code></li>
        <li><code>25</code> - <code>u16</code></li>
        <li><code>26</code> - <code>u32</code></li>
        <li><code>27</code> - <code>u64</code></li>
        <li><code>28-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><code>0b001</code> (<code>1</code>)</td>
    <td>int</td>
    <td>
      <ul>
        <li><code>0-15</code> - values <code>0-15</code></li>
        <li><code>16-23</code> - values <code>-8--1</code></li>
        <li><code>24</code> - <code>i8</code></li>
        <li><code>25</code> - <code>i16</code></li>
        <li><code>26</code> - <code>i32</code></li>
        <li><code>27</code> - <code>i64</code></li>
        <li><code>28-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><code>0b010</code> (<code>2</code>)</td>
    <td>misc</td>
    <td>
      <ul>
        <li><code>0</code> - <code>false</code></li>
        <li><code>1</code> - <code>true</code></li>
        <li><code>2</code> - <code>()</code></li>
        <li><code>3</code> - <code>None</code></li>
        <li><code>4</code> - <code>f32</code></li>
        <li><code>5</code> - <code>f64</code></li>
        <li><code>6-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><code>0b011</code> (<code>3</code>)</td>
    <td>variant (enum)</td>
    <td>
      <ul>
        <li><code>0-23</code> - variant ids <code>0-23</code></li>
        <li><code>24</code> - variant id as <code>u8</code></li>
        <li><code>25</code> - variant id as <code>u16</code></li>
        <li><code>26</code> - variant id as <code>u32</code></li>
        <li><code>27</code> - named variant (see below)</li>
        <li><code>28-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><code>0b100</code> (<code>4</code>)</td>
    <td>seq (array/tuple/struct)</td>
    <td>
      <ul>
        <li><code>0-23</code> - length of <code>0-23</code></li>
        <li><code>24</code> - length as <code>u8</code></li>
        <li><code>25</code> - length as <code>u16</code></li>
        <li><code>26</code> - length as <code>u32</code></li>
        <li><code>27</code> - length as <code>u64</code> (only on 64-bit machines)</li>
        <li><code>28-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><code>0b101</code> (<code>5</code>)</td>
    <td>bytes (string/byte array)</td>
    <td>
      <ul>
        <li><code>0-23</code> - length of <code>0-23</code></li>
        <li><code>24</code> - length as <code>u8</code></li>
        <li><code>25</code> - length as <code>u16</code></li>
        <li><code>26</code> - length as <code>u32</code></li>
        <li><code>27</code> - length as <code>u64</code> (only on 64-bit machines)</li>
        <li><code>28-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><code>0b110</code> (<code>6</code>)</td>
    <td>map</td>
    <td>
      <ul>
        <li><code>0-23</code> - length of <code>0-23</code></li>
        <li><code>24</code> - length as <code>u8</code></li>
        <li><code>25</code> - length as <code>u16</code></li>
        <li><code>26</code> - length as <code>u32</code></li>
        <li><code>27</code> - length as <code>u64</code> (only on 64-bit machines)</li>
        <li><code>28-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><code>0b111</code> (<code>7</code>)</td>
    <td><i>reserved</i></td>
    <td>
      <ul>
        <li><code>0-31</code> - <i>reserved</i></li>
      </ul>
    </td>
  </tr>
</table>

#### Named Variant Byte
* `0-247` - name length of `0-247`
* `248` - name length as `u8`
* `249` - name length as `u16`
* `250` - name length as `u32`
* `251` - name length as `u64` (only on 64-bit machines)
* `252-255` - *reserved*

Note: serialization using named variants isn't currently implemented, but deserialization is.

## Example Data
### Rust Code
```rust
struct Data {
  some_text: String,
  a_small_number: u64,
  a_byte: u8,
  some_important_numbers: Vec<u16>,
}

let data = Data {
  some_text: "Hello world!",
  a_small_number: 0x04,
  a_byte: 0x27,
  some_important_numbers: vec![
    0x1234,
    0x6789,
    0xabcd,
  ]
}
```

### Annotated Hex Dump of DBOR
```
84                    # Seq(4)
  ac                    # Bytes(12)
    48 65 6c 6c 6f 20...
    77 6f 72 6c 64 21     # "Hello world!"
  04                    # uint(4)
  18                    # u8
    27                    # 0x27
  83                    # Seq(3)
    19                    # u16
      34 12                 # 0x1234
    19                    # u16
      89 67                 # 0x6789
    19                    # u16
      cd ab                 # 0xabcd
```
