pub mod token;

use std::borrow::Cow;

use crate::error::{Error, Result, SyntaxError};

use crate::read::{ReadSource, SourceReader};
use token::{Delimiter, Punctuation};

pub type Parser<T> = SourceReader<T>;

pub trait Parse: Sized {
    fn parse(input: &mut impl ParseSource) -> Result<Self>;

    fn opt_parse(input: &mut impl ParseSource) -> Result<Option<Self>> {
        input.default_opt_parse::<Self>()
    }
}

trait PrivParseSource {
    fn opt_parse_punct<'p>(&mut self, punct: &'p str) -> Result<Option<&'p str>>;

    fn opt_parse_lit<'l>(
        &mut self,
        rule: impl FnMut(char) -> bool,
        delim: Option<&str>,
    ) -> Result<Option<Cow<'l, str>>>;

    fn default_opt_parse<P: Parse>(&mut self) -> Result<Option<P>>;
}

#[allow(private_bounds)]
pub trait ParseSource: PrivParseSource + Sized {
    fn is_empty(&mut self) -> Result<bool>;

    fn parse<P: Parse>(&mut self) -> Result<P> {
        P::parse(self)
    }

    fn opt_parse<P: Parse>(&mut self) -> Result<Option<P>> {
        if self.is_empty()? {
            return Ok(None);
        }
        P::opt_parse(self)
    }

    fn delimited<D: Delimiter>(&mut self) -> Result<impl ParseSource> {
        D::parse(self)?;
        Ok(Delimited::new(self, D::End::PUNCT))
    }
}

impl<T: ReadSource> PrivParseSource for T {
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

    fn default_opt_parse<P: Parse>(&mut self) -> Result<Option<P>> {
        let (pos_before, offset_before) = self.pos();

        let result = P::parse(self);
        let (pos, offset) = self.pos();

        match result {
            Err(Error::Syntax(SyntaxError::UnexpectedToken(_))) if pos == pos_before => {
                self.unshift(offset - offset_before);
                Ok(None)
            }
            _ => result.map(Some),
        }
    }
}

impl<T: ReadSource> ParseSource for T {
    fn is_empty(&mut self) -> Result<bool> {
        ReadSource::is_empty(self)
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

impl<'a, T: PrivParseSource> PrivParseSource for Delimited<'a, T> {
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

    fn default_opt_parse<P: Parse>(&mut self) -> Result<Option<P>> {
        self.inner.default_opt_parse()
    }
}

impl<'a, T: ParseSource> ParseSource for Delimited<'a, T> {
    fn is_empty(&mut self) -> Result<bool> {
        self.is_ended |= self.inner.is_empty()?;
        self.is_ended |= self.inner.opt_parse_punct(&self.delim)?.is_some();

        Ok(self.is_ended)
    }

    fn delimited<D: Delimiter>(&mut self) -> Result<impl ParseSource> {
        self.inner.delimited::<D>()
    }
}
