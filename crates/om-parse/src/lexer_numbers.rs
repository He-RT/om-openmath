//! Number boundaries and spelling validation; conversion belongs to the parser.

use super::{Scanner, TokenKind};
use crate::Dialect;

impl Scanner<'_> {
    pub(super) fn number(&mut self, start: usize) -> TokenKind {
        let separators = self.dialect != Dialect::Wolfram;
        let mut valid = true;
        if self.dialect != Dialect::Wolfram
            && (self.rest().starts_with("0x") || self.rest().starts_with("0X"))
        {
            self.pos += 2;
            let (digits, correct) = self.digits(16, separators);
            valid = correct && digits > 0;
        } else {
            let integer_start = self.pos;
            let (digits, correct) = self.digits(10, separators);
            valid &= correct;
            if self.rest().starts_with("^^") && digits > 0 {
                let base = self.src[integer_start..self.pos]
                    .replace('_', "")
                    .parse::<u32>()
                    .ok();
                self.pos += 2;
                let digit_start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
                    self.bump();
                }
                let radix = base.filter(|r| (2..=36).contains(r));
                valid &= self.pos > digit_start
                    && radix.is_some_and(|r| {
                        self.src[digit_start..self.pos]
                            .chars()
                            .all(|c| c.is_digit(r))
                    });
            } else {
                if self.peek() == Some('.') {
                    self.bump();
                    let (_, correct) = self.digits(10, separators);
                    valid &= correct;
                }
                if self.peek() == Some('`') {
                    self.bump();
                    let (_, correct) = self.digits(10, false);
                    valid &= correct;
                    if self.peek() == Some('.') {
                        self.bump();
                        let (_, correct) = self.digits(10, false);
                        valid &= correct;
                    }
                }
                let exponent = if self.rest().starts_with("*^") {
                    self.pos += 2;
                    true
                } else if self.dialect != Dialect::Wolfram
                    && self.peek().is_some_and(|c| c == 'e' || c == 'E')
                    && self
                        .rest()
                        .as_bytes()
                        .get(1)
                        .is_some_and(|c| c.is_ascii_digit() || *c == b'+' || *c == b'-')
                {
                    self.bump();
                    true
                } else {
                    false
                };
                if exponent {
                    if self.peek().is_some_and(|c| c == '+' || c == '-') {
                        self.bump();
                    }
                    let (digits, correct) = self.digits(10, separators);
                    valid &= correct && digits > 0;
                }
            }
        }
        if valid {
            TokenKind::Number
        } else {
            self.error(start, "E004", "非法数字格式")
        }
    }

    fn digits(&mut self, radix: u32, separators: bool) -> (usize, bool) {
        let mut digits = 0;
        let mut valid = true;
        let mut previous_digit = false;
        while let Some(c) = self.peek() {
            if c.is_ascii() && c.is_digit(radix) {
                self.bump();
                digits += 1;
                previous_digit = true;
            } else if separators && c == '_' {
                self.bump();
                valid &= previous_digit
                    && self
                        .peek()
                        .is_some_and(|c| c.is_ascii() && c.is_digit(radix));
                previous_digit = false;
            } else {
                break;
            }
        }
        (digits, valid)
    }
}
