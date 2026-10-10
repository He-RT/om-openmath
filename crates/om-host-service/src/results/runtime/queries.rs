use super::*;
use om_kernel::{
    protocol::{Dialect as KernelDialect, ValueQuery},
    retained_results::{ResultSourceFormat, RetainedGeometryQuery},
};
const MAX_REPLY_BYTES: usize = 256 * 1024;
fn projection_error(error: ResultError) -> String {
    match error {
        ResultError::Budget => "BUDGET_EXCEEDED".into(),
        ResultError::Invalid => "RESULT_NOT_AVAILABLE".into(),
        ResultError::Reference(ReferenceError::Expired | ReferenceError::Revoked) => {
            "RESULT_REF_EXPIRED".into()
        }
        ResultError::Reference(ReferenceError::PermissionDenied) => "PERMISSION_DENIED".into(),
        ResultError::Reference(_) => "INVALID_RESULT_REFERENCE".into(),
        ResultError::Projection(message) => {
            let mut end = message.len().min(240);
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message[..end].into()
        }
    }
}
fn encoded<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|_| "RESULT_ENCODING_FAILED".into())
}
fn source_fragment(
    text: String,
    offset: u64,
    limit: u32,
    ctx: &Interrupt,
) -> Result<Value, String> {
    if text.len() > 8 * 1024 * 1024 {
        return Err("SOURCE_BUDGET_EXCEEDED".into());
    }
    ctx.tick().map_err(|e| e.to_string())?;
    let offset = usize::try_from(offset).map_err(|_| "INVALID_SOURCE_OFFSET")?;
    if offset > text.len() || !text.is_char_boundary(offset) || limit == 0 || limit > 65536 {
        return Err("INVALID_SOURCE_OFFSET".into());
    }
    let mut end = offset
        .checked_add(limit as usize)
        .ok_or("SOURCE_BUDGET_EXCEEDED")?
        .min(text.len());
    while end > offset && !text.is_char_boundary(end) {
        end -= 1;
    }
    if end == offset && offset < text.len() {
        return Err("SOURCE_FRAGMENT_TOO_SMALL".into());
    }
    use sha2::Digest;
    let hash = format!("{:x}", sha2::Sha256::digest(text.as_bytes()));
    Ok(
        json!({"byte_offset":offset,"next_byte_offset":end,"total_utf8_bytes":text.len(),"source_hash":hash,"text":&text[offset..end],"complete":end==text.len()}),
    )
}
pub(super) fn execute(
    store: &mut ResultStore,
    command: &NativeResultHostCommand,
    scope: &ReferenceScope,
    now: u64,
    ctx: &Interrupt,
) -> Result<(Option<ResultBinding>, Value), String> {
    ctx.tick().map_err(|e| e.to_string())?;
    let (binding, payload) = match command {
        NativeResultHostCommand::ResultManifest(body) => (
            None,
            store
                .manifest(
                    &body.root_result_id,
                    scope,
                    body.offset,
                    body.limit,
                    now,
                    ctx,
                )
                .map_err(projection_error)?,
        ),
        NativeResultHostCommand::ResultRevoke(body) => {
            // Resolving first enforces the same document/grants before revoking another capability.
            let _ = store
                .resolve(&body.result_ref, scope, now)
                .map_err(projection_error)?;
            store
                .revoke_reference(&body.result_ref)
                .map_err(projection_error)?;
            (None, json!({"revoked":true}))
        }
        NativeResultHostCommand::ResultInspect(body) => {
            let result = store
                .resolve(&body.result_ref, scope, now)
                .map_err(projection_error)?;
            let binding = result.binding().clone();
            let payload = match &body.query {
                NativeResultQuery::InspectResultPresentation(q) => result
                    .presentation(q.offset, q.limit, ctx)
                    .map_err(projection_error)?,
                NativeResultQuery::InspectResultSummary(_) => {
                    let summary = result.summary();
                    let statement = summary.statements.iter().find(|s| {
                        u64::from(s.out_index) == binding.out_index.get()
                            && Some(s.view_id.as_str()) == binding.view_id.0.as_deref()
                    });
                    json!({"cell_id":summary.cell_id,"cell_kind":summary.cell_kind,"status":summary.status,"source_byte_length":summary.source_byte_length,"statement_count":summary.statements.len(),"statement":statement,"messages":result.messages()})
                }
                NativeResultQuery::InspectResultPage(q) => encoded(
                    result
                        .page(
                            &ValueQuery {
                                cell_id: binding.cell_id.clone(),
                                out_index: u32::try_from(binding.out_index.get())
                                    .map_err(|_| "INVALID_RESULT_OCCURRENCE")?,
                                view_id: binding.view_id.0.clone().ok_or("VALUE_NOT_AVAILABLE")?,
                                path: q.path.clone(),
                                offset: q.offset,
                                limit: q.limit,
                                column_offset: q.column_offset,
                                column_limit: q.column_limit,
                                include_source: false,
                            },
                            ctx,
                        )
                        .map_err(projection_error)?,
                )?,
                NativeResultQuery::InspectResultNumeric(q) => encoded(
                    result
                        .numeric(&q.path, q.digits, ctx)
                        .map_err(projection_error)?,
                )?,
                NativeResultQuery::InspectResultSteps(q) => encoded(
                    result
                        .steps(&q.parent_path, q.offset, q.limit, ctx)
                        .map_err(projection_error)?,
                )?,
                NativeResultQuery::InspectResultGeometry(_) => {
                    encoded(result.geometry(ctx).map_err(projection_error)?)?
                }
                NativeResultQuery::InspectResultGeometryPage(q) => {
                    let channel = serde_json::from_value(encoded(&q.channel)?)
                        .map_err(|_| "INVALID_GEOMETRY_CHANNEL")?;
                    encoded(
                        result
                            .geometry_page(
                                &RetainedGeometryQuery {
                                    channel,
                                    object_index: q.object_index,
                                    segment_index: q.segment_index,
                                    offset: q.offset,
                                    limit: q.limit,
                                },
                                ctx,
                            )
                            .map_err(projection_error)?,
                    )?
                }
                NativeResultQuery::InspectResultExpression(q) => encoded(
                    result
                        .readonly_expression(&q.source, q.numeric, ctx)
                        .map_err(projection_error)?,
                )?,
                NativeResultQuery::InspectResultScratch(q) => {
                    let dialect = match q.dialect {
                        Dialect::Modern => KernelDialect::Modern,
                        Dialect::Wolfram => KernelDialect::Wolfram,
                        Dialect::Auto => KernelDialect::Auto,
                    };
                    json!({"origin":"isolated_result_scratch","effect_committed":false,"response":result.scratch(q.source.clone(),dialect,ctx).map_err(projection_error)?})
                }
                NativeResultQuery::InspectResultSource(q) => {
                    let text = match q.format {
                        InspectResultSourceFormat::CellSource => {
                            if !q.path.is_empty() {
                                return Err("INVALID_RESULT_PATH".into());
                            }
                            result.cell_source()
                        }
                        InspectResultSourceFormat::StatementInput => {
                            if !q.path.is_empty() {
                                return Err("INVALID_RESULT_PATH".into());
                            }
                            result
                                .statement_input(ResultSourceFormat::InputForm, ctx)
                                .map_err(projection_error)?
                        }
                        InspectResultSourceFormat::InputForm => result
                            .source(&q.path, ResultSourceFormat::InputForm, ctx)
                            .map_err(projection_error)?,
                        InspectResultSourceFormat::Modern => result
                            .source(&q.path, ResultSourceFormat::Modern, ctx)
                            .map_err(projection_error)?,
                        InspectResultSourceFormat::Latex => result
                            .source(&q.path, ResultSourceFormat::Latex, ctx)
                            .map_err(projection_error)?,
                    };
                    source_fragment(text, q.byte_offset.get(), q.byte_limit, ctx)?
                }
            };
            (Some(binding), payload)
        }
        NativeResultHostCommand::ResultStatus(_) => return Err("INVALID_RESULT_COMMAND".into()),
    };
    if serde_json::to_vec(&payload)
        .map_err(|_| "RESULT_ENCODING_FAILED")?
        .len()
        > MAX_REPLY_BYTES
    {
        return Err("RESULT_PAGE_TOO_LARGE".into());
    }
    Ok((binding, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_fragments_keep_exact_utf8_and_never_call_a_partial_source_complete() {
        let first = source_fragment("a🙂中".into(), 0, 4, &Interrupt::default()).unwrap();
        assert_eq!(first["text"], "a");
        assert_eq!(first["next_byte_offset"], 1);
        assert_eq!(first["complete"], false);
        let second = source_fragment("a🙂中".into(), 1, 4, &Interrupt::default()).unwrap();
        assert_eq!(second["text"], "🙂");
        assert_eq!(second["next_byte_offset"], 5);
        assert_eq!(first["source_hash"], second["source_hash"]);
        let end = source_fragment("a🙂中".into(), 5, 4, &Interrupt::default()).unwrap();
        assert_eq!(end["text"], "中");
        assert_eq!(end["complete"], true);
        assert_eq!(end["total_utf8_bytes"], 8);
        assert!(source_fragment("a🙂中".into(), 2, 4, &Interrupt::default()).is_err());
        assert!(source_fragment("🙂".into(), 0, 1, &Interrupt::default()).is_err());
    }
}
