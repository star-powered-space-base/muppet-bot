#!/bin/bash
# Database Restore Script
# Restores database from a backup file

set -e

DB_PATH="${DATABASE_PATH:-persona.db}"
BACKUP_DIR="${BACKUP_DIR:-backups}"

# Function to list available backups
list_backups() {
    echo "Available backups:"
    echo ""
    ls -lht "$BACKUP_DIR"/persona_*.db* 2>/dev/null | awk '{print NR". "$9" ("$5", "$6" "$7" "$8")"}' || echo "No backups found"
    echo ""
}

# Show usage
if [ $# -eq 0 ]; then
    echo "Usage: $0 <backup_file>"
    echo "   or: $0 --list"
    echo "   or: $0 --latest"
    echo ""
    list_backups
    exit 0
fi

# Handle --list flag
if [ "$1" = "--list" ]; then
    list_backups
    exit 0
fi

# Handle --latest flag
if [ "$1" = "--latest" ]; then
    BACKUP_FILE=$(ls -t "$BACKUP_DIR"/persona_*.db 2>/dev/null | head -1)
    if [ -z "$BACKUP_FILE" ]; then
        echo "Error: No backups found in $BACKUP_DIR"
        exit 1
    fi
    echo "Using latest backup: $BACKUP_FILE"
else
    BACKUP_FILE="$1"
fi

# Check if backup file exists
if [ ! -f "$BACKUP_FILE" ]; then
    # Try in backup directory
    if [ -f "$BACKUP_DIR/$BACKUP_FILE" ]; then
        BACKUP_FILE="$BACKUP_DIR/$BACKUP_FILE"
    else
        echo "Error: Backup file not found: $BACKUP_FILE"
        exit 1
    fi
fi

# Handle gzipped backups
if [[ "$BACKUP_FILE" == *.gz ]]; then
    echo "Backup is compressed, decompressing..."
    TEMP_FILE=$(mktemp)
    gunzip -c "$BACKUP_FILE" > "$TEMP_FILE"
    BACKUP_FILE="$TEMP_FILE"
    CLEANUP_TEMP=1
fi

# Verify it's a valid SQLite database
if ! sqlite3 "$BACKUP_FILE" "SELECT 1;" &>/dev/null; then
    echo "Error: Invalid SQLite database file"
    [ -n "$CLEANUP_TEMP" ] && rm -f "$TEMP_FILE"
    exit 1
fi

# Show backup info
echo ""
echo "Restore Information:"
echo "  From: $BACKUP_FILE"
echo "  To:   $DB_PATH"
echo ""
echo "Backup details:"
sqlite3 "$BACKUP_FILE" "
    SELECT 'Tables: ' || COUNT(*) FROM sqlite_master WHERE type='table';
    SELECT 'Messages: ' || COUNT(*) FROM conversation_history;
    SELECT 'Users: ' || COUNT(DISTINCT user_id) FROM conversation_history;
" 2>/dev/null || echo "  Unable to read backup details"
echo ""

# Confirmation
if [ -f "$DB_PATH" ]; then
    echo "WARNING: This will overwrite the existing database at $DB_PATH"
    echo -n "Are you sure? (yes/no): "
    read confirm
    if [ "$confirm" != "yes" ]; then
        echo "Restore cancelled"
        [ -n "$CLEANUP_TEMP" ] && rm -f "$TEMP_FILE"
        exit 0
    fi

    # Create safety backup
    SAFETY_BACKUP="${DB_PATH}.before-restore.$(date +%Y%m%d_%H%M%S)"
    echo "Creating safety backup: $SAFETY_BACKUP"
    cp "$DB_PATH" "$SAFETY_BACKUP"
fi

# Perform restore
echo "Restoring database..."
cp "$BACKUP_FILE" "$DB_PATH"

# Verify restore
if sqlite3 "$DB_PATH" "SELECT 1;" &>/dev/null; then
    SIZE=$(du -h "$DB_PATH" | cut -f1)
    echo "✓ Database restored successfully: $DB_PATH ($SIZE)"

    # Optimize database
    echo "Optimizing database..."
    sqlite3 "$DB_PATH" "VACUUM; ANALYZE;"
    echo "✓ Database optimized"
else
    echo "✗ Restore failed - database may be corrupted"
    if [ -f "$SAFETY_BACKUP" ]; then
        echo "Restoring from safety backup..."
        mv "$SAFETY_BACKUP" "$DB_PATH"
        echo "✓ Reverted to previous database"
    fi
    [ -n "$CLEANUP_TEMP" ] && rm -f "$TEMP_FILE"
    exit 1
fi

# Cleanup
[ -n "$CLEANUP_TEMP" ] && rm -f "$TEMP_FILE"

echo ""
echo "Done! Database has been restored."
if [ -f "$SAFETY_BACKUP" ]; then
    echo "Safety backup available at: $SAFETY_BACKUP"
fi
