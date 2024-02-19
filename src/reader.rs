pub mod stream_reader;

use std::str;

use crate::error::Result;
use crate::token::{self, AttValue, Comment, Literal, Name, Parse, Text, XmlSource, S};
use crate::Token;

use self::stream_reader::StreamReader;

#[derive(PartialEq, Debug)]
struct Eq;

impl Parse for Eq {
    fn parse(input: &mut impl XmlSource) -> Result<Self> {
        input.try_parse::<S>()?;
        input.parse::<Token![=]>()?;
        input.try_parse::<S>()?;

        Ok(Self)
    }
}

#[derive(PartialEq, Debug)]
pub struct Attribute<'a>(Name<'a>, AttValue<'a>);

impl<'a> Parse for Attribute<'a> {
    fn parse(input: &mut impl XmlSource) -> Result<Self> {
        let name = input.parse::<Name>()?;
        input.parse::<Eq>()?;

        Ok(Self(name, input.parse::<AttValue>()?))
    }
}

#[derive(PartialEq, Debug)]
pub struct XmlDecl<'a> {
    pub version: AttValue<'a>,
    pub encoding: Option<AttValue<'a>>,
    pub standalone: Option<AttValue<'a>>,
}

impl<'a> Parse for XmlDecl<'a> {
    fn parse(input: &mut impl XmlSource) -> Result<Self> {
        let mut content = input.delimited::<token::XmlDecl>()?;
        content.parse::<S>()?;

        let version = {
            content.parse_punct("version")?;
            content.parse::<Eq>()?;
            content.parse::<AttValue>()?
        };
        content.try_parse::<S>()?;

        let encoding = if content.try_parse_punct("encoding")?.is_some() {
            content.parse::<Eq>()?;
            let value = content.parse::<AttValue>()?;
            content.try_parse::<S>()?;
            Some(value)
        } else {
            None
        };

        let standalone = if content.try_parse_punct("standalone")?.is_some() {
            content.parse::<Eq>()?;
            let value = content.parse::<AttValue>()?;
            content.try_parse::<S>()?;
            Some(value)
        } else {
            None
        };

        Ok(Self {
            version,
            encoding,
            standalone,
        })
    }
}

#[derive(PartialEq, Debug)]
pub struct Pi<'a> {
    target: Name<'a>,
}

impl Parse for Pi<'_> {
    fn parse(input: &mut impl XmlSource) -> Result<Self> {
        let mut content = input.delimited::<token::Pi>()?;
        let target = content.parse::<Name>()?;

        Ok(Self { target })
    }
}

fn try_parse_misc<'a>(input: &mut impl XmlSource) -> Result<Option<XmlEvent<'a>>> {
    if let Some(s) = input.try_parse::<S>()? {
        Ok(Some(XmlEvent::S(s)))
    } else if let Some(pi) = input.try_parse::<Pi>()? {
        Ok(Some(XmlEvent::Pi(pi)))
    } else if let Some(comm) = input.try_parse::<Comment>()? {
        Ok(Some(XmlEvent::Comment(comm)))
    } else {
        Ok(None)
    }
}

#[derive(PartialEq, Debug)]
pub struct StartTag<'a> {
    pub name: Name<'a>,
    pub attrs: Vec<Attribute<'a>>,
}

impl<'a> Parse for StartTag<'a> {
    fn parse(input: &mut impl XmlSource) -> Result<Self> {
        let mut content = input.delimited::<token::STag>()?;
        let name = content.parse::<Name>()?;
        let mut attrs = Vec::new();

        while !content.is_empty()? {
            content.parse::<S>()?;

            if let Some(att) = content.try_parse::<Attribute>()? {
                attrs.push(att);
            }
        }

        Ok(Self { name, attrs })
    }
}

#[derive(PartialEq, Debug)]
pub struct EndTag<'a> {
    pub name: Name<'a>,
}

impl<'a> Parse for EndTag<'a> {
    fn parse(input: &mut impl XmlSource) -> Result<Self> {
        let mut content = input.delimited::<token::ETag>()?;
        let name = content.parse::<Name>()?;

        Ok(Self { name })
    }
}

#[derive(PartialEq, Debug)]
pub struct EmptyElem<'a> {
    pub name: &'a str,
}

impl<'a> Parse for EmptyElem<'a> {
    fn parse(_input: &mut impl XmlSource) -> Result<Self> {
        todo!()
    }
}

#[derive(PartialEq, Debug)]
pub enum XmlEvent<'a> {
    Xml(XmlDecl<'a>),
    Pi(Pi<'a>),
    STag(StartTag<'a>),
    ETag(EndTag<'a>),
    EmptyElem(EmptyElem<'a>),
    Text(Text<'a>),
    CData,
    S(S<'a>),
    Comment(Comment<'a>),
    Eof,
}

pub enum State {
    Start,
    AfterXml,
    InElem,
    AfterText,
    AfterRoot,
    Eof,
}

pub struct EventReader<P> {
    src: P,
    st: State,
    path: Vec<Box<str>>,
}

impl<S: XmlSource> EventReader<S> {
    pub fn new<I: Into<S>>(src: I) -> Self {
        EventReader {
            src: src.into(),
            st: State::Start,
            path: Vec::new(),
        }
    }

    pub fn next_event(&mut self) -> Result<XmlEvent> {
        match self.st {
            State::Start => {
                self.st = State::AfterXml;

                if let Some(xml_decl) = self.src.try_parse::<XmlDecl>()? {
                    Ok(XmlEvent::Xml(xml_decl))
                } else {
                    self.next_event()
                }
            }
            State::AfterXml => {
                if let Some(misc) = try_parse_misc(&mut self.src)? {
                    Ok(misc)
                } else if let Some(s_tag) = self.src.try_parse::<StartTag>()? {
                    self.st = State::InElem;
                    self.path.push(s_tag.name.value().into());
                    Ok(XmlEvent::STag(s_tag))
                } else {
                    todo!("error")
                }
            }
            State::InElem => {
                self.st = State::AfterText;

                if let Some(text) = self.src.try_parse::<Text>()? {
                    Ok(XmlEvent::Text(text))
                } else {
                    self.next_event()
                }
            }
            State::AfterText => {
                self.st = State::InElem;

                if let Some(s_tag) = self.src.try_parse::<StartTag>()? {
                    self.path.push(s_tag.name.value().into());
                    Ok(XmlEvent::STag(s_tag))
                } else if let Some(e_tag) = self.src.try_parse::<EndTag>()? {
                    if self.path.pop().is_some_and(|t| e_tag.name.is(&t)) {
                        if self.path.is_empty() {
                            self.st = State::AfterRoot;
                        }
                        Ok(XmlEvent::ETag(e_tag))
                    } else {
                        todo!("error")
                    }
                } else if let Some(pi) = self.src.try_parse::<Pi>()? {
                    Ok(XmlEvent::Pi(pi))
                } else if let Some(comment) = self.src.try_parse::<Comment>()? {
                    Ok(XmlEvent::Comment(comment))
                } else {
                    todo!("error")
                }
            }
            State::AfterRoot => {
                if let Some(misc) = try_parse_misc(&mut self.src)? {
                    Ok(misc)
                } else if self.src.is_empty()? {
                    self.st = State::Eof;
                    Ok(XmlEvent::Eof)
                } else {
                    todo!("error")
                }
            }
            State::Eof => Ok(XmlEvent::Eof),
        }
    }
}

impl<'a> From<&'a str> for EventReader<StreamReader<&'a [u8]>> {
    fn from(value: &'a str) -> Self {
        EventReader::new(StreamReader::new(value.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
    <note>
        <to>Tove</to>
        <from>Jani</from>
        <heading>Reminder</heading>
        <body>Don't forget me this weekend!</body>
    </note>
    "#;

    #[test]
    fn can_parse_simple_xml() -> Result<()> {
        let mut reader = EventReader::from(XML);

        loop {
            let event = reader.next_event()?;
            println!("{event:?}");

            if event == XmlEvent::Eof {
                break;
            }
        }
        Ok(())
    }
}
