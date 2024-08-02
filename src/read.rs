use std::borrow::Cow;
use std::cell::OnceCell;
use std::io::BufRead;
use std::{char, str};

use crate::encoding::{self, Encoding};
use crate::error::Result;

pub(crate) trait ReadSource {
    fn is_empty(&mut self) -> Result<bool>;

    fn pos(&self) -> (usize, usize);
    fn shift(&mut self, n: usize);
    fn unshift(&mut self, n: usize);

    fn encoding(&mut self) -> Result<Encoding>;

    fn accept(&mut self, needle: &str) -> Result<Option<usize>>;

    fn read_while<'a>(
        &mut self,
        predicate: impl FnMut(char) -> bool,
        delim: &str,
    ) -> Result<Cow<'a, str>>;
}

pub struct ReaderState {
    pos: usize,
    offset: usize,
    enc: OnceCell<Encoding>,
}

impl ReaderState {
    pub fn new() -> Self {
        Self {
            enc: OnceCell::new(),
            pos: 0,
            offset: 0,
        }
    }
}

impl Default for ReaderState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SourceReader<T> {
    reader: T,
    state: ReaderState,
}

impl<T> SourceReader<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            state: ReaderState::new(),
        }
    }
}

impl<T: BufRead> SourceReader<T> {
    fn buf(&mut self) -> Result<&[u8]> {
        let offset = self.state.offset;
        Ok(&self.reader.fill_buf()?[offset..])
    }

    fn advance(&mut self, n: usize) {
        let n = self.state.offset + n;

        self.reader.consume(n);
        self.state.pos += n;
        self.state.offset = 0;
    }
}

impl<T: BufRead> ReadSource for SourceReader<T> {
    fn is_empty(&mut self) -> Result<bool> {
        Ok(self.buf()?.is_empty())
    }

    fn pos(&self) -> (usize, usize) {
        (self.state.pos, self.state.offset)
    }

    fn shift(&mut self, n: usize) {
        self.state.offset += n;
    }

    fn unshift(&mut self, n: usize) {
        self.state.offset -= n.min(self.state.offset);
    }

    fn encoding(&mut self) -> Result<Encoding> {
        Ok(if let Some(enc) = self.state.enc.get() {
            enc
        } else if self.state.pos == 0 {
            let (enc, bom_len) = encoding::detect(self.buf()?);
            self.advance(bom_len);
            self.state.enc.get_or_init(|| enc)
        } else {
            self.state.enc.get_or_init(Encoding::default)
        }
        .to_owned())
    }

    fn accept(&mut self, needle: &str) -> Result<Option<usize>> {
        let _enc = self.encoding()?;
        let needle = needle.as_bytes();

        Ok(self.buf()?.starts_with(needle).then_some(needle.len()))
    }

    fn read_while<'a>(
        &mut self,
        mut predicate: impl FnMut(char) -> bool,
        delim: &str,
    ) -> Result<Cow<'a, str>> {
        if self.is_empty()? {
            return Ok(Cow::Borrowed(""));
        }
        let enc = self.encoding()?;
        let delim = delim.as_bytes();
        let mut buf = self.buf()?;
        let mut byte_read_count = 0;
        let mut result = String::new();

        loop {
            if !delim.is_empty() {
                if buf.len() < delim.len() {
                    self.advance(byte_read_count);
                    byte_read_count = 0;
                    buf = self.buf()?;
                    continue;
                } else if buf.starts_with(delim) {
                    byte_read_count += delim.len();
                    break;
                }
            }
            match enc.next_char(buf) {
                Some((ch, rest)) if predicate(ch) => {
                    byte_read_count += buf.len() - rest.len();
                    buf = rest;
                    result.push(ch);
                }
                None => {
                    self.advance(byte_read_count);
                    byte_read_count = 0;
                    buf = self.buf()?;
                }
                _ => break,
            }
            if buf.is_empty() {
                break;
            }
        }
        if byte_read_count > 0 {
            self.advance(byte_read_count);
        }

        Ok(Cow::Owned(result))
    }
}

impl<'a> From<&'a [u8]> for SourceReader<&'a [u8]> {
    fn from(value: &'a [u8]) -> Self {
        SourceReader::new(value)
    }
}
