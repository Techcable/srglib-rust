pub type FnvIndexMap<K, V> = ::indexmap::IndexMap<K, V, ::fnv::FnvBuildHasher>;

pub struct SimpleParser<'a> {
    text: &'a str,
    remaining: &'a str
}
impl<'a> SimpleParser<'a> {
    #[inline]
    pub fn new(text: &'a str) -> SimpleParser<'a> {
        SimpleParser { text, remaining: text }
    }
    #[inline]
    pub fn peek(&mut self) -> Result<char, SimpleParseError> {
        self.remaining.chars().next().ok_or_else(|| self.error())
    }
    #[inline]
    pub fn peek_str(&mut self, size: usize) -> Result<&'a str, SimpleParseError> {
        self.remaining.get(..size).ok_or_else(|| self.error())
    }
    #[inline]
    pub fn take_until<F: FnMut(char) -> bool>(&mut self, func: F) -> &'a str {
        self.skip(self.remaining.find(func)
            .unwrap_or(self.remaining.len()))
    }
    #[inline]
    pub fn skip_whitespace(&mut self) {
        self.remaining = self.remaining.trim_left();
    }
    #[inline]
    pub fn skip(&mut self, amount: usize) -> &'a str {
        let (taken, remaining) = self.remaining.split_at(amount);
        self.remaining = remaining;
        taken
    }
    pub fn expect(&mut self, expected: char) -> Result<(), SimpleParseError> {
        let actual = self.peek().ok();
        if actual == Some(expected) {
            self.skip(expected.len_utf8());
            Ok(())
        } else {
            Err(SimpleParseError {
                index: self.current_index(),
                reason: Some(format!("Expected {:?}, but got {:?}", expected, actual))
            })
        }
    }
    pub fn expect_str(&mut self, s: &str) -> Result<(), SimpleParseError> {
        if self.remaining.starts_with(s) {
            self.skip(s.len());
            Ok(())
        } else {
            Err(self.error())
        }
    }
    #[inline]
    pub fn parse<T: SimpleParse>(&mut self) -> Result<T, SimpleParseError> {
        T::parse(self)
    }
    #[inline]
    pub fn parse_internal_name(&mut self) -> Result<&'a str, SimpleParseError> {
        let start = self.current_index();
        let s = self.take_until(|c| c == ' ');
        if let Some(bad_index) = s.find('.') {
            Err(SimpleParseError { index: start + bad_index, reason: Some(format!("Invalid internal name: {:?}", s)) })
        } else {
            Ok(s)
        }
    }
    #[inline]
    pub fn error(&self) -> SimpleParseError {
        SimpleParseError { index: self.current_index(), reason: None }
    }
    #[inline]
    pub fn current_index(&self) -> usize {
        self.text.len() - self.remaining.len()
    }
    #[inline]
    pub fn remaining(&self) -> &'a str {
        self.remaining
    }
    pub fn original(&self) -> &'a str { self.text }
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.remaining.is_empty()
    }
    #[inline]
    pub fn ensure_finished(&self) -> Result<(), SimpleParseError> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(self.error())
        }
    }
}
pub trait SimpleParse: Sized {
    fn parse(parser: &mut SimpleParser) -> Result<Self, SimpleParseError>;
    fn parse_fully(parser: &mut SimpleParser) -> Result<Self, SimpleParseError> {
        let value = Self::parse(parser)?;
        parser.ensure_finished()?;
        Ok(value)
    }
    #[inline]
    fn parse_text(text: &str) -> Result<Self, SimpleParseError> {
        let mut parser = SimpleParser::new(text);
        Self::parse_fully(&mut parser)
    }
}

pub struct SimpleParseError {
    pub index: usize,
    pub reason: Option<String>
}
