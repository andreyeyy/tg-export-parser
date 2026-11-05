//! ## TgExportParser
//! Simple library for parsing a Telegram export file (result.json)
//!
//! Why this library? It lets you parse result.json message by message, without storing
//! the entire file in memory at once. This makes it possible to efficiently parse
//! files of any size, while other parsers would run out of memory.
//!
//! #### Example
//!
//! ```ignore
//! use std::path::Path;
//!
//! use tg_export_parser::TgExportParser;
//!
//! fn main() {
//!
//!     let path = Path::new("/path/to/result.json");
//!
//!     let messages = TgExportParser::from_file(path).unwrap();
//!
//!     for message in messages {
//!         // Process messages one by one here
//!         
//!         // For example, print all messages
//!         println!("{:?}", message.unwrap().text);
//!     }
//! }
//! ```


use std::fs::File;
use std::io::{BufReader, BufRead, Read};
use std::path::{Path, PathBuf};
use serde::Deserialize;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ParseError {
    InvalidUnicode,
    ReadError,
    BackslashOutsideString,
    InvalidEscape(char),
    SerdeJsonError,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackslashOutsideString =>
                write!(f, "unexpected backslash outside of a string"),
            Self::InvalidUnicode=>
                write!(f, "encountered invalid unicode character"),
            Self::ReadError=>
                write!(f, "error reading bytes from file"),
            Self::InvalidEscape(ch) =>
                write!(f, "unexpected escaped character: \\{ch}"),
            Self::SerdeJsonError =>
                write!(f, "serde_json failed to parse a message")
        }
    }
}

impl Error for ParseError {}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MessageText {
    Plain(String),
    Parts(Vec<TextPart>),
}

impl MessageText {
    pub fn as_text(&self) -> String {
        match self {
            MessageText::Plain(s) => s.clone(),
            MessageText::Parts(parts) => parts
                .iter()
                .map(|part| match part {
                    TextPart::Plain(s) => s.as_str(),
                    TextPart::Entity(entity) => entity.text.as_str(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TextPart {
    Plain(String),
    Entity(MessageEntity),
}

#[derive(Debug, Deserialize)]
struct MessageEntity {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct RawMessage {
    pub id: i64,
    pub reply_to_message_id: Option<i64>,
    pub date_unixtime: String,
    pub date: String,
    pub from: Option<String>,
    pub from_id: Option<String>,
    pub media_type: Option<String>,
    pub text: Option<MessageText>,
}

impl RawMessage {
    fn from_json(json_message: &str) -> Result<Self, ParseError> {
        serde_json::from_str::<Self>(json_message)
            .map_err(|_| ParseError::SerdeJsonError)
    }
}

pub struct TgExportParser {
    reader: BufReader<File>,
    bracket_level: i32,
    inside_string: bool,
    last_char_escape: bool,
    first_run: bool,
}

fn message_from_bytes(bytes: Vec<u8>) -> Result<RawMessage, ParseError> {
    let message_str = str::from_utf8(&bytes).map_err(|_| ParseError::InvalidUnicode)?;
    return RawMessage::from_json(message_str);
}

impl TgExportParser {
    /// Takes path to result.json, returns an iterator over messages
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        Ok(TgExportParser{
            reader,
            inside_string: false,
            bracket_level: 0,
            last_char_escape: false,
            first_run: true,
        })
    }
}

impl Iterator for TgExportParser {
    type Item = Result<RawMessage, ParseError>;

    /// Returns the next message in the result.json file. The type of each message is RawMessage
    fn next(&mut self) -> Option<Self::Item>{

        if self.first_run {
            self.first_run = false;
            if let Err(_e) = self.reader.skip_until(b'[') {
                return Some(Err(ParseError::ReadError));
            }
        }

        let mut message_buf: Vec<u8> = Vec::new();
        let bytes = (&mut self.reader).bytes();

        for b in bytes {
            let b = match b {
                Ok(b) => b,
                Err(_) => return Some(Err(ParseError::ReadError)),
            };

            message_buf.push(b);

            if self.last_char_escape { // if the character before is '\'
                self.last_char_escape = false;

                if !self.inside_string {
                    return Some(Err(ParseError::BackslashOutsideString));
                }
                if !(
                    b == b'\\' || 
                    b == b'\"' || 
                    b == b'/' || 
                    b == b'b' || 
                    b == b'f' || 
                    b == b'n' || 
                    b == b'r' || 
                    b == b't' || 
                    b == b'u'
                ) {
                    return Some(Err(ParseError::InvalidEscape(b as char)));
                }
                continue;
            }

            if b == b'\\' { // escape character
                self.last_char_escape = true;
            }

            else if b == b'\"' { // the " character, not escaped
                self.inside_string = !self.inside_string;
            }

            else if b == b'{' {
                if self.inside_string {continue;}
                self.bracket_level += 1;
            }

            else if b == b'}' {
                if self.inside_string {continue;}
                self.bracket_level -= 1;
                if self.bracket_level == 0 {

                    if let Err(_e) = self.reader.skip_until(b',') {
                        return Some(Err(ParseError::ReadError));
                    }

                    let msg = message_from_bytes(message_buf);
                    return Some(msg);
                }
            }

        }
        return None;
    }
}


#[test]
fn valid_json() {
    let path: PathBuf = std::env::var("TEST_FILE").expect("Set TEST_FILE to a file path").parse().expect("Invalid TEST_FILE path");

    let messages = TgExportParser::from_file(&path).expect("Create parser");

    for m in messages {
        m.unwrap();
    }
}
