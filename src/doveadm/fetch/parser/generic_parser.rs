use crate::doveadm::fetch::params::ImapField;
use crate::doveadm::fetch::parser::FieldType::MultiLine;
use crate::doveadm::fetch::parser::{FetchFieldRes, FieldType, Parser, FORM_FEED, LINE_FEED};
use crate::doveadm::fetch::stdout_reader::StdoutReader;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::debug;
use regex::Regex;

pub struct GenericParser {
    field_type: ImapField,
    first_line_re: Regex,
}

impl GenericParser {
    pub fn new(field: &ImapField) -> Result<GenericParser> {
        let re_str = format!(r"^{}:(\s(.*))?$", field.to_string());
        debug!("first_line_re:  {:?}", re_str);
        Ok(GenericParser {
            field_type: field.clone(),
            first_line_re: Regex::new(re_str.as_str()).with_context(|| {
                format!(
                    "GenericParser::new: failed to create regex from '{}'",
                    re_str
                )
            })?,
        })
    }
}

#[async_trait]
impl Parser for GenericParser {
    fn get_first_line_re(&self) -> &Regex {
        &self.first_line_re
    }

    async fn parse_first_field(
        &self,
        reader: &mut StdoutReader,
        next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        if let Some(line) = reader.next_line().await? {
            let line = line.trim_end_matches(LINE_FEED);
            if let Some(captures) = self.first_line_re.captures(line) {
                if captures.len() > 2 {
                    // single line field
                    Ok(Some(FetchFieldRes::Generic((
                        self.field_type.clone(),
                        FieldType::SingleLine(
                            captures
                                .get(2)
                                .expect(
                                    "GenericParser::parse_first_field: unexpected empty capture",
                                )
                                .as_str()
                                .to_owned(),
                        ),
                    ))))
                } else {
                    // multi line or empty
                    let next_field_re = if let Some(next_re) = next_re {
                        next_re
                    } else {
                        &self.first_line_re
                    };
                    let mut res: Vec<String> = Vec::new();
                    while let Some(line) = reader.next_line().await? {
                        let line = line.trim_end_matches(LINE_FEED);
                        if line.ends_with(FORM_FEED) {
                            return Ok(Some(FetchFieldRes::Generic((
                                self.field_type.clone(),
                                MultiLine(res),
                            ))));
                        }
                        if next_field_re.is_match(line) {
                            reader.unconsume();
                            return Ok(Some(FetchFieldRes::Generic((
                                self.field_type.clone(),
                                FieldType::MultiLine(res),
                            ))));
                        } else {
                            res.push(line.to_owned());
                        }
                    }
                    // no next line
                    // TODO: really ? or rather return what we have got
                    Err(anyhow!(
                        "GenericParser::parse_first_field: unexpected EOI in field"
                    ))
                }
            } else {
                Err(anyhow!(
                    "GenericParser::parse_first_field: Hdr parser failed to match first line"
                ))
            }
        } else {
            Ok(None)
        }
    }
}
