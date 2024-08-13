use std::{
    io::{BufRead, Result},
    ops::{Deref, DerefMut},
    str,
};

use encoding_rs::{CoderResult, Decoder, Encoding, UTF_8};

pub struct DecBuffer<const S: usize = 8192> {
    buf: [u8; S],
    pos: usize,
    filled: usize,
    decoder: Decoder,
}

impl<const S: usize> DecBuffer<S> {
    pub fn new(enc: &str) -> Self {
        Self {
            buf: [0; S],
            pos: 0,
            filled: 0,
            decoder: Encoding::for_label(enc.as_bytes())
                .unwrap_or(UTF_8)
                .new_decoder(),
        }
    }

    pub fn encoding(&self) -> &str {
        self.decoder.encoding().name()
    }

    pub fn discard(&mut self) {
        self.pos = 0;
        self.filled = 0;
    }

    pub fn consume(&mut self, amt: usize) {
        self.pos += amt.min(self.filled - self.pos);
    }

    pub fn unconsume(&mut self, amt: usize) {
        self.pos -= amt.min(self.pos);
    }

    pub fn fill(&mut self, mut reader: impl BufRead) -> Result<&str> {
        let decoder = &mut self.decoder;
        let mut is_last = false;
        let mut _total_had_errors = false;

        loop {
            let unfilled = unsafe { str::from_utf8_unchecked_mut(&mut self.buf[self.filled..]) };

            let (result, read, written, had_errors) =
                decoder.decode_to_str(reader.fill_buf()?, unfilled, is_last);

            self.filled += written;
            reader.consume(read);
            _total_had_errors |= had_errors;

            match result {
                CoderResult::InputEmpty => {
                    if is_last {
                        break;
                    }
                    is_last = read == 0;
                }
                CoderResult::OutputFull => break,
            }
        }

        Ok(self)
    }
}

impl<const S: usize> Deref for DecBuffer<S> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { str::from_utf8_unchecked(&self.buf[self.pos..self.filled]) }
    }
}

impl<const S: usize> DerefMut for DecBuffer<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { str::from_utf8_unchecked_mut(&mut self.buf[self.pos..self.filled]) }
    }
}
