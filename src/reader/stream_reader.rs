use std::borrow::Cow;
use std::io::BufRead;
use std::{char, str};

use crate::encoding::{self, Encoding};
use crate::error::Result;
use crate::token::XmlSource;

pub struct StreamReader<R> {
    reader: R,
    enc: Option<Encoding>,
    pos: usize,
    shift: usize,
}

impl<R: BufRead> StreamReader<R> {
    pub fn new(reader: R) -> Self {
        StreamReader {
            reader,
            enc: None,
            pos: 0,
            shift: 0,
        }
    }

    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(&self.reader.fill_buf()?[self.shift..])
    }

    fn consume(&mut self, amt: usize) {
        if amt > 0 {
            self.pos += amt + self.shift;
            self.reader.consume(amt + self.shift);
            self.shift = 0;
        }
    }

    fn encoding(&mut self) -> Result<Encoding> {
        if self.enc.is_none() {
            let (enc, bom_len) = encoding::detect(self.fill_buf()?);
            self.consume(bom_len);
            self.enc = Some(enc);
        }

        Ok(*self.enc.get_or_insert_with(Encoding::default))
    }
}

impl<R: BufRead> XmlSource for StreamReader<R> {
    fn pos(&self) -> (usize, usize) {
        (self.pos, self.shift)
    }

    fn shift(&mut self, n: usize) {
        self.shift += n;
    }

    fn unshift(&mut self, n: usize) {
        self.shift -= n.min(self.shift);
    }

    fn is_empty(&mut self) -> Result<bool> {
        Ok(self.fill_buf()?.is_empty())
    }

    fn accept(&mut self, needle: &str) -> Result<Option<usize>> {
        let needle = needle.as_bytes();

        Ok(self.fill_buf()?.starts_with(needle).then_some(needle.len()))
    }

    fn take_until<'a>(&'a mut self, delim: &'a str) -> impl XmlSource {
        TakeUntil::new(self, delim)
    }

    fn read_while<'a>(&mut self, mut predicate: impl FnMut(&char) -> bool) -> Result<Cow<'a, str>> {
        if self.is_empty()? {
            return Ok(Cow::Borrowed(""));
        }
        let enc = self.encoding()?;
        let mut buf = self.fill_buf()?;
        let mut buf_pos = 0;
        let mut result = String::new();

        loop {
            match enc.decode_char(buf) {
                Some((ch, len)) if predicate(&ch) => {
                    result.push(ch);
                    buf = &buf[len..];
                    buf_pos += len;
                }
                None => {
                    self.consume(buf_pos);
                    buf = self.fill_buf()?;
                    buf_pos = 0;
                }
                _ => break,
            }
            if buf.is_empty() {
                break;
            }
        }
        if buf_pos > 0 {
            self.consume(buf_pos);
            self.pos += buf_pos;
        }

        Ok(Cow::Owned(result))
    }
}

pub struct TakeUntil<'a, R> {
    inner: &'a mut R,
    delim: &'a str,
    is_ended: bool,
}

impl<'a, R: BufRead> TakeUntil<'a, StreamReader<R>> {
    fn new(inner: &'a mut StreamReader<R>, delim: &'a str) -> Self {
        TakeUntil {
            inner,
            delim,
            is_ended: false,
        }
    }
}

impl<R: BufRead> XmlSource for TakeUntil<'_, StreamReader<R>> {
    fn pos(&self) -> (usize, usize) {
        self.inner.pos()
    }

    fn shift(&mut self, n: usize) {
        self.inner.shift(n);
    }

    fn unshift(&mut self, n: usize) {
        self.inner.unshift(n);
    }

    fn is_empty(&mut self) -> Result<bool> {
        if self.is_ended || self.inner.is_empty()? {
            Ok(true)
        } else if self.inner.fill_buf()?.starts_with(self.delim.as_bytes()) {
            self.inner.consume(self.delim.len());
            self.is_ended = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn accept(&mut self, needle: &str) -> Result<Option<usize>> {
        self.inner.accept(needle)
    }

    fn take_until<'a>(&'a mut self, delim: &'a str) -> impl XmlSource {
        self.inner.take_until(delim)
    }

    fn read_while<'a>(&mut self, mut predicate: impl FnMut(&char) -> bool) -> Result<Cow<'a, str>> {
        if self.is_empty()? {
            return Ok(Cow::Borrowed(""));
        }
        let enc = self.inner.encoding()?;
        let mut buf = self.inner.fill_buf()?;
        let mut buf_pos = 0;
        let mut result = String::new();

        loop {
            if buf.starts_with(self.delim.as_bytes()) {
                self.inner.consume(self.delim.len());
                self.is_ended = true;
                break;
            }
            match enc.decode_char(buf) {
                Some((ch, len)) if predicate(&ch) => {
                    result.push(ch);
                    buf = &buf[len..];
                    buf_pos += len;
                }
                None => {
                    self.inner.consume(buf_pos);
                    buf = self.inner.fill_buf()?;
                    buf_pos = 0;
                }
                _ => break,
            }
            if buf.is_empty() {
                break;
            }
        }
        if buf_pos > 0 {
            self.inner.consume(buf_pos);
            self.inner.pos += buf_pos;
        }

        Ok(Cow::Owned(result))
    }
}
