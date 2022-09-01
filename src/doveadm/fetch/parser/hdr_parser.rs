use crate::doveadm::fetch::params::ImapField;
use crate::doveadm::fetch::parser::{FetchFieldRes, Parser, FORM_FEED_STR};
use crate::doveadm::fetch::stdout_reader::StdoutLineReader;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::{debug, trace};
use regex::Regex;

pub struct HdrParser {
    first_line_re: Regex,
    subseq_line_re: Regex,
}
impl HdrParser {
    pub fn new() -> Result<HdrParser> {
        let re_str = format!(r"^{}:$", ImapField::Hdr.to_string());
        let subseq_re_str = r"^([^:]+):\s(.*)$";
        debug!("first_line_re:  {:?}", re_str);
        debug!("subseq_line_re: {:?}", subseq_re_str);
        Ok(HdrParser {
            first_line_re: Regex::new(re_str.as_str())
                .with_context(|| format!("failed to create regex from '{}'", re_str))?,
            subseq_line_re: Regex::new(subseq_re_str)
                .with_context(|| format!("failed to create regex from '{}'", subseq_re_str))?,
        })
    }
}

#[async_trait]
impl Parser for HdrParser {
    fn get_first_line_re(&self) -> &Regex {
        &self.first_line_re
    }

    async fn parse_first_field(
        &self,
        reader: &mut StdoutLineReader,
        next_re: &Regex,
    ) -> Result<Option<FetchFieldRes>> {
        trace!("parse_first_field: called");
        if let Some(line) = reader
            .next_line()
            .await
            .with_context(|| "next_line failed".to_owned())?
        {
            trace!("parse_first_field: got first line: [{:?}]", line);
            if self.first_line_re.is_match(line) {
                let mut res: Vec<(String, String)> = Vec::new();
                while let Some(line) = reader
                    .next_line_rfc2047()
                    .await
                    .with_context(|| "parse_first_field: next_line_rfc2047 failed".to_owned())?
                {
                    trace!("parse_first_field: got next line: [{:?}]", line);
                    if line.is_empty() {
                        match reader.next_line_rfc2047().await.with_context(|| {
                            "parse_first_field: next_line_rfc2047 failed".to_owned()
                        })? {
                            Some(line) => {
                                if next_re.is_match(line) {
                                    trace!("parse_first_field: found match for next field, returning results");
                                    reader.unconsume();
                                    return Ok(if res.is_empty() {
                                        None
                                    } else {
                                        Some(FetchFieldRes::Hdr(res))
                                    });
                                } else {
                                    let (_, value) =
                                        res.last_mut().expect("unexpected: last value not found");
                                    value.push('\n');
                                    value.push_str(line.as_ref());
                                }
                            }
                            None => {
                                return Ok(if res.is_empty() {
                                    None
                                } else {
                                    Some(FetchFieldRes::Hdr(res))
                                });
                            }
                        }
                    } else if line.eq(FORM_FEED_STR) {
                        trace!("parse_first_field: found form feed, returning results");
                        return Ok(if res.is_empty() {
                            None
                        } else {
                            Some(FetchFieldRes::Hdr(res))
                        });
                    } else if let Some(captures) = self.subseq_line_re.captures(line) {
                        trace!("parse_first_field: adding tagged string");
                        res.push(
                                (captures
                                     .get(1)
                                     .unwrap_or_else(|| panic!("HdrParser::parse_first_field: unexpected empty Hdr value in line '{}'", line))
                                     .as_str()
                                     .to_owned(),
                                    captures
                                        .get(2)
                                        .unwrap_or_else(|| panic!("HdrParser::parse_first_field: unexpected empty Hdr value in line '{}'", line))
                                        .as_str()
                                        .to_owned(),
                                ));
                    } else {
                        trace!("parse_first_field: adding untagged string: [{:?}]", line);
                        let (_, value) = res.last_mut().expect("unexpected: last value not found");
                        value.push('\n');
                        value.push_str(line.as_ref());
                    }
                    trace!("parse_first_field: done with line");
                }
                trace!("parse_first_field: done with lines");
            } else {
                debug!("first line re did not match {:?}", line);
                return Err(anyhow!(
                    "HdrParser::parse_first_field: Hdr parser failed to match first line"
                ));
            }
            // TODO: really ? or rather return what we have got
            Err(anyhow!("unexpected EOI in field"))
        } else {
            Ok(None)
        }
    }
}
