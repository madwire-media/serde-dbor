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
        <li><code>28-31</code></li>
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