use crate::doveadm::fetch::params::ImapField;
use crate::doveadm::fetch::parser::{FetchFieldRes, Parser};
use crate::doveadm::fetch::stdout_reader::StdoutLineReader;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::{debug, trace};
use regex::Regex;

// TODO: must support duplicate keys - go back to Vec instead of hashmap or
// supply individual header fields allowing for 'Received' and 'DKIM-Signature'
// to support multiple  occurrences

pub struct HdrParser {
    first_line_re: Regex,
    subseq_line_re: Regex,
}
impl HdrParser {
    pub fn new() -> Result<HdrParser> {
        let re_str = format!(r"^{}:$", ImapField::Hdr.to_string());
        let subseq_re_str = r"^(([^:]+):\s(.*)|\s*(\S.*))$";
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
        _next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        trace!("parse_first_field: called");
        if let Some(line) = reader.next_line().await? {
            trace!("parse_first_field: got line: [{:?}]", line);
            if self.first_line_re.is_match(line) {
                let mut res: Vec<(String, String)> = Vec::new();
                while let Some(line) = reader.next_line().await? {
                    trace!("parse_first_field: got next line: [{:?}]", line);
                    if let Some(captures) = self.subseq_line_re.captures(line) {
                        if let Some(no_tag) = captures.get(4) {
                            trace!("parse_first_field: adding untagged string");
                            let add_val = no_tag.as_str();
                            let (_, value) =
                                res.last_mut().expect("unexpected: last value not found");
                            value.push('\n');
                            value.push_str(add_val);
                        } else {
                            trace!("parse_first_field: adding tagged string");
                            res.push(
                                (captures
                                     .get(2)
                                     .unwrap_or_else(|| panic!("HdrParser::parse_first_field: unexpected empty Hdr value in line '{}'", line))
                                     .as_str()
                                     .to_owned(),
                                    captures
                                        .get(3)
                                        .unwrap_or_else(|| panic!("HdrParser::parse_first_field: unexpected empty Hdr value in line '{}'", line))
                                        .as_str()
                                        .to_owned(),
                                ));
                        }
                    } else if line.is_empty() {
                        return Ok(Some(FetchFieldRes::Hdr(res)));
                    } else {
                        return Err(anyhow!("hdr regex failed to match in line '{}'", line));
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
