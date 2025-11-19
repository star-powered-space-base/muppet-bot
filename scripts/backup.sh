#!/bin/bash
# Database Backup Script
# Creates timestamped backups of the persona.db database

set -e

DB_PATH="${DATABASE_PATH:-persona.db}"
BACKUP_DIR="${BACKUP_DIR:-backups}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/persona_$TIMESTAMP.db"

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database file not found at $DB_PATH"
    exit 1
fi

# Create backup
echo "Creating backup of $DB_PATH..."
cp "$DB_PATH" "$BACKUP_FILE"

# Verify backup
if [ -f "$BACKUP_FILE" ]; then
    SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
    echo "✓ Backup created successfully: $BACKUP_FILE ($SIZE)"
else
    echo "✗ Backup failed"
    exit 1
fi

# Optional: Keep only last N backups
MAX_BACKUPS=${MAX_BACKUPS:-10}
BACKUP_COUNT=$(ls -1 "$BACKUP_DIR"/persona_*.db 2>/dev/null | wc -l)

if [ "$BACKUP_COUNT" -gt "$MAX_BACKUPS" ]; then
    echo "Cleaning up old backups (keeping last $MAX_BACKUPS)..."
    ls -1t "$BACKUP_DIR"/persona_*.db | tail -n +$((MAX_BACKUPS + 1)) | xargs rm -f
    echo "✓ Old backups cleaned up"
fi

# Optional: Compress old backups (older than 7 days)
find "$BACKUP_DIR" -name "persona_*.db" -mtime +7 -not -name "*.gz" -exec gzip {} \; 2>/dev/null || true

echo "Done!"
