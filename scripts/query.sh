#!/bin/bash
# Quick Query Runner
# Runs SQL query files from the sql/ directory

set -e

DB_PATH="${DATABASE_PATH:-persona.db}"
SQL_DIR="$(dirname "$0")/sql"

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database file not found at $DB_PATH"
    exit 1
fi

# If no argument, list available queries
if [ $# -eq 0 ]; then
    echo "Available queries:"
    echo ""
    ls -1 "$SQL_DIR"/*.sql 2>/dev/null | xargs -n1 basename | sed 's/\.sql$//' | nl
    echo ""
    echo "Usage: $0 <query_name>"
    echo "   or: $0 <query_name.sql>"
    echo "   or: $0 /path/to/custom.sql"
    exit 0
fi

# Determine SQL file path
QUERY_ARG="$1"
if [ -f "$QUERY_ARG" ]; then
    # Full path provided
    SQL_FILE="$QUERY_ARG"
elif [ -f "$SQL_DIR/$QUERY_ARG" ]; then
    # File in sql directory
    SQL_FILE="$SQL_DIR/$QUERY_ARG"
elif [ -f "$SQL_DIR/${QUERY_ARG}.sql" ]; then
    # Name without .sql extension
    SQL_FILE="$SQL_DIR/${QUERY_ARG}.sql"
else
    echo "Error: Query file not found: $QUERY_ARG"
    exit 1
fi

# Run the query
echo "Running: $SQL_FILE"
echo "Database: $DB_PATH"
echo ""
sqlite3 "$DB_PATH" < "$SQL_FILE"
