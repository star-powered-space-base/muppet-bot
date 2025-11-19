#!/bin/bash
# Database Cleanup Script
# Removes old data based on retention policies

set -e

DB_PATH="${DATABASE_PATH:-persona.db}"

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database file not found at $DB_PATH"
    exit 1
fi

echo "Starting database cleanup..."
echo "Database: $DB_PATH"
echo ""

# Cleanup old conversation history (default: 90 days)
CONVERSATION_RETENTION_DAYS=${CONVERSATION_RETENTION_DAYS:-90}
echo "Cleaning conversation history older than $CONVERSATION_RETENTION_DAYS days..."
sqlite3 "$DB_PATH" "DELETE FROM conversation_history WHERE timestamp < datetime('now', '-$CONVERSATION_RETENTION_DAYS days');"
DELETED=$(sqlite3 "$DB_PATH" "SELECT changes();")
echo "  Deleted $DELETED conversation messages"

# Cleanup old usage stats (default: 180 days)
USAGE_RETENTION_DAYS=${USAGE_RETENTION_DAYS:-180}
echo "Cleaning usage stats older than $USAGE_RETENTION_DAYS days..."
sqlite3 "$DB_PATH" "DELETE FROM usage_stats WHERE timestamp < datetime('now', '-$USAGE_RETENTION_DAYS days');"
DELETED=$(sqlite3 "$DB_PATH" "SELECT changes();")
echo "  Deleted $DELETED usage records"

# Cleanup old performance metrics (default: 30 days)
METRICS_RETENTION_DAYS=${METRICS_RETENTION_DAYS:-30}
echo "Cleaning performance metrics older than $METRICS_RETENTION_DAYS days..."
sqlite3 "$DB_PATH" "DELETE FROM performance_metrics WHERE timestamp < datetime('now', '-$METRICS_RETENTION_DAYS days');"
DELETED=$(sqlite3 "$DB_PATH" "SELECT changes();")
echo "  Deleted $DELETED metric records"

# Cleanup old error logs (default: 60 days)
ERROR_RETENTION_DAYS=${ERROR_RETENTION_DAYS:-60}
echo "Cleaning error logs older than $ERROR_RETENTION_DAYS days..."
sqlite3 "$DB_PATH" "DELETE FROM error_logs WHERE timestamp < datetime('now', '-$ERROR_RETENTION_DAYS days');"
DELETED=$(sqlite3 "$DB_PATH" "SELECT changes();")
echo "  Deleted $DELETED error records"

# Cleanup completed reminders (default: 30 days)
REMINDER_RETENTION_DAYS=${REMINDER_RETENTION_DAYS:-30}
echo "Cleaning completed reminders older than $REMINDER_RETENTION_DAYS days..."
sqlite3 "$DB_PATH" "DELETE FROM reminders WHERE completed = 1 AND completed_at < datetime('now', '-$REMINDER_RETENTION_DAYS days');"
DELETED=$(sqlite3 "$DB_PATH" "SELECT changes();")
echo "  Deleted $DELETED completed reminders"

# Cleanup old message metadata (default: 90 days)
METADATA_RETENTION_DAYS=${METADATA_RETENTION_DAYS:-90}
echo "Cleaning message metadata older than $METADATA_RETENTION_DAYS days..."
sqlite3 "$DB_PATH" "DELETE FROM message_metadata WHERE created_at < datetime('now', '-$METADATA_RETENTION_DAYS days');"
DELETED=$(sqlite3 "$DB_PATH" "SELECT changes();")
echo "  Deleted $DELETED metadata records"

# Cleanup ended sessions (default: 30 days)
SESSION_RETENTION_DAYS=${SESSION_RETENTION_DAYS:-30}
echo "Cleaning old interaction sessions older than $SESSION_RETENTION_DAYS days..."
sqlite3 "$DB_PATH" "DELETE FROM interaction_sessions WHERE session_end IS NOT NULL AND session_end < datetime('now', '-$SESSION_RETENTION_DAYS days');"
DELETED=$(sqlite3 "$DB_PATH" "SELECT changes();")
echo "  Deleted $DELETED session records"

# Vacuum database to reclaim space
echo ""
echo "Vacuuming database to reclaim space..."
SIZE_BEFORE=$(du -h "$DB_PATH" | cut -f1)
sqlite3 "$DB_PATH" "VACUUM;"
SIZE_AFTER=$(du -h "$DB_PATH" | cut -f1)
echo "  Database size: $SIZE_BEFORE -> $SIZE_AFTER"

# Analyze for query optimization
echo "Analyzing database for query optimization..."
sqlite3 "$DB_PATH" "ANALYZE;"

echo ""
echo "✓ Cleanup completed successfully!"
