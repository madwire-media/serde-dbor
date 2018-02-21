# DBOR - Dq's Binary Object Representation

DBOR is a serialization format based on CBOR, designed for Rust, and optimized for speed and file size. I created this because at one point I had to deal with a large 23MB CBOR file containing a huge tree structure, and it was taking 6 seconds to load and 60 seconds to save. However, now with DBOR, both the save and load times went down to 0.3 seconds, and the file size went down to 19MB. (that's a difference of 1:20 in read speed and 1:200 in write speed!)

## Spec
DBOR, just like CBOR, is composed of instruction bytes and additional content bytes. However, in DBOR, every item needs to be described before its content, meaning that indefinite-length arrays, strings, or maps are not allowed because they would require a termination byte at the end of the item. An instruction byte is split up into two sections of 3 bits and 5 bits, respectively. The first 3 bits define the type of the item, and the last 5 are a parameter for that item, which in some cases can be the value of the item itself. For example, an unsigned integer with a value of 21 would be stored as `0x15`, or `0b000 10101`, because type 0 (`0b000`) is a uint and the byte has enough space left over to encode the number 21 (`0b10101`).

When an instruction byte indicates that the parameter is of a certain size `n`, the next `n` bytes will be used for that parameter, and then afterwards will be the content of the item described by the instruction byte.

### Instruction Bytes

<table>
  <tr>
    <th>Type ID</th>
    <th>Encoded Type</th>
    <th>Parameter Descriptions</th>
  </tr>
  <tr>
    <td>`0b000` (`0`)</td>
    <td>uint</td>
    <td>
      <ul>
        <li>`0-23` - values `0-23`</li>
        <li>`24` - `u8`</li>
        <li>`25` - `u16`</li>
        <li>`26` - `u32`</li>
        <li>`27` - `u64`</li>
        <li>`28-31` - *reserved*</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td>`0b001` (`1`)</td>
    <td>int</td>
    <td>
      <ul>
        <li>`0-15` - values `0-15`</li>
        <li>`16-23` - values `-8--1`</li>
        <li>`24` - `i8`</li>
        <li>`25` - `i16`</li>
        <li>`26` - `i32`</li>
        <li>`27` - `i64`</li>
        <li>`28-31` - *reserved*</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td>`0b010` (`2`)</td>
    <td>misc</td>
    <td>
      <ul>
        <li>`0` - `false`</li>
        <li>`1` - `true`</li>
        <li>`2` - `()`</li>
        <li>`3` - `None`</li>
        <li>`4` - `f32`</li>
        <li>`5` - `f64`</li>
        <li>`6-31` - *reserved*</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td>`0b011` (`3`)</td>
    <td>variant (enum)</td>
    <td>
      <ul>
        <li>`0-23` - variant ids `0-23`</li>
        <li>`24` - variant id as `u8`</li>
        <li>`25` - variant id as `u16`</li>
        <li>`26` - variant id as `u32`</li>
        <li>`27` - named variant (see below)</li>
        <li>`28-31` - *reserved*</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td>`0b100` (`4`)</td>
    <td>seq (array/tuple/struct)</td>
    <td>
      <ul>
        <li>`0-23` - length of `0-23`</li>
        <li>`24` - length as `u8`</li>
        <li>`25` - length as `u16`</li>
        <li>`26` - length as `u32`</li>
        <li>`27` - length as `u64` (only on 64-bit machines)</li>
        <li>`28-31` - *reserved*</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td>`0b101` (`5`)</td>
    <td>bytes (string/byte array)</td>
    <td>
      <ul>
        <li>`0-23` - length of `0-23`</li>
        <li>`24` - length as `u8`</li>
        <li>`25` - length as `u16`</li>
        <li>`26` - length as `u32`</li>
        <li>`27` - length as `u64` (only on 64-bit machines)</li>
        <li>`28-31` - *reserved*</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td>`0b110` (`6`)</td>
    <td>map</td>
    <td>
      <ul>
        <li>`0-23` - length of `0-23`</li>
        <li>`24` - length as `u8`</li>
        <li>`25` - length as `u16`</li>
        <li>`26` - length as `u32`</li>
        <li>`27` - length as `u64` (only on 64-bit machines)</li>
        <li>`28-31`</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td>`0b111` (`7`)</td>
    <td>*reserved*</td>
    <td>
      <ul>
        <li>`0-31` - *reserved*</li>
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
