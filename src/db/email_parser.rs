use log::warn;
// use log::{debug, trace};
// use mod_logger::{Level, Logger};
use regex::bytes::Regex;

const EMAIL_REGEX: &str = r#"([-!#-'*+/-9=?A-Z^-~]+(\.[-!#-'*+/-9=?A-Z^-~]+)*|"([]!#-[^-~ \t]|(\\[\t -~]))+")@([-!#-'*+/-9=?A-Z^-~]+(\.[-!#-'*+/-9=?A-Z^-~]+)*|\[[\t -Z^-~]*])"#;

pub struct EmailParser {
    email_regex: Regex,
}

impl EmailParser {
    pub fn new() -> Self {
        Self {
            email_regex: Regex::new(EMAIL_REGEX)
                .expect(format!("failed to create regex from [{}]", EMAIL_REGEX).as_str()),
        }
    }

    pub fn parse(&self, emails: &str) -> Vec<(String, Option<String>, bool)> {
        // Logger::set_default_level(Level::Debug);
        let mut res = Vec::new();
        let mut state = State::Init;
        let mut collect = String::with_capacity(256);
        let mut email = String::with_capacity(256);
        let mut name = String::with_capacity(256);
        for ch in emails.chars() {
            /*            debug!(
                "parse: collect:[{}], name:[{}], email:[{}], state:{:?}",
                collect, name, email, state
            ); */
            match state {
                State::Init => match ch {
                    '<' => {
                        state = State::EmailBracket;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        let trimed = collect.trim();
                        if !trimed.is_empty() {
                            name.push_str(trimed);
                            collect.clear();
                        }
                    }
                    '"' => {
                        state = State::NameQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    '@' => {
                        state = State::Email;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        email.push_str(collect.trim_start());
                        email.push(ch);
                        collect.clear();
                    }
                    _ => collect.push(ch),
                },
                State::Name => match ch {
                    '<' => {
                        state = State::EmailBracket;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    _ => name.push(ch),
                },
                State::EmailBracket => match ch {
                    '>' => {
                        state = State::AfterEmail;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        let trimmed_name = name.trim();
                        res.push((
                            email.clone(),
                            if trimmed_name.is_empty() {
                                None
                            } else {
                                Some(trimmed_name.to_owned())
                            },
                        ));
                        email.clear();
                        name.clear();
                    }
                    _ => email.push(ch),
                },
                State::Email => match ch {
                    ',' => {
                        state = State::Init;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        res.push((
                            email.clone(),
                            if name.is_empty() {
                                None
                            } else {
                                Some(name.clone())
                            },
                        ));
                        email.clear();
                        name.clear();
                    }
                    ' ' => {
                        state = State::AfterEmail;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        res.push((
                            email.clone(),
                            if name.is_empty() {
                                None
                            } else {
                                Some(name.clone())
                            },
                        ));
                        email.clear();
                        name.clear();
                    }
                    _ => email.push(ch),
                },
                State::NameQuoted => match ch {
                    '\\' => {
                        state = State::EscapeName;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    '"' => {
                        state = State::Name;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    _ => name.push(ch),
                },
                State::EscapeName => {
                    if ch == '"' {
                        state = State::NameDoubleQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        name.push(ch);
                    }
                }
                State::AfterEmail => match ch {
                    ',' => {
                        state = State::Init;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    _ => (),
                },
                State::EscapeDoubleQuoted => {
                    if ch == '"' {
                        state = State::NameQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        name.push(ch);
                    } else {
                        name.push('\\');
                        name.push(ch);
                    }
                }
                State::NameDoubleQuoted => match ch {
                    '\\' => {
                        state = State::EscapeDoubleQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    _ => {
                        name.push(ch);
                    }
                },
            }
        }
        if state == State::Email {
            res.push((
                email.clone(),
                if name.is_empty() {
                    None
                } else {
                    Some(name.clone())
                },
            ));
            name.clear();
            email.clear();
        }
        // debug!("parse: state: {:?} res: {:?}", state, res);

        let mut res_final = Vec::with_capacity(res.len());
        for (email, name) in res {
            if self.email_regex.is_match(email.as_ref()) {
                res_final.push((email, name, true))
            } else {
                warn!("parse: invalid email: {}", email);
                warn!("parse: parsed from: {}", emails);
                res_final.push((email, name, false))
            }
        }
        res_final
    }
}
#[derive(Debug, PartialEq)]
enum State {
    Init,               // don't know
    EmailBracket,       // bracketed email
    Email,              // plain email
    NameQuoted,         // quoted name
    AfterEmail,         // after email
    EscapeName,         // escape in quoted name
    EscapeDoubleQuoted, // escape in double quoted name
    NameDoubleQuoted,   // double quoted name
    Name,
}

#[cfg(test)]
mod tests {
    use super::EmailParser;

    #[test]
    fn parse_email_simple_email1() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse("info@etnur.net"),
            vec![("info@etnur.net".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email2() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse("name@domain.tld,"),
            vec![("name@domain.tld".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email3() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse("name@domain.tld,\n"),
            vec![("name@domain.tld".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email4() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse("<name@domain.tld>,\n"),
            vec![("name@domain.tld".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email_with_name1() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse(r#"kurt mustermann <name@domain.tld>"#),
            vec![(
                "name@domain.tld".to_owned(),
                Some("kurt mustermann".to_owned()),
                true
            )]
        );
    }

    #[test]
    fn parse_email_simple_email_with_name2() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse(r#""Kauffmann, Ole" <Ole.Kauffmann@ipdynamics.de>"#),
            vec![(
                "Ole.Kauffmann@ipdynamics.de".to_owned(),
                Some("Kauffmann, Ole".to_owned()),
                true
            )]
        );
    }

    #[test]
    fn parse_email_simple_email_with_name3() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse(r#""\"Firmian\" Steinfath Mathias" <firmian@cenci.de>"#),
            vec![(
                "firmian@cenci.de".to_owned(),
                Some("\"Firmian\" Steinfath Mathias".to_owned()),
                true
            )]
        );
    }

    #[test]
    fn parse_email_simple_email_with_name4() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse(r#"Stölken, Christian <christian@domain.de>"#),
            vec![(
                "christian@domain.de".to_owned(),
                Some("Stölken, Christian".to_owned()),
                true
            )]
        );
    }

    #[test]
    fn parse_email_simple_email_list1() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse("name1@domain1.tld1, name2@domain2.tld2"),
            vec![
                ("name1@domain1.tld1".to_owned(), None, true),
                ("name2@domain2.tld2".to_owned(), None, true)
            ]
        );
    }

    #[test]
    fn parse_email_simple_email_list2() {
        let parser = EmailParser::new();
        assert_eq!(
            parser.parse("name1@domain1.tld1,\n name2@domain2.tld2,\n "),
            vec![
                ("name1@domain1.tld1".to_owned(), None, true),
                ("name2@domain2.tld2".to_owned(), None, true)
            ]
        );
    }

    #[test]
    fn parse_email_simple_email_list3() {
        const EMAILS: &str = r#"Ralf Dreyer <ralf-dreyer@web.de>, Olaf Völker
    <olaf@voelker-wl.de>, "\"Firmian\" Steinfath Mathias" <firmian@cenci.de>,
    Sascha Geering <sashgeer@aol.com>"#;

        let parser = EmailParser::new();
        assert_eq!(
            parser.parse(EMAILS),
            vec![
                (
                    "ralf-dreyer@web.de".to_owned(),
                    Some("Ralf Dreyer".to_owned()),
                    true
                ),
                (
                    "olaf@voelker-wl.de".to_owned(),
                    Some("Olaf Völker".to_owned()),
                    true
                ),
                (
                    "firmian@cenci.de".to_owned(),
                    Some("\"Firmian\" Steinfath Mathias".to_owned()),
                    true
                ),
                (
                    "sashgeer@aol.com".to_owned(),
                    Some("Sascha Geering".to_owned()),
                    true
                ),
            ]
        );
    }

    #[test]
    fn parse_email_complex_email_list1() {
        const EMAILS: &str = r#""\"Firmian\" Steinfath Mathias" <firmian@cenci.de>,
    "Andreas + Angela Horn" <aahorn@gmx.de>,
    "Benedict Dudda" <BenedictDudda@gmx.de>,
    Stölken, Christian <christian@domain.de>,
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
        let res_list = parser.parse(EMAILS);
        res_list.iter().for_each(|(email, name, valid)| {
            if !valid {
                panic!("invalid: {}, {:?}", email.as_str(), name)
            }
        });
        assert_eq!(res_list.len(), 37);
    }
}
