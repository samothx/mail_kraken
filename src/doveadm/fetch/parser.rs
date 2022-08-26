use crate::doveadm::fetch::params::ImapField;
use crate::doveadm::fetch::Reader;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::debug;
use regex::Regex;
use std::collections::HashMap;
// mod single_line_parser;
// pub use single_line_parser::SingleLineParser;
mod generic_parser;
pub use generic_parser::GenericParser;
mod macro_parsers;
pub use macro_parsers::{GuidParser, UidParser};

mod hdr_parser;
pub use hdr_parser::HdrParser;

const LINE_FEED: char = 0xAu8 as char;
const FORM_FEED: char = 0xCu8 as char;
const EIR: &str = "\u{C}\u{A}";

#[derive(Debug)]
pub struct FetchRecord(Vec<FetchFieldRes>);

impl FetchRecord {
    pub async fn parse(
        parsers: &Vec<Box<dyn Parser + Sync>>,
        reader: &mut Reader<'_>,
    ) -> Result<Option<FetchRecord>> {
        debug!("FetchRecord::parse: started");

        if let Some(line) = reader.next_line().await? {
            if !line.ends_with(EIR) {
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
pub enum SingleLineType {
    StringType(String),
    ListType(Vec<String>),
}

#[derive(Debug)]
pub enum FetchFieldRes {
    Hdr(Vec<(String, String)>),
    Uid(String),
    Guid(String),
    Flags(Vec<String>),
    Mailbox(String),
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
        reader: &mut Reader,
        next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>>;
    // parse a field (all lines of it) - for any field other than the first of a record an EOI
    // constitutes an error
    async fn parse_subseq_field(
        &self,
        reader: &mut Reader,
        next_re: Option<&Regex>,
    ) -> Result<FetchFieldRes> {
        if let Some(res) = self.parse_first_field(reader, next_re).await? {
            Ok(res)
        } else {
            Err(anyhow!("unexpected empty subsequent field"))
        }
    }
}
