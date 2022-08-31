use anyhow::{anyhow, Result};
use log::trace;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::ChildStdout;

const BUFF_SIZE: usize = 1024 * 1024;

pub struct StdoutLineReader {
    buffer: Box<[u8; BUFF_SIZE]>,
    line_buf: String,
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
            line_buf: String::with_capacity(1024),
        }
    }

    pub(crate) fn unconsume(&mut self) {
        self.consumed = false;
    }

    pub(crate) async fn next_line(&mut self) -> Result<Option<&str>> {
        trace!("next_line: called");
        if !self.consumed {
            trace!("next_line: returning unconsumed buffer");
            self.consumed = true;
            Ok(Some(self.line_buf.as_str()))
        } else if self.finished {
            trace!("next_line: stream is finished");
            Ok(None)
        } else {
            self.line_buf.clear();
            loop {
                if self.read_pos < self.end_pos {
                    trace!("next_line: parsing buffer");
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
                            "next_line: found @offset {}, {} bytes ",
                            self.read_pos,
                            count
                        );
                        if count > 0 {
                            self.line_buf.push_str(&*String::from_utf8_lossy(
                                &self.buffer[self.read_pos..self.read_pos + count],
                            ));
                        }
                        self.read_pos += count + 1;
                        self.line_count += 1;
                        return Ok(Some(self.line_buf.as_str()));
                    } else {
                        // not found before the end of the buffer
                        trace!("next_line: not found, flushing buffer to line buffer");
                        self.line_buf.push_str(&*String::from_utf8_lossy(
                            &self.buffer[self.read_pos..self.end_pos],
                        ));
                        self.read_pos = self.end_pos;
                    }
                } else {
                    trace!("next_line: filling buffer");
                    self.end_pos = self.stream.read(&mut self.buffer[0..BUFF_SIZE]).await?;
                    if self.end_pos == 0 {
                        self.finished = true;
                        trace!("next_line: stream is finished");
                        if self.line_buf.is_empty() {
                            trace!("next_line: return None");
                            return Ok(None);
                        } else {
                            self.line_count += 1;
                            trace!("next_line: return previously parsed bytes");
                            return Ok(Some(self.line_buf.as_str()));
                        }
                    } else {
                        trace!("next_line: buffer refilled to {}", self.end_pos);
                        self.read_pos = 0;
                    }
                }
            }
        }
    }

    pub(crate) async fn flush(&mut self) -> Result<()> {
        self.finished = true;
        while self.stream.read(&mut self.buffer[0..]).await? > 0 {}
        Ok(())
    }

    #[allow(dead_code)]
    async fn expect_get_line(&mut self) -> Result<&str> {
        if let Some(res) = self.next_line().await? {
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

pub struct StdoutLineReader1 {
    // buffer: Box<[u8; BUFF_SIZE]>,
    line_buf: String,
    consumed: bool,
    finished: bool,
    reader: BufReader<ChildStdout>,
    line_count: usize,
}
impl StdoutLineReader1 {
    pub fn new(stream: ChildStdout) -> StdoutLineReader1 {
        StdoutLineReader1 {
            reader: BufReader::with_capacity(BUFF_SIZE, stream),
            line_count: 0,
            consumed: true,
            finished: false,
            line_buf: String::with_capacity(1024),
        }
    }

    pub(crate) fn unconsume(&mut self) {
        self.consumed = false;
    }

    pub(crate) async fn next_line(&mut self) -> Result<Option<&str>> {
        trace!("next_line: called");
        if !self.consumed {
            trace!("next_line: returning unconsumed buffer");
            self.consumed = true;
            Ok(Some(self.line_buf.as_str()))
        } else if self.finished {
            trace!("next_line: stream is finished");
            Ok(None)
        } else {
            self.line_buf.clear();
            trace!("next_line: reading line from stream");
            if self.reader.read_line(&mut self.line_buf).await? == 0 {
                trace!("next_line: stream is finished");
                self.finished = true;
                if self.line_buf.is_empty() {
                    trace!("next_line: returing None");
                    Ok(None)
                } else {
                    trace!("next_line: returing Some");
                    Ok(Some(self.line_buf.as_str()))
                }
            } else {
                self.line_buf.remove(self.line_buf.len() - 1);
                trace!("next_line: returing Some");
                Ok(Some(self.line_buf.as_str()))
            }
        }
    }

    pub(crate) async fn flush(&mut self) -> Result<()> {
        self.finished = true;
        let mut buffer = Box::new([0u8; BUFF_SIZE]); // 16K
        while self.reader.read(&mut buffer[0..]).await? > 0 {}
        Ok(())
    }

    #[allow(dead_code)]
    async fn expect_get_line(&mut self) -> Result<&str> {
        if let Some(res) = self.next_line().await? {
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
