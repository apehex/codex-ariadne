use pretty_assertions::assert_eq;

use super::BrowserEpoch;
use super::LatestRequest;

#[test]
fn latest_request_accepts_only_the_current_epoch_generation_and_key() {
    let mut requests = LatestRequest::new(BrowserEpoch::new(7));
    requests.submit("old", |token| (token, "old-job"));
    let old = requests.take_job().unwrap();
    requests.submit("current", |token| (token, "current-job"));
    let current = requests.take_job().unwrap();

    assert!(!requests.accept(old.0, &"old"));
    assert!(!requests.accept(current.0, &"other"));
    assert!(requests.accept(current.0, &"current"));
    assert_eq!(requests.pending_key(), None);
}

#[test]
fn invalidation_discards_queued_and_pending_work() {
    let mut requests = LatestRequest::new(BrowserEpoch::new(1));
    requests.submit(1, |token| token);
    requests.invalidate();

    assert_eq!(requests.pending_key(), None);
    assert_eq!(requests.take_job(), None);
}
