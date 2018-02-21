use std::io::{Write as IoWrite, ErrorKind as IoErrorKind};

use error::*;


pub trait Write {
    type Output;

    fn put_byte(&mut self, byte: u8) -> Result<()>;
    fn put_bytes(&mut self, bytes: &[u8], flipped: bool) -> Result<()>;
    fn finish(self) -> Result<Self::Output>;
}

const MAX_BUF_LEN: usize = 1024;

pub(crate) struct IoWriter<T: IoWrite> {
    internal: T,
    buffer: [u8; MAX_BUF_LEN],
    buf_len: usize,
}

impl<T: IoWrite> IoWriter<T> {
    pub fn new(writer: T) -> Self {
        Self {
            internal: writer,
            buffer: [0; 1024],
            buf_len: 0
        }
    }
}

impl<T: IoWrite> Write for IoWriter<T> {
    type Output = T;

    fn put_byte(&mut self, byte: u8) -> Result<()> {
        if self.buf_len < MAX_BUF_LEN {
            self.buffer[self.buf_len] = byte;
            self.buf_len += 1;
            Ok(())
        } else {
            // Flush buffer
            let mut bytes_written = 0;

            while bytes_written < self.buf_len {
                bytes_written += self.internal.write(&self.buffer[bytes_written..self.buf_len])
                    .or_else(|io_error| match io_error.kind() {
                        IoErrorKind::Interrupted => Ok(0),
                        _ => Err(Error::Io(io_error))
                    })?;
            }

            // Write new byte to buffer
            self.buf_len = 1;
            self.buffer[0] = byte;

            Ok(())
        }
    }

    fn put_bytes(&mut self, bytes_ref: &[u8], flipped: bool) -> Result<()> {
        let mut bytes_copy;
        let mut bytes = bytes_ref;

        if flipped {
            bytes_copy = Vec::from(bytes);

            bytes_copy.reverse();

            bytes = &bytes_copy[..];
        }

        if self.buf_len + bytes.len() <= MAX_BUF_LEN {
            self.buffer[self.buf_len..self.buf_len + bytes.len()].copy_from_slice(bytes);
            self.buf_len += bytes.len();
            Ok(())
        } else {
            // Flush buffer
            let mut bytes_written = 0;

            while bytes_written < self.buf_len {
                bytes_written += self.internal.write(&self.buffer[bytes_written..self.buf_len])
                    .or_else(|io_error| match io_error.kind() {
                        IoErrorKind::Interrupted => Ok(0),
                        _ => Err(Error::Io(io_error))
                    })?;
            }

            // Write as many bytes as possible while the remaining bytes don't fit in the buffer
            bytes_written = 0;

            while bytes.len() - bytes_written >= MAX_BUF_LEN {
                bytes_written += self.internal.write(&bytes[bytes_written..])
                    .or_else(|io_error| match io_error.kind() {
                        IoErrorKind::Interrupted => Ok(0),
                        _ => Err(Error::Io(io_error))
                    })?;
            }

            // Write remaining new data to buffer
            self.buf_len = bytes.len() - bytes_written;
            self.buffer[..bytes.len() - bytes_written].copy_from_slice(&bytes[bytes_written..]);

            Ok(())
        }
    }

    fn finish(mut self) -> Result<Self::Output> {
        let mut bytes_written = 0;

        while bytes_written < self.buf_len {
            bytes_written += self.internal.write(&self.buffer[bytes_written..self.buf_len])
                .or_else(|io_error| match io_error.kind() {
                    IoErrorKind::Interrupted => Ok(0),
                    _ => Err(Error::Io(io_error))
                })?;
        }

        Ok(self.internal)
    }
}


pub(crate) struct VecWriter {
    internal: Vec<u8>,
}

impl VecWriter {
    pub fn new() -> Self {
        Self {
            internal: Vec::new(),
        }
    }
}

impl Write for VecWriter {
    type Output = Vec<u8>;

    #[inline]
    fn put_byte(&mut self, byte: u8) -> Result<()> {
        self.internal.push(byte);

        Ok(())
    }

    fn put_bytes(&mut self, bytes: &[u8], flipped: bool) -> Result<()> {
        if flipped {
            let mut bytes_copy = Vec::from(bytes);

            bytes_copy.reverse();

            self.internal.append(&mut bytes_copy);
        } else {
            self.internal.extend_from_slice(bytes);
        }

        Ok(())
    }

    #[inline]
    fn finish(self) -> Result<Self::Output> {
        Ok(self.internal)
    }
}
