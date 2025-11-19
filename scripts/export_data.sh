#!/bin/bash
# Data Export Script
# Exports database tables to CSV, JSON, or SQL format

set -e

DB_PATH="${DATABASE_PATH:-persona.db}"
EXPORT_DIR="${EXPORT_DIR:-exports}"
FORMAT="${1:-csv}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database file not found at $DB_PATH"
    exit 1
fi

# Create export directory
mkdir -p "$EXPORT_DIR"

echo "Exporting database to $FORMAT format..."
echo "Database: $DB_PATH"
echo "Export directory: $EXPORT_DIR"
echo ""

# Tables to export
TABLES=(
    "user_preferences"
    "usage_stats"
    "conversation_history"
    "message_metadata"
    "interaction_sessions"
    "user_bookmarks"
    "reminders"
    "custom_commands"
    "daily_analytics"
    "performance_metrics"
    "error_logs"
    "feature_flags"
    "guild_settings"
    "extended_user_preferences"
)

case $FORMAT in
    csv)
        for table in "${TABLES[@]}"; do
            output_file="$EXPORT_DIR/${table}_${TIMESTAMP}.csv"
            echo "Exporting $table to $output_file..."
            sqlite3 -header -csv "$DB_PATH" "SELECT * FROM $table;" > "$output_file"
        done
        ;;

    json)
        for table in "${TABLES[@]}"; do
            output_file="$EXPORT_DIR/${table}_${TIMESTAMP}.json"
            echo "Exporting $table to $output_file..."
            sqlite3 "$DB_PATH" "SELECT json_group_array(json_object(
                $(sqlite3 "$DB_PATH" "PRAGMA table_info($table);" | awk -F'|' '{printf "\047%s\047, %s,", $2, $2}' | sed 's/,$//')
            )) FROM $table;" > "$output_file"
        done
        ;;

    sql)
        output_file="$EXPORT_DIR/full_backup_${TIMESTAMP}.sql"
        echo "Exporting full database to $output_file..."
        sqlite3 "$DB_PATH" .dump > "$output_file"
        ;;

    *)
        echo "Error: Unknown format '$FORMAT'"
        echo "Usage: $0 [csv|json|sql]"
        exit 1
        ;;
esac

# Create archive
if [ "$FORMAT" != "sql" ]; then
    echo ""
    echo "Creating archive..."
    tar -czf "$EXPORT_DIR/export_${TIMESTAMP}.tar.gz" -C "$EXPORT_DIR" *_${TIMESTAMP}.*

    # Clean up individual files
    rm -f "$EXPORT_DIR"/*_${TIMESTAMP}.{csv,json}

    echo "✓ Archive created: $EXPORT_DIR/export_${TIMESTAMP}.tar.gz"
else
    echo "✓ SQL dump created: $output_file"
fi

echo ""
echo "✓ Export completed successfully!"
