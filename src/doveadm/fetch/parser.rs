use anyhow::{anyhow, Result};
use log::debug;
use regex::Regex;

// mod generic_parser;
// pub use generic_parser::GenericParser;
mod macro_parsers;
pub use macro_parsers::{
    DateReceivedParser, DateSavedParser, DateSentParser, FlagsParser, GuidParser, MailboxParser,
    SizePhysicalParser, UidParser,
};

mod hdr_parser;
use crate::doveadm::fetch::stdout_reader::StdoutLineReader;
pub use hdr_parser::HdrParser;

const FORM_FEED: char = 0xCu8 as char;
const FORM_FEED_STR: &str = "\u{c}";

#[derive(Debug)]
pub struct FetchRecord(Vec<FetchFieldRes>);

impl FetchRecord {
    pub fn parse(
        parsers: &[Box<dyn Parser>],
        reader: &mut StdoutLineReader,
    ) -> Result<Option<FetchRecord>> {
        debug!("FetchRecord::parse: started");

        if let Some(line) = reader.next_line()? {
            debug!("parse: got line: {:?}", line);
            if !line.ends_with(FORM_FEED) {
                reader.unconsume();
            }

            let mut res: Vec<FetchFieldRes> = Vec::new();
            let first_parser = if let Some(first_parser) = parsers.first() {
                first_parser
            } else {
                return Err(anyhow!("parse: empty list of parsers encoutered"));
            };

            let mut parsers = parsers.iter();
            let parser = parsers.next().expect("unexpected empty parser list");
            let mut next_parser = parsers.next();

            if let Some(curr_res) = parser.parse_first_field(
                reader,
                next_parser.unwrap_or(first_parser).get_first_line_re(),
            )? {
                res.push(curr_res);
            } else {
                // EOI on first field
                return Ok(None);
            }

            while let Some(parser) = next_parser {
                next_parser = parsers.next();
                res.push(parser.parse_subseq_field(
                    reader,
                    next_parser.unwrap_or(first_parser).get_first_line_re(),
                )?);
            }

            Ok(Some(FetchRecord(res)))
        } else {
            Ok(None)
        }
    }
}

impl IntoIterator for FetchRecord {
    type Item = FetchFieldRes;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/*
#[derive(Debug)]
pub enum FieldType {
    MultiLine(Vec<String>),
    SingleLine(String),
}
*/

#[derive(Debug)]
pub enum FetchFieldRes {
    Hdr(Vec<(String, String)>),
    Uid(String),
    Guid(String),
    Flags(Vec<String>),
    Mailbox(String),
    DateSaved(String),
    DateReceived(String),
    DateSent(String),
    SizePhysical(usize),
    // Generic((ImapField, FieldType)),
}

pub trait Parser {
    // used by some preceding parsers to find the end of record (start of next)
    fn get_first_line_re(&self) -> &Regex;
    // parse a field (all lines of it) - only the first field of a record may encounter an EOI
    // next_re: ist the next fields first line parser regex - if it is None, there is no next field
    // and we will have to use this parsers first line re
    fn parse_first_field(
        &self,
        reader: &mut StdoutLineReader,
        next_re: &Regex,
    ) -> Result<Option<FetchFieldRes>>;
    // parse a field (all lines of it) - for any field other than the first of a record an EOI
    // constitutes an error
    fn parse_subseq_field(
        &self,
        reader: &mut StdoutLineReader,
        next_re: &Regex,
    ) -> Result<FetchFieldRes> {
        if let Some(res) = self.parse_first_field(reader, next_re)? {
            Ok(res)
        } else {
            Err(anyhow!("unexpected empty subsequent field"))
        }
    }
}
