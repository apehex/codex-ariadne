use std::collections::BTreeMap;

use pretty_assertions::assert_eq;
use serde_json::json;

use super::TraceFactIndex;
use crate::EvidenceGrade;
use crate::SessionSummary;
use crate::SessionTrace;
use crate::TraceCapabilities;
use crate::TraceCorrelation;
use crate::TraceFactAvailability;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceObjectRef;
use crate::TraceOrder;
use crate::TraceOrderDomain;
use crate::TraceOwnership;
use crate::TraceRecordPresentation;
use crate::TraceRelation;
use crate::TraceSourceKind;
use crate::TraceStatus;

#[test]
fn rich_sequence_orders_a_thread_even_when_wall_clock_regresses() {
    let second = locator("second");
    let first = locator("first");
    let trace = trace(vec![
        (
            second,
            rich_facts(/*sequence*/ 2, /*wall_clock_start_ms*/ 10),
        ),
        (
            first,
            rich_facts(/*sequence*/ 1, /*wall_clock_start_ms*/ 100),
        ),
    ]);
    let index = TraceFactIndex::new(&trace);

    assert_eq!(
        index
            .thread_positions(&trace, "thread", TraceOrderDomain::Rich)
            .collect::<Vec<_>>(),
        vec![1, 0]
    );
}

#[test]
fn exact_correlation_lookup_returns_all_matching_nodes() {
    let correlation = TraceCorrelation {
        relation: TraceRelation::ModelVisibleCall,
        target: TraceObjectRef::ModelVisibleCall("call".to_string()),
    };
    let mut first_facts = rich_facts(/*sequence*/ 1, /*wall_clock_start_ms*/ 1);
    first_facts.correlations.push(correlation.clone());
    let mut second_facts = rich_facts(/*sequence*/ 2, /*wall_clock_start_ms*/ 2);
    second_facts.correlations.push(correlation.clone());
    let trace = trace(vec![
        (locator("first"), first_facts),
        (locator("second"), second_facts),
    ]);
    let index = TraceFactIndex::new(&trace);

    assert_eq!(
        index
            .correlated_positions(&trace, &correlation)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn merged_sources_share_identity_without_claiming_cross_domain_order() {
    let identity = TraceCorrelation {
        relation: TraceRelation::SourceIdentity,
        target: TraceObjectRef::ConversationItem("item".to_string()),
    };
    let ordinary = TraceNodeFacts::new(
        TraceOrder::ordinary(/*ordinal*/ 8, Some(80)),
        TraceOwnership {
            thread_id: Some("thread".to_string()),
            turn_id: None,
        },
        TraceFactAvailability::Partial,
        [identity.clone()],
    );
    let mut rich = rich_facts(/*sequence*/ 9, /*wall_clock_start_ms*/ 90);
    rich.correlations.push(identity.clone());
    let trace = trace(vec![
        (locator("ordinary"), ordinary.clone()),
        (locator("rich"), rich.clone()),
    ]);
    let index = trace.fact_index();

    assert_eq!(ordinary.source_cmp(&rich), None);
    assert_eq!(
        index
            .correlated_positions(&trace, &identity)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn structural_mutation_requires_rebuilding_the_fact_snapshot() {
    let first = locator("first");
    let mut trace = trace(vec![(
        first,
        rich_facts(/*sequence*/ 1, /*wall_clock_start_ms*/ 1),
    )]);
    let stale = trace.fact_index();
    let second = locator("second");
    trace.nodes.push(node(second.clone()));
    trace.facts.insert(
        second.clone(),
        rich_facts(/*sequence*/ 2, /*wall_clock_start_ms*/ 2),
    );

    assert_eq!(stale.facts(&trace, &second), None);
    assert_eq!(
        trace
            .fact_index()
            .facts(&trace, &second)
            .map(|facts| facts.order),
        Some(TraceOrder::rich(
            /*started_seq*/ 2, /*ended_seq*/ None, /*started_at_unix_ms*/ 2,
            /*ended_at_unix_ms*/ None
        ))
    );
}

fn trace(entries: Vec<(TraceNodeLocator, TraceNodeFacts)>) -> SessionTrace {
    let nodes = entries
        .iter()
        .map(|(locator, _)| node(locator.clone()))
        .collect();
    let facts = entries.into_iter().collect();
    SessionTrace {
        summary: SessionSummary {
            session_id: "session".to_string(),
            root_thread_id: "thread".to_string(),
            source: TraceSourceKind::Rich,
            capabilities: TraceCapabilities::default(),
            created_at: None,
            cwd: None,
            model_provider: None,
            status: TraceStatus::Unknown,
            archived: false,
            thread_count: None,
        },
        nodes,
        facts,
        diagnostics: Vec::new(),
        payloads: BTreeMap::new(),
    }
}

fn node(locator: TraceNodeLocator) -> TraceNode {
    TraceNode {
        locator,
        parent: None,
        provenance: TraceSourceKind::Rich,
        evidence: EvidenceGrade::Semantic,
        timestamp: None,
        label: "node".to_string(),
        presentation: TraceRecordPresentation::default(),
        detail: json!({}),
    }
}

fn locator(id: &str) -> TraceNodeLocator {
    TraceNodeLocator::new("session", TraceNodeKind::ConversationItem, id)
}

fn rich_facts(sequence: u64, wall_clock_start_ms: i64) -> TraceNodeFacts {
    TraceNodeFacts::new(
        TraceOrder::rich(
            sequence,
            /*ended_seq*/ None,
            wall_clock_start_ms,
            /*ended_at_unix_ms*/ None,
        ),
        TraceOwnership {
            thread_id: Some("thread".to_string()),
            turn_id: None,
        },
        TraceFactAvailability::Complete,
        [],
    )
}
