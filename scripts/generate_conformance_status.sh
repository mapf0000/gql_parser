#!/usr/bin/env bash
set -euo pipefail

MATRIX_FILE="${1:-docs/conformance_matrix.csv}"
OUTPUT_FILE="${2:-docs/conformance_status.md}"

if [[ ! -f "$MATRIX_FILE" ]]; then
  echo "matrix file not found: $MATRIX_FILE" >&2
  exit 1
fi

total_rows=$(( $(wc -l < "$MATRIX_FILE") - 1 ))
closed_rows=$(awk -F, 'NR > 1 && $5 == "closed" { count++ } END { print count + 0 }' "$MATRIX_FILE")
open_rows=$(awk -F, 'NR > 1 && $5 == "open" { count++ } END { print count + 0 }' "$MATRIX_FILE")
closed_no_rows=$(awk -F, 'NR > 1 && $4 == "No" && $5 == "closed" { count++ } END { print count + 0 }' "$MATRIX_FILE")
open_yes_or_partial_rows=$(awk -F, 'NR > 1 && ($4 == "Yes" || $4 == "Partial") && $5 == "open" { count++ } END { print count + 0 }' "$MATRIX_FILE")

{
  echo "# Conformance Status"
  echo
  echo "Generated: $(date -u +%Y-%m-%d)"
  echo
  echo "## Summary"
  echo
  echo "- Total tracked rows: $total_rows"
  echo "- Closed rows: $closed_rows"
  echo "- Open rows: $open_rows"
  echo "- Closed DB/Catalog No rows: $closed_no_rows"
  echo "- Open DB/Catalog Yes or Partial rows: $open_yes_or_partial_rows"
  echo
  echo "## Row Status"
  echo
  echo "| ID | Milestone | Workstream | DB/Catalog | Status | Owner | Tests |"
  echo "| --- | --- | --- | --- | --- | --- | --- |"

  awk -F, 'NR > 1 {
    printf("| %s | %s | %s | %s | %s | %s | `%s` |\n", $1, $2, $3, $4, $5, $6, $11)
  }' "$MATRIX_FILE"
} > "$OUTPUT_FILE"

echo "wrote $OUTPUT_FILE"
