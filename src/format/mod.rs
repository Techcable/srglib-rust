use std::io::{self, BufRead, Write};

use failure_derive::Fail;

use crate::prelude::*;

pub mod srg;
pub mod csrg;

#[derive(Debug, Fail)]
pub enum MappingsParseError {
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
    // TODO: Somehow include reason
    #[fail(display = "Invalid line at {}: {:?}", index, line)]
    InvalidLine {
        line: String,
        index: usize,
        reason: Option<String>
    }
}
impl From<io::Error> for MappingsParseError {
    #[inline]
    fn from(e: io::Error) -> Self {
        MappingsParseError::Io(e)
    }
}

pub trait MappingsFormat {
    type Processor: MappingsLineProcessor;
    fn parse_stream<R: BufRead>(mut read: R) -> Result<FrozenMappings, MappingsParseError> {
        let mut buffer = String::new();
        let mut processer = Self::processor();
        loop {
            buffer.clear();
            if read.read_line(&mut buffer)? == 0 { break }
            processer.process_line(&buffer)?;
        }
        processer.finish()
    }
    fn parse_lines<I: IntoIterator>(lines: I) -> Result<FrozenMappings, MappingsParseError>
        where I::Item: AsRef<str>  {
        let mut processer = Self::processor();
        for line in lines {
            processer.process_line(line.as_ref())?;
        }
        processer.finish()
    }
    fn parse_text(text: &str) -> Result<FrozenMappings, MappingsParseError> {
        Self::parse_lines(text.lines())
    }
    fn write<'a, T: IterableMappings<'a>, W: Write>(mappings: &'a T, writer: W) -> io::Result<()>;
    fn write_line_array<'a, T: IterableMappings<'a>>(mappings: &'a T) -> Vec<String> {
        Self::write_string(mappings).lines().map(String::from).collect()
    }
    fn write_string<'a, T: IterableMappings<'a>>(mappings: &'a T) -> String {
        let mut buffer = Vec::new();
        Self::write(mappings, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
    fn processor() -> Self::Processor;
}
pub trait MappingsLineProcessor {
    fn process_line(&mut self, s: &str) -> Result<(), MappingsParseError>;
    fn finish(self) -> Result<FrozenMappings, MappingsParseError>;
}