use crate::doveadm::params::ImapField;
use crate::doveadm::Reader;
use anyhow::{anyhow, Context, Result};
use log::debug;
use regex::Regex;

mod flags_parser;
pub use flags_parser::FlagsParser;
mod generic_parser;
pub use generic_parser::GenericParser;
mod hdr_parser;
pub use hdr_parser::HdrParser;

#[derive(Debug)]
pub struct FetchRecord(Vec<FetchFieldRes>);

impl FetchRecord {
    pub fn parse(
        parsers: &Vec<Box<dyn Parser>>,
        reader: &mut Reader,
    ) -> Result<Option<FetchRecord>> {
        debug!("FetchRecord::parse: started");
        let mut res: Vec<FetchFieldRes> = Vec::new();
        let mut parsers = parsers.iter();
        let mut parser = parsers.next().expect("unexpected empty parser list");
        let mut next_parser = parsers.next();

        if let Some(curr_res) = parser
            .parse_first_field(reader, next_parser.map(|parser| parser.get_first_line_re()))?
        {
            res.push(curr_res);
        } else {
            // EOI on first field
            return Ok(None);
        }

        while let Some(parser) = next_parser {
            next_parser = parsers.next();
            res.push(parser.parse_subseq_field(
                reader,
                next_parser.map(|parser| parser.get_first_line_re()),
            )?);
        }

        Ok(Some(FetchRecord(res)))
    }
}

#[derive(Debug)]
enum FieldType {
    MultiLine(Vec<(String, String)>),
    SingleLine(Vec<String>),
}

#[derive(Debug)]
pub enum FetchFieldRes {
    Flags(Vec<String>),
    Hdr(Vec<(String, String)>),
    Generic((ImapField, FieldType)),
}

pub trait Parser {
    // used by some preceding parsers to find the end of record (start of next)
    fn get_first_line_re(&self) -> &Regex;
    // parse a field (all lines of it) - only the first field of a record may encounter an EOI
    // next_re: ist the next fields first line parser regex - if it is None, there is no next field
    // and we will have to use this parsers first line re
    fn parse_first_field(
        &self,
        reader: &mut Reader,
        next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>>;
    // parse a field (all lines of it) - for any field other than the first of a record an EOI
    // constitutes an error
    fn parse_subseq_field(
        &self,
        reader: &mut Reader,
        next_re: Option<&Regex>,
    ) -> Result<FetchFieldRes> {
        if let Some(res) = self.parse_first_field(reader, next_re)? {
            Ok(res)
        } else {
            Err(anyhow!("unexpected empty subsequent field"))
        }
    }
}
