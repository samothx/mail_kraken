use anyhow::{anyhow, Result};
use regex::Regex;

const EMAIL_REGEX: &str = r#"(([^<^,]*)\s+<([^>^,]+)>|([^,]+)),?\s*"#;

pub struct EmailParser {
    regex: Regex,
}

impl EmailParser {
    pub fn new() -> EmailParser {
        Self {
            regex: Regex::new(EMAIL_REGEX)
                .expect(format!("failed to create regex from [{}]", EMAIL_REGEX).as_str()),
        }
    }

    pub fn parse(&self, emails: &str) -> Result<Vec<(String, Option<String>)>> {
        let captures = self.regex.captures_iter(emails);
        let mut res = Vec::new();
        for cap in captures {
            if let Some(email) = cap.get(4) {
                res.push((email.as_str().to_owned(), None))
            } else if let Some(email) = cap.get(3) {
                res.push((
                    email.as_str().to_owned(),
                    cap.get(2).map(|name| name.as_str().to_owned()),
                ))
            }
        }
        if res.len() > 0 {
            Ok(res)
        } else {
            Err(anyhow!("no matches found"))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::EmailParser;

    #[test]
    fn parse_email_simple_email1() {
        let parser = EmailParser::new();
        match parser.parse("name@domain.tld") {
            Ok(res_list) => {
                assert_eq!(res_list, vec![("name@domain.tld".to_owned(), None)]);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }
    #[test]
    fn parse_email_simple_email2() {
        let parser = EmailParser::new();
        match parser.parse("name@domain.tld,") {
            Ok(res_list) => {
                assert_eq!(res_list, vec![("name@domain.tld".to_owned(), None)]);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn parse_email_simple_email3() {
        let parser = EmailParser::new();
        match parser.parse("name@domain.tld,\n") {
            Ok(res_list) => {
                assert_eq!(res_list, vec![("name@domain.tld".to_owned(), None)]);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn parse_email_simple_email_with_name() {
        let parser = EmailParser::new();
        match parser.parse(r#"kurt mustermann <name@domain.tld>"#) {
            Ok(res_list) => {
                assert_eq!(
                    res_list,
                    vec![(
                        "name@domain.tld".to_owned(),
                        Some("kurt mustermann".to_owned())
                    )]
                );
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn parse_email_simple_email_list1() {
        let parser = EmailParser::new();
        match parser.parse("name1@domain1.tld1, name2@domain2.tld2") {
            Ok(res_list) => {
                assert_eq!(
                    res_list,
                    vec![
                        ("name1@domain1.tld1".to_owned(), None),
                        ("name2@domain2.tld2".to_owned(), None)
                    ]
                );
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn parse_email_simple_email_list2() {
        let parser = EmailParser::new();
        match parser.parse("name1@domain1.tld1,\n name2@domain2.tld2,\n ") {
            Ok(res_list) => {
                assert_eq!(
                    res_list,
                    vec![
                        ("name1@domain1.tld1".to_owned(), None),
                        ("name2@domain2.tld2".to_owned(), None)
                    ]
                );
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    #[test]
    fn parse_email_complex_email_list1() {
        const EMAILS: &str = r#"\"Firmian\" Steinfath Mathias" <firmian@cenci.de>,
"Andreas + Angela Horn" <aahorn@gmx.de>,
"Benedict Dudda" <BenedictDudda@gmx.de>,
"Britta Friedmann" <friedman.mail@arcor.de>,
"Carmen Hajunga" <carmen.hajunga@gmx.de>,
"Carsten Hajunga" <carsten.hajunga@gmx.de>,
"Carsten Oerzen" <carstenoertzen@freenet.de>,
Enrico Röwer <rico-netti@t-online.de>,
"Fridolin Dudda" <fridolin_dudda@t-online.de>,
"Gebhardt Mirko" <mirko-gebhardt@gmx.de>,
"Hans-Dieter Protsch" <h-d-p@web.de>,
Holger Heß <holger@bikerstorch.de>,
Ines Möllendorf <carathis@gmx.de>,
"Ingrid Hillmer" <stjerne@gmx.de>,
"kim kahle" <kk@kimkahle.com>,
"Mau Malte" <malte_49@yahoo.de>,
"Mau Wilfried" <wilfried-mau@hotmail.de>,
"Michael Bostelmann" <bostelmaennerx5@t-online.de>,
Olaf Völker <olaf@voelker-wl.de>,
"Peters Christine" <christine-peters@gmx.de>,
Petersen Jörn <joernp@arcor.de>,
"Plath Dominick" <dominickplath@web.de>,
"Rainer Hillmer" <rainer.hillmer@gmx.de>,
"Ralf Dreyer" <ralf-dreyer@web.de>,
"Ralf Lukas" <wire-mail@gmx.de>,
"Rudi Bohn" <nicobohn1@web.de>,
"Sascha Geering" <sashgeer@aol.com>,
<Sascha.Geering@edeka.de>,
"Schrader Siegbert" <si.schrader@t-online.de>,
"Schulz Claudia" <sz.claudia.schulz@googlemail.com>,
"Thomas Runte" <thomas@etnur.de>,
<jatie@gmx.de>,
"Ute Boschmann" <info@ute-boschmann.de>,
"Wilhus Reiner" <reinerwilhus@t-online.de>,
"Wilhus Heidi" <heidiwilhus@t-online.de>,
"Yvonne und Tom Kanthak" <blacksilver1@t-online.de>"#;

        let parser = EmailParser::new();
        match parser.parse(EMAILS) {
            Ok(res_list) => {
                assert_eq!(res_list.len(), 36);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }
}