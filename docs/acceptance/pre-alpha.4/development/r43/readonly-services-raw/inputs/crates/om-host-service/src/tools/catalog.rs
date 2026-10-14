//! Readonly actual callback metadata. Native accepted definition ownership takes precedence.
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

/// A catalog query is data only and never authorizes evaluating an example.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    /// Closed argument shape/range/selector invalid.
    InvalidArgument,
    /// A supplied content revision does not describe this executable catalog.
    StaleMetadata,
    /// No actual callback/current user binding matches the selector.
    NotFound,
    /// Actual metadata cannot fit the closed result contract without truncating its meaning.
    LimitExceeded,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    query: String,
    category: Option<String>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Docs {
    function_id: Option<String>,
    name: Option<String>,
    metadata_version: Option<u32>,
}
fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(om_kernel::capabilities::function_documents)
}
fn input<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, CatalogError> {
    let value = om_agent_contract::json::strict_json_bounded(bytes, 65536)
        .map_err(|_| CatalogError::InvalidArgument)?;
    if !value.is_object()
        || value
            .as_object()
            .is_some_and(|o| o.values().any(Value::is_null))
    {
        return Err(CatalogError::InvalidArgument);
    }
    serde_json::from_value(value).map_err(|_| CatalogError::InvalidArgument)
}
fn bounded(value: &str, max: usize) -> bool {
    !value.is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control)
}
/// Search only compiled callbacks. Filtering order is deterministic and never executes sources.
pub fn search(bytes: &[u8]) -> Result<Value, CatalogError> {
    let args: Search = input(bytes)?;
    let limit = args.limit.unwrap_or(8);
    if !bounded(&args.query, 500)
        || !(1..=20).contains(&limit)
        || args.category.as_ref().is_some_and(|c| !bounded(c, 128))
    {
        return Err(CatalogError::InvalidArgument);
    }
    let query = args.query.to_lowercase();
    let mut matches = vec![];
    for f in data()["functions"].as_array().expect("actual catalog") {
        if args
            .category
            .as_ref()
            .is_some_and(|c| !f["category"].as_str().unwrap_or("").eq_ignore_ascii_case(c))
        {
            continue;
        }
        let descriptor = &f["descriptor"];
        let mut names = vec![
            descriptor["name"].as_str().unwrap_or(""),
            descriptor["modern_name"].as_str().unwrap_or(""),
            descriptor["id"].as_str().unwrap_or(""),
        ];
        if let Some(aliases) = descriptor["aliases"].as_array() {
            names.extend(aliases.iter().filter_map(Value::as_str));
        }
        let exact = names.iter().any(|n| n.eq_ignore_ascii_case(&args.query));
        let text = names
            .into_iter()
            .chain(
                [
                    "signature",
                    "wolfram_signature",
                    "summary_zh",
                    "summary_en",
                    "category",
                ]
                .iter()
                .filter_map(|key| f[*key].as_str()),
            )
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if !exact && !text.contains(&query) {
            continue;
        }
        matches.push((!exact,descriptor["name"].as_str().unwrap_or("").to_owned(),json!({"function_id":descriptor["id"],"name":descriptor["name"],"signature":f["signature"],"support":descriptor["support"],"metadata_version":data()["metadata_version"]})));
    }
    matches.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    super::checked_data("search_functions_data",json!({"functions":matches.into_iter().take(limit).map(|(_,_,f)|f).collect::<Vec<_>>(),"metadata_version":data()["metadata_version"]})).map_err(|_|CatalogError::LimitExceeded)
}
/// Query actual accepted user ownership, not syntactic declarations from notebook text.
/// The caller must retain/check the accepted state binding and currentness before publication.
pub fn docs(
    bytes: &[u8],
    context: &om_kernel::source::SourceExecutionContext,
) -> Result<Value, CatalogError> {
    let args: Docs = input(bytes)?;
    if args.function_id.is_some() == args.name.is_some()
        || args.function_id.as_ref().is_some_and(|s| !bounded(s, 80))
        || args.name.as_ref().is_some_and(|s| !bounded(s, 128))
    {
        return Err(CatalogError::InvalidArgument);
    }
    if args.metadata_version == Some(0) {
        return Err(CatalogError::InvalidArgument);
    }
    if args
        .metadata_version
        .is_some_and(|v| data()["metadata_version"].as_u64() != Some(u64::from(v)))
    {
        return Err(CatalogError::StaleMetadata);
    }
    if let Some(name) = &args.name {
        let owners = context
            .owned
            .iter()
            .map(|o| (o.symbol.as_str(), o.cell_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        if let Some(cell) = owners.get(name.as_str()) {
            return super::checked_data("get_function_docs_data",json!({"function":{"kind":"user_binding","name":name,"callable":context.known_functions.contains(name),"cell_id":cell,"source":null,"source_complete":false,"support":"Current accepted Session user binding overrides builtin alias; read original cell for complete source."},"examples":[],"metadata_version":data()["metadata_version"]})).map_err(|_|CatalogError::LimitExceeded);
        }
    }
    let entries = data()["functions"].as_array().expect("actual catalog");
    let found = if let Some(id) = args.function_id {
        entries.iter().find(|f| f["descriptor"]["id"] == id)
    } else {
        let name = args.name.expect("one selector");
        entries
            .iter()
            .find(|f| f["descriptor"]["name"] == name)
            .or_else(|| {
                entries.iter().find(|f| {
                    name.chars().count() >= 2
                        && (f["descriptor"]["modern_name"]
                            .as_str()
                            .is_some_and(|n| n.eq_ignore_ascii_case(&name))
                            || f["descriptor"]["aliases"].as_array().is_some_and(|a| {
                                a.iter().any(|v| {
                                    v.as_str().is_some_and(|s| s.eq_ignore_ascii_case(&name))
                                })
                            }))
                })
            })
    }
    .ok_or(CatalogError::NotFound)?;
    super::checked_data("get_function_docs_data",json!({"function":found,"examples":found["examples"],"metadata_version":data()["metadata_version"]})).map_err(|_|CatalogError::LimitExceeded)
}
