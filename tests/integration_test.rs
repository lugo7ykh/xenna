use std::error::Error;

use xenna::reader::{EventReader, XmlEvent};

const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
    <note>
        <to>Tove</to>
        <from>Jani</from>
        <heading>Reminder</heading>
        <body>Don't forget me this weekend!</body>
    </note>
"#;

#[test]
fn can_parse_simple_xml() -> Result<(), Box<dyn Error>> {
    let mut reader = EventReader::from(XML);

    loop {
        let event = reader.next_event()?;
        println!("{event:?}");

        if event == XmlEvent::Eof {
            return Ok(());
        }
    }
}
