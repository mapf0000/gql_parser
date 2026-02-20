use std::collections::HashSet;
use std::fs;

const MATRIX_FILE: &str = "docs/conformance_matrix.csv";
const STATUS_FILE: &str = "docs/conformance_status.md";

fn load_matrix_rows() -> Vec<Vec<String>> {
    let content = fs::read_to_string(MATRIX_FILE).expect("failed to read conformance matrix file");
    let mut rows = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        if line_idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields = line
            .split(',')
            .map(|field| field.trim().to_string())
            .collect::<Vec<_>>();
        rows.push(fields);
    }

    rows
}

#[test]
fn conformance_matrix_rows_are_well_formed() {
    let rows = load_matrix_rows();
    assert!(!rows.is_empty(), "expected non-empty conformance matrix");

    let mut ids = HashSet::new();
    for row in &rows {
        assert_eq!(
            row.len(),
            12,
            "expected 12 columns per row in conformance matrix"
        );
        assert!(!row[0].is_empty(), "row id must not be empty");
        assert!(ids.insert(row[0].clone()), "duplicate row id: {}", row[0]);
        assert!(
            row[4] == "open" || row[4] == "closed",
            "status must be open or closed"
        );
    }
}

#[test]
fn every_matrix_row_has_owner_and_traceability() {
    let rows = load_matrix_rows();
    for row in &rows {
        assert!(
            !row[5].is_empty(),
            "owner must not be empty for row {}",
            row[0]
        );
        assert!(
            !row[10].is_empty(),
            "tests column must not be empty for row {}",
            row[0]
        );
    }
}

#[test]
fn generated_status_lists_every_row() {
    let rows = load_matrix_rows();
    let status = fs::read_to_string(STATUS_FILE).expect("failed to read generated status file");

    for row in &rows {
        let id = &row[0];
        assert!(
            status.contains(&format!("| {id} |")),
            "status report does not include matrix row {id}"
        );
    }
}
