//! CSV fields remain text; header tables retain their schema even with zero rows.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
const LIMIT: usize = 8 * 1024 * 1024;
struct Scanner<'a> {
    text: &'a str,
    at: usize,
    ctx: &'a Interrupt,
}
impl Scanner<'_> {
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
    fn field(&mut self) -> Result<String, EvalError> {
        let mut bytes = vec![];
        if self.peek() == Some(b'"') {
            self.bump()?;
            loop {
                match self.bump()? {
                    Some(b'"') => {
                        if self.peek() == Some(b'"') {
                            self.bump()?;
                            bytes.push(b'"');
                        } else {
                            break;
                        }
                    }
                    Some(c) => bytes.push(c),
                    None => return Err(error("CSV引号字段未结束")),
                }
            }
            if !matches!(self.peek(), None | Some(b',' | b'\r' | b'\n')) {
                return Err(error("CSV关闭引号后只能接分隔符"));
            }
        } else {
            while !matches!(self.peek(), None | Some(b',' | b'\r' | b'\n')) {
                match self.bump()? {
                    Some(b'"') => return Err(error("CSV未加引号的字段含引号")),
                    Some(c) => bytes.push(c),
                    None => break,
                }
            }
        }
        String::from_utf8(bytes).map_err(|_| error("CSV字段UTF-8无效"))
    }
}
pub(super) fn parse(text: &str, header: bool, ctx: &Interrupt) -> Result<Expr, EvalError> {
    if text.len() > LIMIT {
        return Err(error("CSV输入超过8MiB限制"));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut scan = Scanner { text, at: 0, ctx };
    let mut rows: Vec<Vec<Expr>> = vec![];
    let mut fields = 0;
    if !text.is_empty() {
        'rows: loop {
            let mut row = vec![];
            loop {
                ctx.tick()?;
                fields += 1;
                if fields > 100000 {
                    return Err(error("CSV字段超过100000项限制"));
                }
                row.push(Expr::string(&scan.field()?));
                match scan.bump()? {
                    Some(b',') => {}
                    Some(b'\n') => break,
                    Some(b'\r') => {
                        if scan.bump()? != Some(b'\n') {
                            return Err(error("CSV行结束使用LF或CRLF"));
                        }
                        break;
                    }
                    None => {
                        rows.push(row);
                        break 'rows;
                    }
                    _ => return Err(error("CSV分隔符无效")),
                }
            }
            rows.push(row);
            if scan.at == text.len() {
                break;
            }
        }
    }
    let width = rows.first().map_or(0, Vec::len);
    if rows.iter().any(|r| r.len() != width) {
        return Err(error("CSV行宽不一致"));
    }
    if !header {
        return Ok(list(rows.into_iter().map(list)));
    }
    let columns = if rows.is_empty() {
        vec![]
    } else {
        rows.remove(0)
    };
    let mut keys = BTreeSet::new();
    for column in &columns {
        ctx.tick()?;
        if !keys.insert(string(column)?) {
            return Err(error("CSV表头重复"));
        }
    }
    let records = rows.into_iter().map(|row| {
        Expr::call(
            B::RECORD,
            columns
                .iter()
                .cloned()
                .zip(row)
                .map(|(k, v)| Expr::call(B::RULE, [k, v])),
        )
    });
    Ok(Expr::call(
        B::DATA_TABLE,
        [list(columns.clone()), list(records)],
    ))
}
pub(super) fn entries<'a>(
    e: &'a Expr,
    ctx: &Interrupt,
) -> Result<BTreeMap<&'a str, &'a Expr>, EvalError> {
    if !e.is_head(B::RECORD) {
        return Err(error("表格数据行需要记录"));
    }
    let mut fields = BTreeMap::new();
    for entry in e.args() {
        ctx.tick()?;
        if !entry.is_head(B::RULE) || entry.args().len() != 2 {
            return Err(error("记录字段结构无效"));
        }
        let key = string(&entry.args()[0])?;
        if fields.insert(key, &entry.args()[1]).is_some() {
            return Err(error("记录字段重复"));
        }
    }
    Ok(fields)
}
pub(super) fn validate_table(e: &Expr, ctx: &Interrupt) -> Result<(), EvalError> {
    if !e.is_head(B::DATA_TABLE)
        || e.args().len() != 2
        || !e.args()[0].is_head(B::LIST)
        || !e.args()[1].is_head(B::LIST)
    {
        return Err(error("表格需要columns/rows列表"));
    }
    let mut columns = BTreeSet::new();
    for column in e.args()[0].args() {
        ctx.tick()?;
        if !columns.insert(string(column)?) {
            return Err(error("表格列重复"));
        }
    }
    let mut fields = columns.len();
    if columns.is_empty() && !e.args()[1].args().is_empty() {
        return Err(error("零列表格不能含数据行"));
    }
    for row in e.args()[1].args() {
        let row = entries(row, ctx)?;
        fields += row.len();
        if fields > 100000 {
            return Err(error("表格字段超过100000项限制"));
        }
        if row.len() != columns.len() || row.keys().any(|k| !columns.contains(k)) {
            return Err(error("记录字段与表格列不一致"));
        }
    }
    Ok(())
}
struct Writer<'a> {
    text: String,
    ctx: &'a Interrupt,
}
impl Writer<'_> {
    fn push(&mut self, text: &str) -> Result<(), EvalError> {
        if self.text.len().saturating_add(text.len()) > LIMIT {
            return Err(error("CSV输出超过8MiB限制"));
        }
        for _ in text.bytes() {
            self.ctx.tick()?;
        }
        self.text.push_str(text);
        Ok(())
    }
    fn field(&mut self, e: &Expr) -> Result<(), EvalError> {
        let text = match e.kind() {
            ExprKind::String(s) => s.to_string(),
            ExprKind::Number(_) => super::data_number::encode(e, self.ctx)?,
            ExprKind::Symbol(s) if *s == B::NULL => String::new(),
            ExprKind::Symbol(s) if *s == B::TRUE => "true".into(),
            ExprKind::Symbol(s) if *s == B::FALSE => "false".into(),
            _ => return Err(error("CSV单元格只接受字符串、有限数、布尔或Null")),
        };
        if text.contains([',', '"', '\r', '\n']) || text.is_empty() {
            self.push("\"")?;
            for part in text.split_inclusive('"') {
                self.push(part)?;
                if part.ends_with('"') {
                    self.push("\"")?;
                }
            }
            self.push("\"")
        } else {
            self.push(&text)
        }
    }
    fn row<'a>(&mut self, row: impl IntoIterator<Item = &'a Expr>) -> Result<(), EvalError> {
        for (i, e) in row.into_iter().enumerate() {
            if i > 0 {
                self.push(",")?;
            }
            self.field(e)?;
        }
        self.push("\r\n")
    }
}
pub(crate) fn encode(e: &Expr, ctx: &Interrupt) -> Result<String, EvalError> {
    let mut writer = Writer {
        text: String::new(),
        ctx,
    };
    if e.is_head(B::DATA_TABLE) {
        validate_table(e, ctx)?;
        let columns = e.args()[0].args();
        if !columns.is_empty() {
            writer.row(columns)?;
        }
        for row in e.args()[1].args() {
            let fields = entries(row, ctx)?;
            writer.row(
                columns
                    .iter()
                    .map(|k| fields[string(k).expect("invariant: validated column string")]),
            )?;
        }
    } else {
        if !e.is_head(B::LIST) {
            return Err(error("CSV导出需要表格或行列表"));
        }
        let rows = e.args();
        if rows.is_empty() {
            return Ok(String::new());
        }
        if rows[0].is_head(B::RECORD) {
            let columns: Vec<_> = rows[0]
                .args()
                .iter()
                .map(|e| {
                    e.args()
                        .first()
                        .cloned()
                        .ok_or_else(|| error("记录字段结构无效"))
                })
                .collect::<Result<_, _>>()?;
            return encode(&Expr::call(B::DATA_TABLE, [list(columns), e.clone()]), ctx);
        }
        let width = rows[0].args().len();
        if width == 0 || rows.len().saturating_mul(width) > 100000 {
            return Err(error("CSV行宽或字段数无效"));
        }
        for row in rows {
            ctx.tick()?;
            if !row.is_head(B::LIST) || row.args().len() != width {
                return Err(error("CSV导出需要非空等宽行列表"));
            }
            writer.row(row.args())?;
        }
    }
    Ok(writer.text)
}
