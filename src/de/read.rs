use std::io::{self, Read as IoRead, ErrorKind as IoErrorKind};
use std::cmp;


pub enum Borrowed<'a, 'de: 'a> {
    Transient(&'a [u8]),
    Permanent(&'de [u8]),
    Copied(Vec<u8>),
}

impl<'a, 'de> Borrowed<'a, 'de> {
    #[inline]
    pub fn as_slice(&'a self) -> &'a [u8] {
        match self {
            &Borrowed::Transient(slice) => slice,
            &Borrowed::Permanent(slice) => slice,
            &Borrowed::Copied(ref vec) => vec.as_slice(),
        }
    }

    #[inline]
    pub fn into_vec(&self) -> Vec<u8> {
        match self {
            &Borrowed::Transient(slice) => Vec::from(slice),
            &Borrowed::Permanent(slice) => Vec::from(slice),
            &Borrowed::Copied(ref vec) => vec.clone(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self {
            &Borrowed::Transient(slice) => slice.len(),
            &Borrowed::Permanent(slice) => slice.len(),
            &Borrowed::Copied(ref vec) => vec.len(),
        }
    }
}


pub trait Read<'de> {
    fn next(&mut self) -> Option<u8>;
    fn peek_next(&mut self) -> Option<u8>;
    fn read<'a>(&'a mut self, bytes: usize, flipped: bool) -> Option<Borrowed<'a, 'de>>;
    // For now flipping on peek isn't allowed, because if it was we'd have to copy the bytes
    //   and handle another variant of Borrowed which owns the flipped content
    fn peek<'a>(&'a mut self, bytes: usize) -> Option<Borrowed<'a, 'de>>;
    fn consume(&mut self, bytes: usize) -> Option<usize>;
    fn max_instant_read(&self) -> usize;
    fn finished(&mut self) -> bool;
}


const MAX_BUF_LEN: usize = 1024;

pub(crate) struct BufferedReader<T: io::Read> {
    internal: T,
    buffer: [u8; MAX_BUF_LEN],
    buf_len: usize,
    index: usize,
    finished: bool,
}

impl<T: IoRead> BufferedReader<T> {
    pub fn new(reader: T) -> Self {
        Self {
            internal: reader,
            buffer: [0; 1024],
            buf_len: 0,
            index: 0,
            finished: false,
        }
    }
}

impl<'de, T: IoRead> Read<'de> for BufferedReader<T> {
    fn next(&mut self) -> Option<u8> {
         if self.index >= self.buf_len {
            if self.finished {
                None
            } else {
                loop {
                    match self.internal.read(&mut self.buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                self.finished = true;
                                break None;
                            } else {
                                self.buf_len = bytes_read;

                                self.index = 1;
                                break Some(self.buffer[0]);
                            }
                        }
                        Err(ref error) if error.kind() == IoErrorKind::Interrupted => {}
                        Err(_) => {
                            self.finished = true;
                            break None;
                        }
                    }
                }
            }
        } else {
            let byte = self.buffer[self.index];

            self.index += 1;

            Some(byte)
        }
    }

    fn peek_next(&mut self) -> Option<u8> {
        if self.index >= self.buf_len {
            if self.finished {
                None
            } else {
                loop {
                    match self.internal.read(&mut self.buffer) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                self.finished = true;
                                break None;
                            } else {
                                self.buf_len = bytes_read;

                                break Some(self.buffer[0]);
                            }
                        }
                        Err(ref error) if error.kind() == IoErrorKind::Interrupted => {}
                        Err(_) => {
                            self.finished = true;
                            break None;
                        }
                    }
                }
            }
        } else {
            Some(self.buffer[self.index])
        }
    }

    fn read<'a>(&'a mut self, bytes: usize, flipped: bool) -> Option<Borrowed<'a, 'de>> {
        if bytes > MAX_BUF_LEN {
            panic!("Cannot read more than {} bytes from buffer", MAX_BUF_LEN);
        } else if self.finished {
            if self.index >= self.buf_len {
                // No bytes left to peek
                None
            } else {
                // Return as many bytes as we can
                let consumed = &mut self.buffer[
                    self.index..cmp::min(self.index + bytes, self.buf_len)
                ];

                self.index = cmp::min(self.index + bytes, self.buf_len);

                // We are never going to read these bytes again, so we might as well flip them in
                //   place
                if flipped {
                    consumed.reverse();
                }

                Some(Borrowed::Transient(consumed))
            }
        } else {
            if bytes + self.index >= self.buf_len {
                // Window requested requires us to read more bytes than we have buffered, so read
                //   in more

                let mut new_buffer = [0; MAX_BUF_LEN];

                // Shift the array over so that index is back at the start
                new_buffer[..MAX_BUF_LEN - self.index].copy_from_slice(&self.buffer[self.index..]);

                self.buffer = new_buffer;

                let index = self.index;

                self.buf_len -= index;

                // Try reading new bytes into the buffer
                loop {
                    match self.internal.read(&mut self.buffer[MAX_BUF_LEN - index..]) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                self.finished = true;
                                self.index = self.buf_len;

                                // Return as many bytes as we have left
                                let consumed = &mut self.buffer[..self.buf_len];

                                // We are never going to read these bytes again, so we might as
                                //   well flip them in place
                                if flipped {
                                    consumed.reverse();
                                }

                                break Some(Borrowed::Transient(consumed));
                            } else {
                                self.buf_len += bytes_read;
                                self.index = cmp::min(bytes, self.buf_len);

                                // Return as many bytes as we can
                                let consumed = &mut self.buffer[..cmp::min(bytes, self.buf_len)];

                                // We are never going to read these bytes again, so we might as
                                //   well flip them in place
                                if flipped {
                                    consumed.reverse();
                                }

                                break Some(Borrowed::Transient(consumed));
                            }
                        }
                        Err(ref error) if error.kind() == IoErrorKind::Interrupted => {}
                        Err(_) => {
                            self.finished = true;
                            self.index = self.buf_len;

                            // Return as many bytes as we have left
                            let consumed = &mut self.buffer[..self.buf_len];

                            // We are never going to read these bytes again, so we might as well
                            //   flip them in place
                            if flipped {
                                consumed.reverse();
                            }

                            break Some(Borrowed::Transient(consumed));
                        }
                    }
                }
            } else {
                let orig_index = self.index;
                self.index = cmp::min(self.index + bytes, self.buf_len);

                let consumed = &mut self.buffer[orig_index..self.index];

                // We are never going to read these bytes again, so we might as well flip them in
                //   place
                if flipped {
                    consumed.reverse();
                }

                Some(Borrowed::Transient(consumed))
            }
        }
    }

    fn peek<'a>(&'a mut self, bytes: usize) -> Option<Borrowed<'a, 'de>> {
        if bytes > MAX_BUF_LEN {
            panic!("Cannot read more than {} bytes from buffer", MAX_BUF_LEN);
        } else if self.finished {
            if self.index >= self.buf_len {
                // No bytes left to peek
                None
            } else {
                // Return as many bytes as we can
                Some(Borrowed::Transient(
                    &self.buffer[self.index..cmp::min(self.index +  bytes, self.buf_len)]
                ))
            }
        } else {
            if bytes + self.index >= self.buf_len {
                // Window requested requires us to read more bytes than we have buffered, so read
                //   in more

                let mut new_buffer = [0; MAX_BUF_LEN];

                // Shift the array over so that index is back at the start
                new_buffer[..MAX_BUF_LEN - self.index].copy_from_slice(&self.buffer[self.index..]);

                self.buffer = new_buffer;

                let index = self.index;

                self.buf_len -= index;
                self.index = 0;

                // Try reading new bytes into the buffer
                loop {
                    match self.internal.read(&mut self.buffer[MAX_BUF_LEN - index..]) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                self.finished = true;

                                // Return as many bytes as we have left
                                break Some(Borrowed::Transient(&self.buffer[..self.buf_len]));
                            } else {
                                self.buf_len += bytes_read;

                                // Return as many bytes as we can
                                break Some(Borrowed::Transient(&self.buffer[..cmp::min(bytes, self.buf_len)]));
                            }
                        }
                        Err(ref error) if error.kind() == IoErrorKind::Interrupted => {}
                        Err(_) => {
                            self.finished = true;

                            // Return as many bytes as we have left
                            break Some(Borrowed::Transient(&self.buffer[..self.buf_len]));
                        }
                    }
                }
            } else {
                Some(Borrowed::Transient(
                    &self.buffer[self.index..cmp::min(self.index + bytes, self.buf_len)]
                ))
            }
        }
    }

    fn consume(&mut self, bytes: usize) -> Option<usize> {
        if bytes > MAX_BUF_LEN {
            panic!("Cannot consume more than {} bytes from buffer", MAX_BUF_LEN);
        } else if self.finished {
            if self.index < self.buf_len {
                let consumed = cmp::min(bytes, self.buf_len - self.index);

                self.index += consumed;

                Some(consumed)
            } else {
                None
            }
        } else {
            if bytes + self.index >= self.buf_len {
                // Window requested requires us to read more bytes than we have buffered, so read
                //   in more

                let mut tmp_buffer = [0; MAX_BUF_LEN * 2];

                // The number of bytes which are going to be consumed from the read
                let bytes_preconsumed = bytes + self.index - self.buf_len;

                // Try to read enough bytes to refill the buffer completely even after consumption
                loop {
                    match self.internal.read(&mut tmp_buffer[..bytes_preconsumed + MAX_BUF_LEN]) {
                        Ok(bytes_read) => {
                            if bytes_read < bytes_preconsumed {
                                let consumed = bytes_read + (self.buf_len - self.index);

                                // Consumed all bytes, none left
                                self.finished = true;
                                self.buf_len = 0;

                                break Some(consumed);
                            } else {
                                self.buf_len = bytes_read - bytes_preconsumed;
                                self.buffer.copy_from_slice(&tmp_buffer[
                                    bytes_preconsumed..bytes_read
                                ]);

                                break Some(bytes);
                            }
                        }
                        Err(ref error) if error.kind() == IoErrorKind::Interrupted => {}
                        Err(_) => {
                            let consumed = self.buf_len - self.index;

                            // Consumed all bytes, none left
                            self.finished = true;
                            self.buf_len = 0;

                            break Some(consumed);
                        }
                    }
                }
            } else {
                self.index += bytes;

                Some(bytes)
            }
        }
    }

    #[inline]
    fn max_instant_read(&self) -> usize {
        cmp::min(self.buf_len, MAX_BUF_LEN)
    }

    #[inline]
    fn finished(&mut self) -> bool {
        // Make sure to load the next section if possible
        self.peek_next();

        self.finished && self.index >= self.buf_len
    }
}


pub(crate) struct SliceReader<'de> {
    internal: &'de [u8],
    index: usize,
}

impl<'de> SliceReader<'de> {
    pub fn new<T: AsRef<[u8]> + 'de>(data: &'de T) -> Self {
        Self {
            internal: data.as_ref(),
            index: 0,
        }
    }
}

impl<'de> Read<'de> for SliceReader<'de> {
    fn next(&mut self) -> Option<u8> {
        if self.index >= self.internal.len() {
            None
        } else {
            let byte = self.internal[self.index];

            self.index += 1;

            Some(byte)
        }
    }

    #[inline]
    fn peek_next(&mut self) -> Option<u8> {
        if self.index >= self.internal.len() {
            None
        } else {
            Some(self.internal[self.index])
        }
    }

    fn read<'a>(&'a mut self, bytes: usize, flipped: bool) -> Option<Borrowed<'a, 'de>> {
        if self.index >= self.internal.len() {
            None
        } else {
            let new_index = cmp::min(self.index + bytes, self.internal.len());
            let consumed = &self.internal[self.index..new_index];

            self.index = new_index;

            if flipped {
                let mut consumed = consumed.to_vec();

                consumed.reverse();

                Some(Borrowed::Copied(consumed))
            } else {
                Some(Borrowed::Permanent(consumed))
            }
        }
    }

    fn peek<'a>(&'a mut self, bytes: usize) -> Option<Borrowed<'a, 'de>> {
        if self.index >= self.internal.len() {
            None
        } else {
            Some(
                Borrowed::Permanent(
                    &self.internal[self.index..cmp::min(self.index + bytes, self.internal.len())]
                )
            )
        }
    }

    fn consume(&mut self, bytes: usize) -> Option<usize> {
        if self.index >= self.internal.len() {
            None
        } else if self.index + bytes >= self.internal.len() {
            let consumed = self.internal.len() - self.index;

            self.index = self.internal.len();

            Some(consumed)
        } else {
            self.index += bytes;

            Some(bytes)
        }
    }

    #[inline]
    fn max_instant_read(&self) -> usize {
        self.internal.len() - self.index
    }

    #[inline]
    fn finished(&mut self) -> bool {
        self.index >= self.internal.len()
    }
}
