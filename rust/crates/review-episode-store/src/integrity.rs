use review_episode_core::codec::{JsValue, field};
use review_episode_core::state::ReviewEpisodeState;

use crate::{StoreError, StoreResult};

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub sequence: i64,
    pub identity_key: String,
    pub revision: String,
    pub predecessor_revision: Option<String>,
    pub state_json: String,
    pub state: ReviewEpisodeState,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub current: Option<ReviewEpisodeState>,
    pub history: Vec<HistoryEntry>,
}

pub fn validate_history(
    key: &str,
    current: Option<(&str, &str)>,
    rows: Vec<(i64, String, String, Option<String>, String)>,
) -> StoreResult<Snapshot> {
    let mut history: Vec<HistoryEntry> = Vec::with_capacity(rows.len());
    for (sequence, identity_key, revision, predecessor_revision, state_json) in rows {
        let location =
            format!("review_episode_history identity_key={identity_key:?} sequence={sequence}");
        let rejected = |detail: String| StoreError::Integrity(format!("{location}: {detail}"));
        if sequence <= 0 || identity_key != key {
            return Err(rejected(format!(
                "sequence or identity key differs from requested identity {key:?}"
            )));
        }
        let state =
            ReviewEpisodeState::from_stored_json(&state_json).map_err(|e| rejected(e.message))?;
        if state.identity().key().0 != key || state.revision().0 != revision {
            return Err(rejected(
                "row disagrees with embedded identity/revision".into(),
            ));
        }
        if let Some(prior) = history.last() {
            if sequence <= prior.sequence
                || predecessor_revision.as_deref() != Some(&prior.revision)
            {
                return Err(rejected("predecessor or sequence is invalid".into()));
            }
            let before = prior
                .state
                .get("handledTransitions")
                .as_object()
                .map_err(|e| rejected(e.message))?;
            let after = state
                .get("handledTransitions")
                .as_object()
                .map_err(|e| rejected(e.message))?;
            if after.len() != before.len() + 1
                || before
                    .iter()
                    .any(|(id, digest)| after.get(id) != Some(digest))
            {
                return Err(rejected("transition digest chain differs".into()));
            }
        } else {
            if predecessor_revision.is_some() {
                return Err(rejected("root has a predecessor".into()));
            }
            let handled =
                field(state.value(), "handledTransitions").map_err(|e| rejected(e.message))?;
            if handled.as_object().map_err(|e| rejected(e.message))?.len() != 1 {
                return Err(rejected("root must contain one begin transition".into()));
            }
        }
        history.push(HistoryEntry {
            sequence,
            identity_key,
            revision,
            predecessor_revision,
            state_json,
            state,
        });
    }
    let current = match current {
        Some((revision, state_json)) => {
            let rejected = |detail: &str| {
                StoreError::Integrity(format!(
                    "review_episode_current identity_key={key:?}: {detail}"
                ))
            };
            let last = history
                .last()
                .ok_or_else(|| rejected("row has no history"))?;
            if last.revision != revision || last.state_json != state_json {
                return Err(rejected("row differs from final history row"));
            }
            Some(last.state.clone())
        }
        None if !history.is_empty() => {
            return Err(StoreError::Integrity(format!(
                "review_episode_current identity_key={key:?}: orphan history has no current row"
            )));
        }
        None => None,
    };
    Ok(Snapshot { current, history })
}

pub(crate) fn state_json(state: &ReviewEpisodeState) -> String {
    review_episode_core::codec::canonical_json(state.value())
}

pub(crate) fn introduced_transition(
    before: Option<&ReviewEpisodeState>,
    after: &ReviewEpisodeState,
) -> StoreResult<(review_episode_core::codec::JsString, JsValue)> {
    let next = after
        .get("handledTransitions")
        .as_object()
        .map_err(|e| StoreError::Integrity(e.message))?;
    let previous = match before {
        Some(state) => Some(
            state
                .get("handledTransitions")
                .as_object()
                .map_err(|e| StoreError::Integrity(e.message))?,
        ),
        None => None,
    };
    let added: Vec<_> = next
        .iter()
        .filter(|(id, _)| previous.is_none_or(|old| !old.contains_key(*id)))
        .collect();
    if added.len() != 1 {
        return Err(StoreError::Integrity(
            "applied state must introduce one transition".into(),
        ));
    }
    if let Some(old) = previous
        && (next.len() != old.len() + 1
            || old.iter().any(|(id, digest)| next.get(id) != Some(digest)))
    {
        return Err(StoreError::Integrity(
            "applied state rewrote handled transitions".into(),
        ));
    }
    Ok((added[0].0.clone(), added[0].1.clone()))
}
