//! Ordinary-rollout parent topology with conflict and cycle rejection.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::catalog::OrdinaryThread;

/// Precomputed ordinary-thread identity and containment relationships.
pub(crate) struct OrdinaryTopology<'a> {
    by_id: BTreeMap<&'a str, Vec<&'a OrdinaryThread>>,
}

impl<'a> OrdinaryTopology<'a> {
    /// Indexes every retained observation without silently choosing a conflicting parent.
    pub(crate) fn new(threads: &'a [OrdinaryThread]) -> Self {
        let mut by_id = BTreeMap::<&str, Vec<&OrdinaryThread>>::new();
        for thread in threads {
            by_id
                .entry(thread.thread_id.as_str())
                .or_default()
                .push(thread);
        }
        Self { by_id }
    }

    /// Resolves a same-session parent while rejecting gaps, conflicts, and cycles.
    pub(crate) fn parent_id(&self, thread: &'a OrdinaryThread) -> Result<Option<&'a str>, String> {
        let Some(parent_id) = thread.parent_thread_id.as_deref() else {
            return Ok(None);
        };
        let mut next = Some(parent_id);
        let mut seen = BTreeSet::new();
        while let Some(id) = next {
            if !seen.insert(id) {
                return Err(format!(
                    "parent cycle at {id}; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            }
            let Some(observations) = self.by_id.get(id) else {
                return Err(format!(
                    "parent rollout {id} is missing or belongs to another session; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            };
            let same_session = observations
                .iter()
                .copied()
                .filter(|parent| parent.session_id == thread.session_id)
                .collect::<Vec<_>>();
            if same_session.is_empty() {
                let observed_session = &observations[0].session_id;
                return Err(format!(
                    "parent rollout {id} belongs to session {observed_session}; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            }
            let parent_parent = same_session[0].parent_thread_id.as_deref();
            if same_session
                .iter()
                .any(|parent| parent.parent_thread_id.as_deref() != parent_parent)
            {
                return Err(format!(
                    "parent rollout {id} has conflicting containment observations; attached thread {} to session {}",
                    thread.thread_id, thread.session_id
                ));
            }
            next = parent_parent;
        }
        Ok(Some(parent_id))
    }
}
