pub mod token;

use std::borrow::Cow;

use crate::error::{Error, Result, SyntaxError};

use crate::read::{ReadSource, SrcReader};
use token::{Delimiter, ParseToken, Punctuation};

pub trait Parse: Sized {
    fn parse(input: &mut impl ParseSource) -> Result<Self>;
}

#[allow(private_bounds)]
pub trait ParseSource: ParseToken + Sized {
    fn is_empty(&mut self) -> Result<bool>;

    fn parse<T: Parse>(&mut self) -> Result<T> {
        T::parse(self)
    }

    fn opt_parse<P: Parse>(&mut self) -> Result<Option<P>>;

    fn delimited<D: Delimiter>(&mut self) -> Result<impl ParseSource> {
        D::parse(self)?;
        Ok(Delimited::new(self, D::End::PUNCT))
    }
}

pub type Parser<T> = SrcReader<T>;

impl<T: ReadSource> ParseToken for T {
    fn opt_parse_punct<'p>(&mut self, punct: &'p str) -> Result<Option<&'p str>> {
        Ok(self.accept(punct)?.map(|n| {
            self.shift(n);
            punct
        }))
    }

    fn opt_parse_lit<'a>(
        &mut self,
        rule: impl FnMut(char) -> bool,
        delim: Option<&str>,
    ) -> Result<Option<Cow<'a, str>>> {
        let token = self.read_while(rule, delim.unwrap_or_default())?;

        if token.is_empty() {
            return Ok(None);
        }
        Ok(Some(token))
    }
}

impl<T: ReadSource> ParseSource for T {
    fn is_empty(&mut self) -> Result<bool> {
        ReadSource::is_empty(self)
    }

    fn opt_parse<P: Parse>(&mut self) -> Result<Option<P>> {
        let (pos_before, offset_before) = self.pos();

        let result = P::parse(self);
        let (pos, offset) = self.pos();

        result.map(Some).or_else(|e| match e {
            Error::Syntax(SyntaxError::UnexpectedToken(_)) if pos == pos_before => {
                self.unshift(offset - offset_before);
                Ok(None)
            }
            _ => Err(e),
        })
    }
}

pub struct Delimited<'a, T> {
    inner: &'a mut T,
    delim: Cow<'static, str>,
    is_ended: bool,
}

impl<'a, T> Delimited<'a, T> {
    fn new(inner: &'a mut T, delim: &'static str) -> Self {
        Self {
            inner,
            delim: delim.into(),
            is_ended: false,
        }
    }
}

impl<'a, T: ParseToken> ParseToken for Delimited<'a, T> {
    fn opt_parse_punct<'p>(&mut self, punct: &'p str) -> Result<Option<&'p str>> {
        self.inner.opt_parse_punct(punct)
    }

    fn opt_parse_lit<'l>(
        &mut self,
        rule: impl FnMut(char) -> bool,
        delim: Option<&str>,
    ) -> Result<Option<Cow<'l, str>>> {
        self.inner.opt_parse_lit(rule, delim)
    }
}

impl<'a, T: ParseSource> ParseSource for Delimited<'a, T> {
    fn is_empty(&mut self) -> Result<bool> {
        self.is_ended |= self.inner.is_empty()?;
        self.is_ended |= self.inner.opt_parse_punct(&self.delim)?.is_some();

        Ok(self.is_ended)
    }

    fn opt_parse<P: Parse>(&mut self) -> Result<Option<P>> {
        if self.is_empty()? {
            return Ok(None);
        }
        self.inner.opt_parse::<P>()
    }

    fn delimited<D: Delimiter>(&mut self) -> Result<impl ParseSource> {
        self.inner.parse::<D>()?;

        let mut source = Delimited::new(self.inner, D::End::PUNCT);
        source.is_ended = self.is_ended;

        Ok(source)
    }
}
