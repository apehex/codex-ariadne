use pretty_assertions::assert_eq;

use super::ColumnPlan;
use crate::PreviewMode;
use crate::TraceColumn;

#[test]
fn column_plan_drops_lower_priority_columns_as_width_contracts() {
    let columns = vec![
        TraceColumn::Kind,
        TraceColumn::Class,
        TraceColumn::Timestamp,
        TraceColumn::Identifier,
    ];

    assert_eq!(
        ColumnPlan::new(&columns, PreviewMode::Never, 100).omitted(),
        0
    );
    assert_eq!(
        ColumnPlan::new(&columns, PreviewMode::Never, 45).omitted(),
        2
    );
}

#[test]
fn column_plan_forces_header_and_preview_onto_one_line() {
    let plan = ColumnPlan::new(&[TraceColumn::Kind], PreviewMode::Always, 80);
    let row = plan.row(
        /*selected*/ false,
        "name\ncontinued",
        /*node*/ None,
        Some("preview\tcontinued"),
    );

    assert!(!row.to_string().contains('\n'));
    assert!(!row.to_string().contains('\t'));
}
