use crate::doveadm::params::ImapField;
use crate::doveadm::parser::{FetchFieldRes, FieldType, Parser};
use crate::doveadm::{Reader, FORM_FEED, LINE_FEED};
use anyhow::{anyhow, Context, Result};
use regex::Regex;

pub struct GenericParser {
    field_type: ImapField,
    first_line_re: Regex,
    subseq_line_re: Regex,
}

impl GenericParser {
    pub fn new(field: &ImapField) -> Result<GenericParser> {
        let re_str = format!(r"^{}:(\s(.*))?$", field.to_string());
        let subseq_re_str = r"^([\S^:]+):\s(.*)$";
        Ok(GenericParser {
            field_type: field.clone(),
            first_line_re: Regex::new(re_str.as_str()).with_context(|| {
                format!(
                    "GenericParser::new: failed to create regex from '{}'",
                    re_str
                )
            })?,
            subseq_line_re: Regex::new(subseq_re_str)?,
        })
    }
}

impl Parser for GenericParser {
    fn get_first_line_re(&self) -> &Regex {
        &self.first_line_re
    }

    fn parse_first_field(
        &self,
        reader: &mut Reader,
        next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        if let Some(line) = reader.next_line()? {
            if let Some(captures) = self.first_line_re.captures(line) {
                if captures.len() > 1 {
                    // single line field
                    Ok(Some(FetchFieldRes::Generic((
                        self.field_type.clone(),
                        FieldType::SingleLine(
                            captures
                                .get(1)
                                .expect(
                                    "GenericParser::parse_first_field: unexpected empty capture",
                                )
                                .as_str()
                                .split_whitespace()
                                .map(|part| part.to_owned())
                                .collect(),
                        ),
                    ))))
                } else {
                    // multi line or empty
                    let next_field_re = if let Some(next_re) = next_re {
                        next_re
                    } else {
                        &self.first_line_re
                    };
                    let mut res: Vec<(String, String)> = Vec::new();
                    while let Some(line) = reader.next_line()? {
                        let line = line.trim_end_matches(LINE_FEED);
                        if line.ends_with(FORM_FEED) || next_field_re.is_match(line) {
                            reader.unconsume();
                            if res.len() > 0 {
                                return Ok(Some(FetchFieldRes::Generic((
                                    self.field_type.clone(),
                                    FieldType::MultiLine(res),
                                ))));
                            } else {
                                return Err(anyhow!(
                                    "GenericParser::parse_first_field: unexpected empty field"
                                ));
                            }
                        } else {
                            if let Some(captures) = self.subseq_line_re.captures(line) {
                                res.push((
                                    captures
                                        .get(1)
                                        .expect(
                                            format!("GenericParser::parse_first_field: unexpected empty Hdr name in line '{}'", line)
                                                .as_str(),
                                        )
                                        .as_str()
                                        .to_owned(),
                                    captures
                                        .get(2)
                                        .expect(
                                            format!("GenericParser::parse_first_field: unexpected empty Hdr value in line '{}'", line)
                                                .as_str(),
                                        )
                                        .as_str()
                                        .to_owned(),
                                ));
                            } else {
                                if let Some(last_res) = res.last_mut() {
                                    last_res.1.push('\n');
                                    last_res.1.push_str(line);
                                } else {
                                    return Err(anyhow!(
                                    "GenericParser::parse_first_field: hdr regex failed to match in line '{}'",
                                    line
                                ));
                                }
                            }
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
