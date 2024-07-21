use std::borrow::Cow;
use std::io::BufRead;
use std::{char, str};

use crate::encoding::{self, Encoding};
use crate::error::Result;

pub(crate) trait ReadSource {
    fn buf(&mut self) -> Result<&[u8]>;

    fn advance(&mut self, n: usize);

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
    enc: Option<Encoding>,
    pos: usize,
    offset: usize,
}

impl ReaderState {
    pub fn new() -> Self {
        Self {
            enc: None,
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

pub struct SrcReader<T> {
    reader: T,
    state: ReaderState,
}

impl<T> SrcReader<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            state: ReaderState::new(),
        }
    }
}

impl<T: BufRead> ReadSource for SrcReader<T> {
    fn buf(&mut self) -> Result<&[u8]> {
        let offset = self.state.offset;
        Ok(&self.reader.fill_buf()?[offset..])
    }

    fn advance(&mut self, n: usize) {
        if n > 0 {
            let n = self.state.offset + n;

            self.reader.consume(n);
            self.state.pos += n;
            self.state.offset = 0;
        }
    }

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
        if self.state.enc.is_none() && self.state.pos == 0 {
            let (enc, bom_len) = encoding::detect(self.buf()?);
            self.advance(bom_len);
            self.state.enc = Some(enc);
        }

        Ok(*self.state.enc.get_or_insert_with(Encoding::default))
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
        let has_delim = !delim.is_empty();
        let mut buf = self.buf()?;
        let mut buf_pos = 0;
        let mut result = String::new();

        loop {
            if has_delim {
                if buf.len() < delim.len() {
                    self.advance(buf_pos);
                    buf = self.buf()?;
                    buf_pos = 0;
                    continue;
                } else if buf.starts_with(delim) {
                    buf_pos += delim.len();
                    break;
                }
            }
            match enc.decode_char(buf) {
                Some((ch, len)) if predicate(ch) => {
                    result.push(ch);
                    buf = &buf[len..];
                    buf_pos += len;
                }
                None => {
                    self.advance(buf_pos);
                    buf = self.buf()?;
                    buf_pos = 0;
                }
                _ => break,
            }
            if buf.is_empty() {
                break;
            }
        }
        if buf_pos > 0 {
            self.advance(buf_pos);
            self.state.pos += buf_pos;
        }

        Ok(Cow::Owned(result))
    }
}

impl<'a> From<&'a [u8]> for SrcReader<&'a [u8]> {
    fn from(value: &'a [u8]) -> Self {
        SrcReader::new(value)
    }
}
