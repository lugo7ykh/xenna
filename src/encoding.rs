use std::{
    io::{Error, ErrorKind, Result},
    str,
};

const UTF_8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF_16BE_BOM: &[u8] = &[0xFE, 0xFF];
const UTF_16LE_BOM: &[u8] = &[0xFF, 0xFE];

#[derive(Default, PartialEq, Clone, Copy, Debug)]
pub enum Encoding {
    Ascii,
    #[default]
    Utf8,
    Utf16Be,
    Utf16Le,
}

impl Encoding {
    pub fn next_char<'a>(&self, bytes: &'a [u8]) -> Option<(char, &'a [u8])> {
        let char_len = match self {
            Self::Utf8 => utf8_char_len(*bytes.first()?).unwrap(),
            Encoding::Utf16Be | Encoding::Utf16Le => 2,
            _ => 1,
        };
        let (char_bytes, rest) = bytes.split_at(char_len);

        match self {
            Self::Ascii | Self::Utf8 => unsafe { str::from_utf8_unchecked(char_bytes) },
            _ => todo!(),
        }
        .chars()
        .next()
        .map(|char| (char, rest))
    }
}

pub fn detect(bytes: &[u8]) -> (Encoding, usize) {
    let mut bom_len = 0;

    let encoding = if bytes.starts_with(UTF_16BE_BOM) {
        bom_len = UTF_16BE_BOM.len();
        Encoding::Utf16Be
    } else if bytes.starts_with(UTF_16LE_BOM) {
        bom_len = UTF_16LE_BOM.len();
        Encoding::Utf16Le
    } else if bytes.starts_with(UTF_8_BOM) {
        bom_len = UTF_8_BOM.len();
        Encoding::Utf8
    } else if bytes.starts_with(&[0x00, b'<', 0x00, b'?']) {
        Encoding::Utf16Be
    } else if bytes.starts_with(&[b'<', 0x00, b'?', 0x00]) {
        Encoding::Utf16Le
    } else if bytes.starts_with(&[b'<', b'?', b'x', b'm']) {
        Encoding::Ascii
    } else {
        Default::default()
    };

    (encoding, bom_len)
}

pub fn utf8_char_len(first_byte: u8) -> Result<usize> {
    match first_byte {
        ..=0b01111111 => Ok(1),
        0b11000000..=0b11011111 => Ok(2),
        0b11100000..=0b11101111 => Ok(3),
        0b11110000..=0b11110111 => Ok(4),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("1st byte `{first_byte:b}` of Utf-8 char is invalid"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_detect_encoding_and_remove_bom() {
        assert_eq!(detect(UTF_8_BOM), (Encoding::Utf8, 3));
        assert_eq!(detect(UTF_16BE_BOM), (Encoding::Utf16Be, 2));
        assert_eq!(detect(UTF_16LE_BOM), (Encoding::Utf16Le, 2));
    }

    #[test]
    fn can_detect_utf8_char_size() -> Result<()> {
        assert_eq!(utf8_char_len(0b01111111)?, 1);
        assert_eq!(utf8_char_len(0b11011111)?, 2);
        assert_eq!(utf8_char_len(0b11101111)?, 3);
        assert_eq!(utf8_char_len(0b11110111)?, 4);

        assert!(utf8_char_len(0b11111011).is_err());
        assert!(utf8_char_len(0b10000001).is_err());
        Ok(())
    }
}
