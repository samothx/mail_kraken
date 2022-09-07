use anyhow::{anyhow, Context, Result};
use log::{error, trace, warn};
use std::io::Read;
use std::process::ChildStdout;

const BUFF_SIZE: usize = 1024 * 1024;
const STR_BUFF_SIZE: usize = 1024 * 64; // 64K

pub struct StdoutLineReader {
    buffer: Box<[u8; BUFF_SIZE]>,
    line_buf: Vec<u8>,
    str_buf: Option<String>,
    rfc2047_buf: Option<String>,
    consumed: bool,
    finished: bool,
    stream: ChildStdout,
    line_count: usize,
    read_pos: usize,
    end_pos: usize,
}
impl StdoutLineReader {
    pub fn new(stream: ChildStdout) -> StdoutLineReader {
        StdoutLineReader {
            stream,
            read_pos: BUFF_SIZE,
            end_pos: BUFF_SIZE,
            buffer: Box::new([0; BUFF_SIZE]),
            line_count: 0,
            consumed: true,
            finished: false,
            line_buf: Vec::with_capacity(STR_BUFF_SIZE),
            str_buf: None,
            rfc2047_buf: None,
        }
    }

    pub(crate) fn unconsume(&mut self) {
        self.consumed = false;
    }

    fn next_line_raw(&mut self) -> Result<Option<&[u8]>> {
        trace!("next_line_raw: called");
        if !self.consumed {
            trace!("next_line_raw: returning unconsumed buffer");
            self.consumed = true;
            Ok(Some(&self.line_buf[..]))
        } else if self.finished {
            trace!("next_line_raw: stream is finished");
            Ok(None)
        } else {
            self.line_buf.clear();
            loop {
                if self.read_pos < self.end_pos {
                    trace!("next_line_raw: parsing buffer");
                    let mut count = 0;
                    let found = self.buffer[self.read_pos..self.end_pos]
                        .iter()
                        .enumerate()
                        .any(|(idx, ch)| {
                            if *ch == 0xA {
                                count = idx;
                                true
                            } else {
                                false
                            }
                        });

                    if found {
                        // found before the end of the buffer
                        trace!(
                            "next_line_raw: found @offset {}, {} bytes ",
                            self.read_pos,
                            count
                        );
                        if count > 0 {
                            self.line_buf.extend_from_slice(
                                &self.buffer[self.read_pos..self.read_pos + count],
                            );
                        }
                        self.read_pos += count + 1;
                        self.line_count += 1;
                        // TODO: decode RFC2047
                        return Ok(Some(&self.line_buf[..]));
                    } else {
                        // not found before the end of the buffer
                        trace!("next_line_raw: not found, flushing buffer to line buffer");
                        self.line_buf
                            .extend_from_slice(&self.buffer[self.read_pos..self.end_pos]);
                        self.read_pos = self.end_pos;
                    }
                } else {
                    trace!("next_line_raw: filling buffer");
                    self.end_pos = self.stream.read(&mut self.buffer[0..BUFF_SIZE])?;
                    if self.end_pos == 0 {
                        self.finished = true;
                        trace!("next_line_raw: stream is finished");
                        if self.line_buf.is_empty() {
                            trace!("next_line_raw: return None");
                            return Ok(None);
                        } else {
                            self.line_count += 1;
                            trace!("next_line_raw: return previously parsed bytes");
                            return Ok(Some(&self.line_buf[..]));
                        }
                    } else {
                        trace!("next_line_raw: buffer refilled to {}", self.end_pos);
                        self.read_pos = 0;
                    }
                }
            }
        }
    }

    pub(crate) fn next_line_rfc2047(&mut self) -> Result<Option<&str>> {
        trace!("next_line: called");
        self.str_buf = None;
        if !self.consumed {
            self.consumed = true;
            if self.rfc2047_buf.is_some() {
                Ok(Some(self.rfc2047_buf.as_deref().unwrap()))
            } else {
                Some(match rfc2047_decoder::decode(&self.line_buf[..]) {
                    Ok(res) => res,
                    Err(e) => {
                        let res = String::from_utf8_lossy(&self.line_buf[..]);
                        warn!(
                            "next_line_rfc2047 rfc2047_decoder::decode failed {:?} on [{}]",
                            e, res
                        );
                        res.to_string()
                    }
                });
                Ok(self.rfc2047_buf.as_deref())
            }
        } else if self.next_line_raw()?.is_some() {
            self.rfc2047_buf = Some(match rfc2047_decoder::decode(&self.line_buf[..]) {
                Ok(res) => res,
                Err(e) => {
                    let res = String::from_utf8_lossy(&self.line_buf[..]);
                    warn!(
                        "next_line_rfc2047 rfc2047_decoder::decode failed {:?} on [{}]",
                        e, res
                    );
                    res.to_string()
                }
            });
            Ok(self.rfc2047_buf.as_deref())
        } else {
            Ok(None)
        }
    }

    pub(crate) fn next_line(&mut self) -> Result<Option<&str>> {
        trace!("next_line: called");
        self.rfc2047_buf = None;
        if !self.consumed {
            self.consumed = true;
            if self.str_buf.is_some() {
                Ok(Some(self.str_buf.as_deref().unwrap()))
            } else {
                self.str_buf = Some((&*String::from_utf8_lossy(&self.line_buf[..])).to_owned());
                Ok(self.str_buf.as_deref())
            }
        } else if self.next_line_raw()?.is_some() {
            self.str_buf = Some((&*String::from_utf8_lossy(&self.line_buf[..])).to_owned());
            Ok(self.str_buf.as_deref())
        } else {
            Ok(None)
        }
    }

    pub(crate) fn flush(&mut self) -> Result<()> {
        self.finished = true;
        while self.stream.read(&mut self.buffer[0..])? > 0 {}
        Ok(())
    }

    #[allow(dead_code)]
    fn expect_get_line(&mut self) -> Result<&str> {
        if let Some(res) = self.next_line()? {
            Ok(res)
        } else {
            Err(anyhow!("encountered unexpected EOI"))
        }
    }

    #[allow(dead_code)]
    fn line_count(&self) -> usize {
        self.line_count
    }
}
