use crate::encoding::DecBuffer;
use crate::error::Result;
use std::borrow::Cow;
use std::io::BufRead;
use std::{char, str};

pub(crate) trait ReadSource {
    fn _encoding(&mut self) -> &str;
    fn is_empty(&mut self) -> Result<bool>;

    fn pos(&self) -> usize;
    fn go_back(&mut self, n: usize) -> bool;

    fn skip_next(&mut self, slice: &str) -> Result<bool>;

    fn read_while<'a>(
        &mut self,
        predicate: impl FnMut(char) -> bool,
        delim: &str,
    ) -> Result<Cow<'a, str>>;
}

pub struct ReaderState {
    pos: usize,
    skipped: usize,
}

impl ReaderState {
    fn new() -> Self {
        Self { pos: 0, skipped: 0 }
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
    buf: DecBuffer,
}

impl<T> SourceReader<T> {
    pub fn new(reader: T, enc: &str) -> Self {
        Self {
            reader,
            state: ReaderState::new(),
            buf: DecBuffer::new(enc),
        }
    }
}

impl<T: BufRead> SourceReader<T> {
    fn buf(&mut self) -> Result<&str> {
        if self.buf.is_empty() {
            return self.fill_buf();
        }
        Ok(&self.buf[self.state.skipped..])
    }

    fn fill_buf(&mut self) -> Result<&str> {
        self.advance(0);

        if self.reader.fill_buf()?.is_empty() {
            return Ok("");
        }
        Ok(self.buf.fill(&mut self.reader)?)
    }

    fn advance(&mut self, n: usize) {
        let n = self.state.skipped + n;

        self.state.pos += n;
        self.state.skipped = 0;
        self.buf.consume(n);
    }
}

impl<T: BufRead> ReadSource for SourceReader<T> {
    fn _encoding(&mut self) -> &str {
        self.buf.encoding()
    }

    fn is_empty(&mut self) -> Result<bool> {
        Ok(self.buf()?.is_empty())
    }

    fn pos(&self) -> usize {
        self.state.pos + self.state.skipped
    }

    fn go_back(&mut self, n: usize) -> bool {
        if n <= self.state.skipped {
            self.state.skipped -= n;
            return true;
        }
        false
    }

    fn skip_next(&mut self, slice: &str) -> Result<bool> {
        if self.buf()?.starts_with(slice) {
            self.state.skipped += slice.len();
            return Ok(true);
        }
        Ok(false)
    }

    fn read_while<'r>(
        &mut self,
        mut predicate: impl FnMut(char) -> bool,
        delim: &str,
    ) -> Result<Cow<'r, str>> {
        let mut buf = self.buf()?;
        let mut result = String::new();
        let mut delim_reached = false;
        let mut check_failed = false;

        loop {
            if buf.is_empty() {
                break;
            }
            let mut total_read = 0;
            let mut too_small = false;

            if delim.is_empty() {
                buf.char_indices()
                    .map(|(pos, ch)| {
                        total_read = pos;
                        ch
                    })
                    .take_while(|&ch| {
                        check_failed = !predicate(ch);
                        !check_failed
                    })
                    .for_each(|_| ());
            } else {
                buf.char_indices()
                    .map(|(pos, ch)| {
                        total_read = pos;
                        (&buf[pos..], ch)
                    })
                    .map_while(|(buf, ch)| {
                        too_small = buf.len() < delim.len();
                        delim_reached = buf.starts_with(delim);
                        (!too_small && !delim_reached).then_some(ch)
                    })
                    .take_while(|&ch| {
                        check_failed = !predicate(ch);
                        !check_failed
                    })
                    .for_each(|_| ());
            };

            if too_small || delim_reached || check_failed {
                if total_read > 0 {
                    result.push_str(&buf[..total_read]);
                    self.advance(total_read);
                }
                if too_small {
                    buf = self.fill_buf()?;
                    continue;
                }
                if delim_reached {
                    self.advance(delim.len());
                }
                break;
            }
            result.push_str(buf);
            total_read = buf.len();
            self.advance(total_read);

            buf = self.fill_buf()?;
        }

        Ok(Cow::Owned(result))
    }
}
