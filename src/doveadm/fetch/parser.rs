use crate::doveadm::fetch::params::ImapField;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::debug;
use regex::Regex;

mod generic_parser;
pub use generic_parser::GenericParser;
mod macro_parsers;
pub use macro_parsers::{
    DateReceivedParser, DateSavedParser, DateSentParser, FlagsParser, GuidParser, MailboxParser,
    SizePhysicalParser, UidParser,
};

mod hdr_parser;
use crate::doveadm::fetch::stdout_reader::StdoutLineReader;
pub use hdr_parser::HdrParser;

const LINE_FEED: char = 0xAu8 as char;
const FORM_FEED: char = 0xCu8 as char;

#[derive(Debug)]
pub struct FetchRecord(Vec<FetchFieldRes>);

impl FetchRecord {
    pub async fn parse(
        parsers: &[Box<dyn Parser + Sync + Send>],
        reader: &mut StdoutLineReader,
    ) -> Result<Option<FetchRecord>> {
        debug!("FetchRecord::parse: started");

        if let Some(line) = reader.next_line().await? {
            debug!("parse: got line: {:?}", line);
            if !line.ends_with(FORM_FEED) {
                reader.unconsume();
            }

            let mut res: Vec<FetchFieldRes> = Vec::new();
            let mut parsers = parsers.iter();
            let parser = parsers.next().expect("unexpected empty parser list");
            let mut next_parser = parsers.next();

            if let Some(curr_res) = parser
                .parse_first_field(reader, next_parser.map(|parser| parser.get_first_line_re()))
                .await?
            {
                res.push(curr_res);
            } else {
                // EOI on first field
                return Ok(None);
            }

            while let Some(parser) = next_parser {
                next_parser = parsers.next();
                res.push(
                    parser
                        .parse_subseq_field(
                            reader,
                            next_parser.map(|parser| parser.get_first_line_re()),
                        )
                        .await?,
                );
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

#[derive(Debug)]
pub enum FieldType {
    MultiLine(Vec<String>),
    SingleLine(String),
}

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
    Generic((ImapField, FieldType)),
}

#[async_trait]
pub trait Parser {
    // used by some preceding parsers to find the end of record (start of next)
    fn get_first_line_re(&self) -> &Regex;
    // parse a field (all lines of it) - only the first field of a record may encounter an EOI
    // next_re: ist the next fields first line parser regex - if it is None, there is no next field
    // and we will have to use this parsers first line re
    async fn parse_first_field(
        &self,
        reader: &mut StdoutLineReader,
        next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>>;
    // parse a field (all lines of it) - for any field other than the first of a record an EOI
    // constitutes an error
    async fn parse_subseq_field(
        &self,
        reader: &mut StdoutLineReader,
        next_re: Option<&Regex>,
    ) -> Result<FetchFieldRes> {
        if let Some(res) = self.parse_first_field(reader, next_re).await? {
            Ok(res)
        } else {
            Err(anyhow!("unexpected empty subsequent field"))
        }
    }
}
