//! Four pure data entry points; all file access remains with the host.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$arity:expr,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED,arity:$arity,doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Parse or serialize bounded pure data without executing code or accessing files.",examples:&[$example],category:"Data"}});
    };}
    entry!(
        parse_json,
        "ParseJSON",
        Arity::Exactly(1),
        "parse_json(text)",
        "ParseJSON[text]",
        "有预算的JSON纯数据解析，拒绝重复键。",
        "ParseJSON[\"{\\\"a\\\":1}\"]"
    );
    entry!(
        parse_csv,
        "ParseCSV",
        Arity::Range(1, 2),
        "parse_csv(text,header:true)",
        "ParseCSV[text,Header->True]",
        "CSV字符串字段；保留表头的原生表格。",
        "ParseCSV[\"a,b\\n1,2\"]"
    );
    entry!(
        to_json,
        "ToJSON",
        Arity::Exactly(1),
        "to_json(value)",
        "ToJSON[value]",
        "JSON纯数据导出，保留有限十进制数值。",
        "ToJSON[Record[\"a\"->1]]"
    );
    entry!(
        to_csv,
        "ToCSV",
        Arity::Exactly(1),
        "to_csv(table)",
        "ToCSV[table]",
        "表格、记录或等宽行列表导出CSV。",
        "ToCSV[{{1,2},{3,4}}]"
    );
}
pub(super) fn dispatch(
    name: &str,
    args: &super::Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let value = match name {
        "ParseJSON" => super::json_data::parse(super::string(args.values[0])?, ctx)?,
        "ParseCSV" => super::csv_data::parse(
            super::string(args.values[0])?,
            args.boolean("Header", true)?,
            ctx,
        )?,
        "ToJSON" => Expr::string(&super::json_data::encode(args.values[0], ctx)?),
        "ToCSV" => Expr::string(&super::csv_data::encode(args.values[0], ctx)?),
        _ => return Ok(None),
    };
    Ok(Some(value))
}
