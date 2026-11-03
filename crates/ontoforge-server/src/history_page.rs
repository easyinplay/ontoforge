use ontoforge_api::dto as wire;
use ontoforge_query::QueryError;
use ontoforge_storage::history::{HistoryError, HistoryRequest};
use ontoforge_storage::store::{ActiveDefinition, EditStore};

pub(crate) async fn read_history_page(
    store: &EditStore,
    mut request: HistoryRequest<'_>,
    active: &ActiveDefinition,
    max_bytes: usize,
) -> Result<wire::HistoryPage, QueryError> {
    request.limit = request.limit.clamp(1, 500);
    let entries = store
        .history_with_snapshot(&request, active)
        .await
        .map_err(|error| history_failure(&error))?;
    let has_older = if let Some(last) = entries.last() {
        request.before_seq = Some(last.seq);
        request.limit = 1;
        !store
            .history_with_snapshot(&request, active)
            .await
            .map_err(|error| history_failure(&error))?
            .is_empty()
    } else {
        false
    };
    let mut entries: Vec<_> = entries
        .into_iter()
        .map(crate::rest::history_entry)
        .collect();
    let mut entry_bytes = 0usize;
    let mut retained = 0;
    for (index, entry) in entries.iter().enumerate() {
        entry_bytes = entry_bytes
            .checked_add(
                serde_json::to_vec(entry)
                    .map_err(|_| QueryError::Store)?
                    .len(),
            )
            .ok_or(QueryError::CostExceeded)?;
        if entries
            .get(index + 1)
            .is_some_and(|next| next.seq == entry.seq)
        {
            continue;
        }
        let has_more = index + 1 < entries.len() || has_older;
        let envelope = wire::HistoryPage {
            entries: Vec::new(),
            truncated: has_more,
            next_before_seq: has_more.then_some(entry.seq),
        };
        let envelope_bytes = serde_json::to_vec(&envelope)
            .map_err(|_| QueryError::Store)?
            .len();
        let page_bytes = entry_bytes
            .checked_add(index)
            .and_then(|bytes| bytes.checked_add(envelope_bytes))
            .ok_or(QueryError::CostExceeded)?;
        if page_bytes > max_bytes {
            if retained == 0 {
                return Err(QueryError::CostExceeded);
            }
            break;
        }
        retained = index + 1;
    }
    let truncated = retained < entries.len() || has_older;
    entries.truncate(retained);
    let page = wire::HistoryPage {
        next_before_seq: entries.last().filter(|_| truncated).map(|entry| entry.seq),
        entries,
        truncated,
    };
    if page.entries.is_empty()
        && serde_json::to_vec(&page)
            .map_err(|_| QueryError::Store)?
            .len()
            > max_bytes
    {
        return Err(QueryError::CostExceeded);
    }
    Ok(page)
}

fn history_failure(error: &HistoryError) -> QueryError {
    match error {
        HistoryError::NotFound => QueryError::NotFound,
        HistoryError::Store(_) => QueryError::Store,
    }
}
