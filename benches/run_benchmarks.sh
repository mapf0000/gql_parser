#!/bin/bash
# Benchmark runner script for GQL parser
# Usage: ./run_benchmarks.sh [OPTIONS]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_msg() {
    local color=$1
    local msg=$2
    echo -e "${color}${msg}${NC}"
}

# Print usage
usage() {
    cat << EOF
GQL Parser Benchmark Runner

Usage: $0 [OPTIONS]

OPTIONS:
    -a, --all               Run all benchmarks (default)
    -s, --simple            Run simple query benchmarks only
    -c, --complex           Run complex query benchmarks only
    -t, --stress            Run stress test benchmarks only
    -d, --ddl               Run DDL operation benchmarks only
    -v, --validate          Run parse_and_validate benchmarks only
    -l, --lexer             Run lexer-only benchmarks only
    -p, --pipeline          Run pipeline stage comparison benchmarks
    -q, --quick             Run in quick mode (faster, less accurate)
    --test                  Run benchmarks in test mode (validation only)
    --baseline NAME         Save results as baseline with given name
    --compare NAME          Compare results against baseline with given name
    --open                  Open HTML report after benchmarks complete
    -h, --help              Show this help message

EXAMPLES:
    # Run all benchmarks
    $0

    # Run only simple queries
    $0 --simple

    # Run stress tests in quick mode
    $0 --stress --quick

    # Save baseline for comparison
    $0 --baseline main

    # Compare against baseline
    $0 --compare main

    # Run and open results
    $0 --simple --open
EOF
}

# Parse command line arguments
BENCHMARK_FILTER=""
EXTRA_ARGS=""
OPEN_REPORT=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -a|--all)
            BENCHMARK_FILTER=""
            shift
            ;;
        -s|--simple)
            BENCHMARK_FILTER="simple_queries|where_clauses"
            shift
            ;;
        -c|--complex)
            BENCHMARK_FILTER="complex_queries|aggregation"
            shift
            ;;
        -t|--stress)
            BENCHMARK_FILTER="large_queries|deep_nesting|wide_patterns"
            shift
            ;;
        -d|--ddl)
            BENCHMARK_FILTER="ddl_operations"
            shift
            ;;
        -v|--validate)
            BENCHMARK_FILTER="parse_and_validate"
            shift
            ;;
        -l|--lexer)
            BENCHMARK_FILTER="lexer_only"
            shift
            ;;
        -p|--pipeline)
            BENCHMARK_FILTER="pipeline_stages"
            shift
            ;;
        -q|--quick)
            EXTRA_ARGS="$EXTRA_ARGS --quick"
            shift
            ;;
        --test)
            EXTRA_ARGS="$EXTRA_ARGS --test"
            shift
            ;;
        --baseline)
            if [[ -z "$2" ]]; then
                print_msg "$RED" "Error: --baseline requires a name argument"
                exit 1
            fi
            EXTRA_ARGS="$EXTRA_ARGS --save-baseline $2"
            shift 2
            ;;
        --compare)
            if [[ -z "$2" ]]; then
                print_msg "$RED" "Error: --compare requires a name argument"
                exit 1
            fi
            EXTRA_ARGS="$EXTRA_ARGS --baseline $2"
            shift 2
            ;;
        --open)
            OPEN_REPORT=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            print_msg "$RED" "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Print banner
print_msg "$BLUE" "╔═══════════════════════════════════════════╗"
print_msg "$BLUE" "║   GQL Parser Benchmark Suite             ║"
print_msg "$BLUE" "╚═══════════════════════════════════════════╝"
echo ""

# Build benchmark
print_msg "$YELLOW" "Building benchmarks..."
cargo build --release --bench parser_benchmarks

# Run benchmarks
print_msg "$GREEN" "Running benchmarks..."
if [[ -n "$BENCHMARK_FILTER" ]]; then
    print_msg "$YELLOW" "Filter: $BENCHMARK_FILTER"
    cargo bench --bench parser_benchmarks "$BENCHMARK_FILTER" -- $EXTRA_ARGS
else
    cargo bench --bench parser_benchmarks -- $EXTRA_ARGS
fi

# Open report if requested
if [[ "$OPEN_REPORT" == true ]]; then
    REPORT_FILE="target/criterion/report/index.html"
    if [[ -f "$REPORT_FILE" ]]; then
        print_msg "$GREEN" "Opening benchmark report..."
        if command -v xdg-open &> /dev/null; then
            xdg-open "$REPORT_FILE"
        elif command -v open &> /dev/null; then
            open "$REPORT_FILE"
        else
            print_msg "$YELLOW" "Report available at: $REPORT_FILE"
        fi
    else
        print_msg "$YELLOW" "HTML report not found. Run without --test to generate reports."
    fi
fi

print_msg "$GREEN" "✓ Benchmarks complete!"
echo ""
print_msg "$BLUE" "Results saved to: target/criterion/"
print_msg "$BLUE" "HTML report: target/criterion/report/index.html"
