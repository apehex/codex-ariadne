//! Browser-instance and latest-request identities for background work.

/// Identity of one installed browser instance, including same-session reloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BrowserEpoch(u64);

impl BrowserEpoch {
    /// Creates an epoch from the application-owned monotonic counter.
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }

    /// Fixed epoch used by isolated browser-state tests.
    #[cfg(test)]
    pub(crate) const TEST: Self = Self(1);
}

/// Complete identity of one generation-tagged browser request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RequestToken {
    pub(crate) browser_epoch: BrowserEpoch,
    pub(crate) generation: u64,
}

/// Keeps only the newest queued or pending request for one work family.
pub(crate) struct LatestRequest<K, J> {
    browser_epoch: BrowserEpoch,
    generation: u64,
    pending: Option<(RequestToken, K)>,
    queued: Option<J>,
}

impl<K, J> LatestRequest<K, J> {
    /// Creates an empty controller scoped to one browser instance.
    pub(crate) fn new(browser_epoch: BrowserEpoch) -> Self {
        Self {
            browser_epoch,
            generation: 0,
            pending: None,
            queued: None,
        }
    }

    /// Replaces prior work and queues a job built with the new request token.
    pub(crate) fn submit(&mut self, key: K, build: impl FnOnce(RequestToken) -> J) {
        self.generation = self.generation.wrapping_add(1);
        let token = RequestToken {
            browser_epoch: self.browser_epoch,
            generation: self.generation,
        };
        self.pending = Some((token, key));
        self.queued = Some(build(token));
    }

    /// Returns the key currently awaiting completion.
    pub(crate) fn pending_key(&self) -> Option<&K> {
        self.pending.as_ref().map(|(_, key)| key)
    }

    /// Takes the newest queued job for background dispatch.
    pub(crate) fn take_job(&mut self) -> Option<J> {
        self.queued.take()
    }

    /// Accepts and clears a result only when both token and semantic key match.
    pub(crate) fn accept(&mut self, token: RequestToken, key: &K) -> bool
    where
        K: PartialEq,
    {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.0 != token || pending.1 != *key)
        {
            return false;
        }
        self.pending = None;
        true
    }

    /// Invalidates queued and in-flight work without reusing its generation.
    pub(crate) fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.queued = None;
    }
}

impl<K, J> std::fmt::Debug for LatestRequest<K, J>
where
    K: std::fmt::Debug,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LatestRequest")
            .field("browser_epoch", &self.browser_epoch)
            .field("generation", &self.generation)
            .field("pending", &self.pending)
            .field("queued", &self.queued.is_some())
            .finish()
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
