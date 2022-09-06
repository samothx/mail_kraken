use log::warn;
use regex::bytes::Regex;

const EMAIL_REGEX: &str = r#"([-!#-'*+/-9=?A-Z^-~]+(\.[-!#-'*+/-9=?A-Z^-~]+)*|"([]!#-[^-~ \t]|(\\[\t -~]))+")@([-!#-'*+/-9=?A-Z^-~]+(\.[-!#-'*+/-9=?A-Z^-~]+)*|\[[\t -Z^-~]*])"#;

pub struct EmailParser {
    email_regex: Regex,
    collect: String,
    email: String,
    name: String,
}

impl EmailParser {
    pub fn new() -> Self {
        Self {
            email_regex: Regex::new(EMAIL_REGEX)
                .expect(format!("failed to create regex from [{}]", EMAIL_REGEX).as_str()),
            collect: String::with_capacity(256),
            email: String::with_capacity(256),
            name: String::with_capacity(256),
        }
    }

    pub fn parse(&mut self, emails: &str) -> Vec<(String, Option<String>, bool)> {
        // Logger::set_default_level(Level::Debug);
        let mut res = Vec::new();
        let mut state = State::Init;

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
                        let trimed = self.collect.trim();
                        if !trimed.is_empty() {
                            self.name.push_str(trimed);
                            self.collect.clear();
                        }
                    }
                    '"' => {
                        state = State::NameQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    '@' => {
                        state = State::Email;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        self.email.push_str(self.collect.trim_start());
                        self.email.push(ch);
                        self.collect.clear();
                    }
                    _ => self.collect.push(ch),
                },
                State::Name => match ch {
                    '<' => {
                        state = State::EmailBracket;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    _ => self.name.push(ch),
                },
                State::EmailBracket => match ch {
                    '>' => {
                        state = State::AfterEmail;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        let trimmed_email = self.email.trim();
                        if trimmed_email.is_empty() {
                            warn!("parse: empty email brackets found in [{}]", emails);
                        } else {
                            let trimmed_name = self.name.trim();
                            res.push((
                                trimmed_email.to_lowercase().to_owned(),
                                if trimmed_name.is_empty() {
                                    None
                                } else {
                                    Some(trimmed_name.to_owned())
                                },
                            ));
                        }
                        self.email.clear();
                        self.name.clear();
                    }
                    _ => self.email.push(ch),
                },
                State::Email => match ch {
                    ',' => {
                        state = State::Init;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        res.push((
                            self.email.trim().to_lowercase().to_owned(),
                            if self.name.is_empty() {
                                None
                            } else {
                                Some(self.name.trim().to_owned())
                            },
                        ));
                        self.email.clear();
                        self.name.clear();
                    }
                    ' ' => {
                        state = State::AfterEmail;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        let trimmed_email = self.email.trim();
                        let trimmed_name = self.name.trim();
                        res.push((
                            trimmed_email.to_lowercase().to_owned(),
                            if trimmed_name.is_empty() {
                                None
                            } else {
                                Some(trimmed_name.to_owned())
                            },
                        ));
                        self.email.clear();
                        self.name.clear();
                    }
                    _ => self.email.push(ch),
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
                    _ => self.name.push(ch),
                },
                State::EscapeName => {
                    if ch == '"' {
                        state = State::NameDoubleQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        self.name.push(ch);
                    } else {
                        state = State::NameQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                        self.name.push(ch);
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
                    } else {
                        state = State::NameDoubleQuoted;
                    }
                    // trace!("parse:   -> {:?} on {}", state, ch);
                    self.name.push(ch);
                }
                State::NameDoubleQuoted => match ch {
                    '\\' => {
                        state = State::EscapeDoubleQuoted;
                        // trace!("parse:   -> {:?} on {}", state, ch);
                    }
                    _ => {
                        self.name.push(ch);
                    }
                },
            }
        }

        if state == State::Email {
            res.push((
                self.email.to_lowercase().clone(),
                if self.name.is_empty() {
                    None
                } else {
                    Some(self.name.clone())
                },
            ));
            self.name.clear();
            self.email.clear();
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
        let mut parser = EmailParser::new();
        assert_eq!(
            parser.parse("info@etnur.net"),
            vec![("info@etnur.net".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email2() {
        let mut parser = EmailParser::new();
        assert_eq!(
            parser.parse("name@domain.tld,"),
            vec![("name@domain.tld".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email3() {
        let mut parser = EmailParser::new();
        assert_eq!(
            parser.parse("name@domain.tld,\n"),
            vec![("name@domain.tld".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email4() {
        let mut parser = EmailParser::new();
        assert_eq!(
            parser.parse("<name@domain.tld>,\n"),
            vec![("name@domain.tld".to_owned(), None, true)]
        );
    }

    #[test]
    fn parse_email_simple_email5() {
        let mut parser = EmailParser::new();
        assert_eq!(
            parser.parse(r#""James Wei \(via Dropbox\)" <no-reply@dropbox.com>"#),
            vec![(
                "no-reply@dropbox.com".to_owned(),
                Some(r#"James Wei (via Dropbox)"#.to_owned()),
                true
            )]
        );
    }

    #[test]
    fn parse_email_simple_email6() {
        let mut parser = EmailParser::new();
        assert_eq!(
            parser.parse(r#""Bob at /\\/\\etBob" <bob@metbob.com>"#),
            vec![(
                "bob@metbob.com".to_owned(),
                Some(r#"Bob at /\/\etBob"#.to_owned()),
                true
            )]
        );
    }

    #[test]
    fn parse_email_simple_email7() {
        let mut parser = EmailParser::new();
        assert_eq!(parser.parse(r#"Root User <>"#), vec![]);
    }

    #[test]
    fn parse_email_simple_email_with_name1() {
        let mut parser = EmailParser::new();
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
        let mut parser = EmailParser::new();
        assert_eq!(
            parser.parse(r#""Kauffmann, Ole" <Ole.Kauffmann@ipdynamics.de>"#),
            vec![(
                "ole.kauffmann@ipdynamics.de".to_owned(),
                Some("Kauffmann, Ole".to_owned()),
                true
            )]
        );
    }

    #[test]
    fn parse_email_simple_email_with_name3() {
        let mut parser = EmailParser::new();
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
        let mut parser = EmailParser::new();
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
        let mut parser = EmailParser::new();
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
        let mut parser = EmailParser::new();
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

        let mut parser = EmailParser::new();
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

        let mut parser = EmailParser::new();
        let res_list = parser.parse(EMAILS);
        res_list.iter().for_each(|(email, name, valid)| {
            if !valid {
                panic!("invalid: {}, {:?}", email.as_str(), name)
            }
        });
        assert_eq!(res_list.len(), 37);
    }
}