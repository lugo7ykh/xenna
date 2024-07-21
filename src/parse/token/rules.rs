pub(super) fn accept_as_char(ch: char) -> bool {
    matches!(ch,
        | '\u{9}'
        | '\u{A}'
        | '\u{D}'
        | '\u{20}'..='\u{D7FF}'
        | '\u{E000}'..='\u{FFFD}'
        | '\u{10000}'..='\u{10FFFF}'
    )
}

pub(super) fn accept_as_white_space(ch: char) -> bool {
    matches!(ch, '\u{20}' | '\u{9}' | '\u{D}' | '\u{A}')
}

fn accept_as_name_start_char(ch: char) -> bool {
    matches!(ch,
        | ':'
        | 'A'..='Z'
        | '_'
        | 'a'..='z'
        | '\u{C0}'..='\u{D6}'
        | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}'
        | '\u{370}'..='\u{37D}'
        | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}'
    )
}

fn accept_as_name_char(ch: char) -> bool {
    accept_as_name_start_char(ch)
        || matches!(ch,
            | '-'
            | '.'
            | '0'..='9'
            | '\u{B7}'
            | '\u{0300}'..='\u{036F}'
            | '\u{203F}'..='\u{2040}'
        )
}

pub(super) fn accept_as_att_value(ch: char) -> bool {
    !matches!(ch, '<' | '&')
}

pub(super) fn accept_as_name() -> impl FnMut(char) -> bool {
    let mut is_start_char = true;

    move |ch| {
        if is_start_char {
            is_start_char = false;
            accept_as_name_start_char(ch)
        } else {
            accept_as_name_char(ch)
        }
    }
}

pub(super) fn accept_as_comment() -> impl FnMut(char) -> bool {
    let mut previous_was_a_hyphen = false;

    move |ch| {
        let current_is_a_hyphen = ch == '-';

        let is_accepted = if current_is_a_hyphen {
            !previous_was_a_hyphen
        } else {
            accept_as_char(ch)
        };
        previous_was_a_hyphen = current_is_a_hyphen;

        is_accepted
    }
}

const CDATA_CLOSE_DELIM: &str = "]]>";

pub(super) fn accept_as_char_data() -> impl FnMut(char) -> bool {
    let delim_len = CDATA_CLOSE_DELIM.len();
    let mut matched_bytes_count = 0;

    move |ch| {
        if CDATA_CLOSE_DELIM[matched_bytes_count..].starts_with(ch) {
            matched_bytes_count += ch.len_utf8();
        } else {
            matched_bytes_count = 0;
        }

        matched_bytes_count < delim_len && !matches!(ch, '<' | '&')
    }
}
