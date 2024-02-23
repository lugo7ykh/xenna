use std::borrow::Cow;
use std::io::BufRead;
use std::ops::Deref;
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
        let shift = self.shift;
        Ok(&self.reader.fill_buf()?[shift..])
    }

    fn consume(&mut self, amt: usize) {
        if amt == 0 {
            return;
        }
        let shift = self.shift;

        self.shift = 0;
        self.pos += amt + shift;
        self.reader.consume(amt + shift)
    }

    fn encoding(&mut self) -> Result<Encoding> {
        if self.enc.is_none() && self.pos == 0 && self.shift == 0 {
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
            if buf.is_empty() {
                break;
            }

            match enc.decode_char(&buf[buf_pos..]) {
                Some((ch, len)) if predicate(&ch) => {
                    result.push(ch);
                    buf_pos += len;
                    continue;
                }
                None => {
                    self.consume(buf_pos);
                    buf = self.fill_buf()?;
                    buf_pos = 0;
                }
                _ => break,
            }
        }
        if buf_pos > 0 {
            self.consume(buf_pos);
        }

        Ok(Cow::Owned(result))
    }
}

struct Delim<'a> {
    raw: &'a str,
    encoded: &'a [u8],
}

impl<'a> Delim<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            raw,
            encoded: raw.as_bytes(),
        }
    }
}

impl<'a> Deref for Delim<'a> {
    type Target = &'a [u8];
    fn deref(&self) -> &Self::Target {
        &self.encoded
    }
}

pub struct TakeUntil<'a, R> {
    inner: &'a mut R,
    delim: Delim<'a>,
    is_ended: bool,
}

impl<'a, R: BufRead> TakeUntil<'a, StreamReader<R>> {
    fn new(inner: &'a mut StreamReader<R>, delim: &'a str) -> Self {
        TakeUntil {
            inner,
            delim: Delim::new(delim),
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
        let mut is_empty = self.is_ended || self.inner.is_empty()?;

        if !is_empty {
            self.is_ended = self.inner.fill_buf()?.starts_with(&self.delim);

            if self.is_ended {
                self.inner.consume(self.delim.len());
                is_empty = self.is_ended;
            }
        }
        Ok(is_empty)
    }

    fn accept(&mut self, needle: &str) -> Result<Option<usize>> {
        if self.is_empty()? {
            return Ok(None);
        }
        let needle = needle.as_bytes();

        Ok(self
            .inner
            .fill_buf()?
            .starts_with(needle)
            .then_some(needle.len()))
    }

    fn take_until<'a>(&'a mut self, delim: &'a str) -> impl XmlSource {
        self.inner.take_until(delim)
    }

    fn read_while<'a>(&mut self, mut predicate: impl FnMut(&char) -> bool) -> Result<Cow<'a, str>> {
        if self.is_empty()? {
            return Ok(Cow::Borrowed(""));
        }
        let delim_len = self.delim.len();
        let mut taken_delim_bytes = 0;
        let mut matched_delim_bytes = 0;

        let mut result = self.inner.read_while(|ch| {
            if self.delim.raw[matched_delim_bytes..].starts_with(*ch) {
                taken_delim_bytes = matched_delim_bytes;
                matched_delim_bytes += ch.len_utf8();
            } else {
                matched_delim_bytes = 0;
                taken_delim_bytes = 0;
            }

            matched_delim_bytes < delim_len && predicate(ch)
        })?;

        self.is_ended = self
            .inner
            .fill_buf()?
            .starts_with(&self.delim[taken_delim_bytes..]);

        if self.is_ended {
            let remaining_delim_bytes = delim_len - taken_delim_bytes;

            if remaining_delim_bytes > 0 {
                self.inner.consume(remaining_delim_bytes);
            }
            if taken_delim_bytes > 0 {
                if let Cow::Owned(ref mut r) = result {
                    r.truncate(r.len() - taken_delim_bytes);
                }
            }
        }
        Ok(result)
    }
}
