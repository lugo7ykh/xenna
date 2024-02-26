mod rules;

use std::{borrow::Cow, fmt::Debug};

use crate::error::{Error, Result, SyntaxError};

pub trait Parse: Sized + Debug {
    fn parse(input: &mut impl XmlSource) -> Result<Self>;

    fn try_parse(input: &mut impl XmlSource) -> Result<Option<Self>> {
        let (pos_before_try, shift_before_try) = input.pos();

        Self::parse(input).map(|r| Some(r)).or_else(|e| match e {
            Error::Syntax(SyntaxError::UnexpectedToken(_)) if input.pos().0 == pos_before_try => {
                input.unshift(input.pos().1 - shift_before_try);
                Ok(None)
            }
            _ => Err(e),
        })
    }
}

pub trait Token: Parse {
    fn display() -> &'static str;
}

pub trait Punctuation: Token {
    const PUNCT: &'static str;
}

pub trait Delimiter: Punctuation {
    type End: Punctuation;
}

pub trait Literal: Token {
    fn value(&self) -> &str;

    fn is(&self, value: &str) -> bool {
        value == self.value()
    }
}

pub trait XmlSource: Sized {
    fn pos(&self) -> (usize, usize);
    fn shift(&mut self, step: usize);
    fn unshift(&mut self, step: usize);
    fn is_empty(&mut self) -> Result<bool>;

    fn accept(&mut self, needle: &str) -> Result<Option<usize>>;
    fn take_until<'a>(&'a mut self, delim: &'a str) -> impl XmlSource;
    fn read_while<'a>(&mut self, predicate: impl FnMut(char) -> bool) -> Result<Cow<'a, str>>;

    fn parse<T: Parse>(&mut self) -> Result<T> {
        T::parse(self)
    }

    fn try_parse<T: Parse>(&mut self) -> Result<Option<T>> {
        T::try_parse(self)
    }

    fn parse_punct(&mut self, punct: &'static str) -> Result<&'static str> {
        self.try_parse_punct(punct)?
            .ok_or_else(|| SyntaxError::UnexpectedToken(punct).into())
    }

    fn try_parse_punct(&mut self, punct: &'static str) -> Result<Option<&'static str>> {
        try_parse_punct(self, punct)
    }

    fn delimited<D: Delimiter>(&mut self) -> Result<impl XmlSource> {
        D::parse(self)?;
        let content = self.take_until(D::End::PUNCT);

        Ok(content)
    }
}

pub fn try_parse_punct<'a>(input: &mut impl XmlSource, punct: &'a str) -> Result<Option<&'a str>> {
    Ok(input.accept(punct)?.map(|n| {
        input.shift(n);
        punct
    }))
}

pub fn try_parse_lit<'a>(
    input: &mut impl XmlSource,
    rule: impl FnMut(char) -> bool,
) -> Result<Option<Cow<'a, str>>> {
    let token = input.read_while(rule)?;

    if token.is_empty() {
        return Ok(None);
    }
    Ok(Some(token))
}

#[macro_export]
macro_rules! define_punctuation {
    ($( $name:ident $punct:literal ),+ $(,)?) => {$(
        #[derive(Debug)]
        pub struct $name;

        impl $crate::token::Token for $name {
            fn display() -> &'static str {
                concat!("`", $punct, "`")
            }
        }

        impl $crate::token::Punctuation for $name {
            const PUNCT: &'static str = $punct;
        }

        impl $crate::token::Parse for $name {
            fn parse(input: &mut impl $crate::token::XmlSource) -> $crate::error::Result<Self> {
                use  $crate::token::Token;
                Self::try_parse(input)?.ok_or_else(
                    || $crate::error::SyntaxError::UnexpectedToken($name::display()).into()
                )
            }

            fn try_parse(
                input: &mut impl $crate::token::XmlSource
            ) -> $crate::error::Result<Option<Self>> {
                use  $crate::token::Punctuation;
                $crate::token::try_parse_punct(input, $name::PUNCT).map(|r| r.map(|_| Self))
            }
        }
    )+};
}

macro_rules! define_delimiters {
    ($( $name:ident $start:literal .. $end:literal ),+ $(,)?) => {
        pub mod end_delim {
            use super::*;
            define_punctuation! { $( $name $end),+ }
        }

        define_punctuation! { $( $name $start),+ }
        $(
            impl Delimiter for $name {
                type End = end_delim::$name;
            }
        )+
    };
}

#[macro_export]
macro_rules! define_literals {
    ($($name:ident by { $rule:expr } $( in $( $delim:ident )|+ )?),+ $(,)?) => {$(
        #[derive(PartialEq, Clone, Debug)]
        pub struct $name<'a>(std::borrow::Cow<'a, str>);

        impl<'a> $name<'a> {
            pub fn new<T: Into<Cow<'a, str>>>(value: T) -> Self {
                Self(value.into())
            }
        }

        impl $crate::token::Token for $name<'_> {
            fn display() -> &'static str {
                stringify!($name)
            }
        }

        impl<'a> $crate::token::Literal for $name<'a> {
            fn value(&self) -> &str {
                self.0.as_ref()
            }
        }

        impl $crate::token::Parse for $name<'_> {
            fn parse(input: &mut impl $crate::token::XmlSource) -> $crate::error::Result<Self> {
                use  $crate::token::Token;
                Self::try_parse(input)?.ok_or_else(
                    || $crate::error::SyntaxError::UnexpectedToken($name::display()).into()
                )
            }

            fn try_parse(
                input: &mut impl $crate::token::XmlSource
            ) -> $crate::error::Result<Option<Self>> {
                $(
                    let input = &mut $(if try_parse_punct(input, $delim::PUNCT)?.is_some() {
                        input.take_until(<$delim as Delimiter>::End::PUNCT)
                    } )else+ else {
                        return Ok(None);
                    };
                )?

                $crate::token::try_parse_lit(input, $rule).map(|r| r.map(|lit| Self(lit)))
            }
        }
    )+};
}

define_punctuation! {
    Eq "=",
    Colon ":",
}

define_delimiters! {
    XmlDecl "<?xml" .. "?>",
    Pi "<?" .. "?>",
    Comm "<!--" .. "-->",
    STag "<" .. ">",
    ETag "</" .. ">",
    CData "<![CDATA[" .. "]]>",
    Reference "&" .. ";",
    SQuote "'" .. "'",
    DQuote "\"" .. "\"",
}

define_literals! {
    S by { rules::accept_as_white_space },
    Comment by { rules::accept_as_comment() } in Comm,
    Name by { rules::accept_as_name() },
    AttValue by { rules::accept_as_att_value } in DQuote | SQuote,
    Text by { rules::accept_as_char_data() },
}

#[macro_export]
macro_rules! Token {
    [=] => { $crate::token::Eq };
    [:] => { $crate::token::Colon };
}
