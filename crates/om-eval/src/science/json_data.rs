//! RFC8259 data scanning never calls the source parser or evaluator.
use super::*;
use std::collections::BTreeSet;
const LIMIT: usize = 8 * 1024 * 1024;
struct Parser<'a> {
    text: &'a str,
    at: usize,
    nodes: usize,
    ctx: &'a Interrupt,
}
impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }
    fn bump(&mut self) -> Result<Option<u8>, EvalError> {
        self.ctx.tick()?;
        let c = self.peek();
        if c.is_some() {
            self.at += 1;
        }
        Ok(c)
    }
    fn space(&mut self) -> Result<(), EvalError> {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.bump()?;
        }
        Ok(())
    }
    fn expect(&mut self, c: u8) -> Result<(), EvalError> {
        self.space()?;
        if self.bump()? == Some(c) {
            Ok(())
        } else {
            Err(error("JSON分隔符或结构无效"))
        }
    }
    fn quoted(&mut self) -> Result<String, EvalError> {
        self.space()?;
        let start = self.at;
        if self.bump()? != Some(b'"') {
            return Err(error("JSON键需要字符串"));
        }
        loop {
            match self.bump()? {
                Some(b'"') => break,
                Some(b'\\') => {
                    if self.bump()?.is_none() {
                        return Err(error("JSON字符串转义未结束"));
                    }
                }
                Some(_) => {}
                None => return Err(error("JSON字符串未结束")),
            }
        }
        serde_json::from_str(&self.text[start..self.at])
            .map_err(|_| error("JSON字符串或Unicode转义无效"))
    }
    fn digits(&mut self) -> Result<usize, EvalError> {
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump()?;
        }
        Ok(self.at - start)
    }
    fn number(&mut self) -> Result<Expr, EvalError> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.bump()?;
        }
        match self.peek() {
            Some(b'0') => {
                self.bump()?;
            }
            Some(b'1'..=b'9') => {
                self.digits()?;
            }
            _ => return Err(error("JSON数值无效")),
        }
        if self.peek() == Some(b'.') {
            self.bump()?;
            if self.digits()? == 0 {
                return Err(error("JSON小数缺少数字"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.bump()?;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump()?;
            }
            if self.digits()? == 0 {
                return Err(error("JSON指数缺少数字"));
            }
        }
        Ok(Expr::number(
            Number::Rational(super::data_number::decimal(
                &self.text[start..self.at],
                self.ctx,
            )?)
            .normalize(),
        ))
    }
    fn value(&mut self, depth: usize) -> Result<Expr, EvalError> {
        self.ctx.tick()?;
        self.space()?;
        self.nodes += 1;
        if depth > 64 || self.nodes > 100000 {
            return Err(error("JSON嵌套或节点超过资源界限"));
        }
        match self.peek() {
            Some(b'"') => Ok(Expr::string(&self.quoted()?)),
            Some(b'[') => {
                self.bump()?;
                self.space()?;
                let mut values = vec![];
                if self.peek() != Some(b']') {
                    loop {
                        values.push(self.value(depth + 1)?);
                        self.space()?;
                        if self.peek() != Some(b',') {
                            break;
                        }
                        self.bump()?;
                    }
                }
                self.expect(b']')?;
                Ok(list(values))
            }
            Some(b'{') => {
                self.bump()?;
                self.space()?;
                let mut values = vec![];
                let mut keys = BTreeSet::new();
                if self.peek() != Some(b'}') {
                    loop {
                        let key = self.quoted()?;
                        if !keys.insert(key.clone()) {
                            return Err(error("JSON解码后键重复"));
                        }
                        self.expect(b':')?;
                        values.push(Expr::call(
                            B::RULE,
                            [Expr::string(&key), self.value(depth + 1)?],
                        ));
                        self.space()?;
                        if self.peek() != Some(b',') {
                            break;
                        }
                        self.bump()?;
                    }
                }
                self.expect(b'}')?;
                Ok(Expr::call(B::RECORD, values))
            }
            Some(b't' | b'f' | b'n') => {
                let (word, symbol) = match self.peek() {
                    Some(b't') => ("true", B::TRUE),
                    Some(b'f') => ("false", B::FALSE),
                    _ => ("null", B::NULL),
                };
                for byte in word.bytes() {
                    if self.bump()? != Some(byte) {
                        return Err(error("JSON字面值无效"));
                    }
                }
                Ok(Expr::sym(symbol))
            }
            _ => self.number(),
        }
    }
}
pub(super) fn parse(text: &str, ctx: &Interrupt) -> Result<Expr, EvalError> {
    if text.len() > LIMIT {
        return Err(error("JSON输入超过8MiB限制"));
    }
    let mut parser = Parser {
        text,
        at: 0,
        nodes: 0,
        ctx,
    };
    let value = parser.value(0)?;
    parser.space()?;
    if parser.at != text.len() {
        return Err(error("JSON末尾有多余数据"));
    }
    Ok(value)
}
struct Writer<'a> {
    text: String,
    nodes: usize,
    ctx: &'a Interrupt,
}
impl Writer<'_> {
    fn push(&mut self, s: &str) -> Result<(), EvalError> {
        if self.text.len().saturating_add(s.len()) > LIMIT {
            return Err(error("JSON输出超过8MiB限制"));
        }
        for _ in s.bytes() {
            self.ctx.tick()?;
        }
        self.text.push_str(s);
        Ok(())
    }
    fn quoted(&mut self, s: &str) -> Result<(), EvalError> {
        if s.len() > LIMIT {
            return Err(error("字符串超过8MiB限制"));
        }
        for _ in s.bytes() {
            self.ctx.tick()?;
        }
        let encoded = serde_json::to_string(s).map_err(|_| error("字符串编码失败"))?;
        self.push(&encoded)
    }
    fn value(&mut self, e: &Expr, depth: usize) -> Result<(), EvalError> {
        self.ctx.tick()?;
        self.nodes += 1;
        if depth > 64 || self.nodes > 100000 {
            return Err(error("数据嵌套或节点超过资源界限"));
        }
        match e.kind() {
            ExprKind::String(s) => self.quoted(s),
            ExprKind::Number(_) => self.push(&super::data_number::encode(e, self.ctx)?),
            ExprKind::Symbol(s) if matches!(*s, B::TRUE | B::FALSE | B::NULL) => {
                self.push(match *s {
                    B::TRUE => "true",
                    B::FALSE => "false",
                    _ => "null",
                })
            }
            _ if e.is_head(B::LIST) => {
                self.push("[")?;
                for (i, e) in e.args().iter().enumerate() {
                    if i > 0 {
                        self.push(",")?;
                    }
                    self.value(e, depth + 1)?;
                }
                self.push("]")
            }
            _ if e.is_head(B::RECORD) => {
                self.push("{")?;
                let mut keys = BTreeSet::new();
                for (i, entry) in e.args().iter().enumerate() {
                    self.ctx.tick()?;
                    if !entry.is_head(B::RULE) || entry.args().len() != 2 {
                        return Err(error("记录键值结构无效"));
                    }
                    let key = string(&entry.args()[0])?;
                    if !keys.insert(key) {
                        return Err(error("记录键重复"));
                    }
                    if i > 0 {
                        self.push(",")?;
                    }
                    self.quoted(key)?;
                    self.push(":")?;
                    self.value(&entry.args()[1], depth + 1)?;
                }
                self.push("}")
            }
            _ if e.is_head(B::DATA_TABLE) => {
                super::csv_data::validate_table(e, self.ctx)?;
                self.value(
                    &record([
                        ("columns", e.args()[0].clone()),
                        ("rows", e.args()[1].clone()),
                    ]),
                    depth + 1,
                )
            }
            _ => Err(error("JSON只接受纯数据，不能导出未求值数学式或代码")),
        }
    }
}
pub(super) fn encode(e: &Expr, ctx: &Interrupt) -> Result<String, EvalError> {
    let mut writer = Writer {
        text: String::new(),
        nodes: 0,
        ctx,
    };
    writer.value(e, 0)?;
    Ok(writer.text)
}
