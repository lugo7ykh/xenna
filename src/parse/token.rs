mod rules;

use std::borrow::Cow;

use crate::error::Result;
use crate::parse::Parse;

use super::ParseSource;

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

pub fn opt_parse_punct<'p>(
    input: &mut impl ParseSource,
    punct: &'p str,
) -> Result<Option<&'p str>> {
    input.opt_parse_punct(punct)
}

pub fn opt_parse_lit<'l>(
    input: &mut impl ParseSource,
    rule: impl FnMut(char) -> bool,
    delim: Option<&str>,
) -> Result<Option<Cow<'l, str>>> {
    input.opt_parse_lit(rule, delim)
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

        impl $crate::parse::Parse for $name {
            fn parse(input: &mut impl $crate::parse::ParseSource) -> $crate::error::Result<Self> {
                use  $crate::token::Token;

                Self::opt_parse(input)?.ok_or_else(
                    || $crate::error::SyntaxError::MismatchedToken($name::display()).into()
                )
            }

            fn opt_parse(input: &mut impl $crate::parse::ParseSource) -> $crate::error::Result<Option<Self>> {
                use  $crate::token::Punctuation;

                $crate::token::opt_parse_punct(input, $name::PUNCT).map(|r| r.map(|_| Self))
            }
        }
    )+};
}

macro_rules! define_delimiters {
    ($( $name:ident $start:literal .. $end:literal ),+ $(,)?) => {
        pub mod end_delim {
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
            fn parse(input: &mut impl $crate::parse::ParseSource) -> $crate::error::Result<Self> {
                use  $crate::token::Token;

                Self::opt_parse(input)?.ok_or_else(
                    || $crate::error::SyntaxError::MismatchedToken($name::display()).into()
                )
            }

            fn opt_parse(input: &mut impl $crate::parse::ParseSource) -> $crate::error::Result<Option<Self>> {
                let delim = None $( .or($(if input.opt_parse_punct($delim::PUNCT)?.is_some() {
                        Some(<$delim as Delimiter>::End::PUNCT)
                    } )else+ else {
                        return Ok(None)
                    })
                )?;

                $crate::token::opt_parse_lit(input, $rule, delim).map(|r| r.map(|lit| Self(lit)))
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
