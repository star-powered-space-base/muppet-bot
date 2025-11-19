# Database Management Scripts

This directory contains utility scripts for managing, querying, and analyzing the persona bot SQLite database.

## Directory Structure

```
scripts/
├── backup.sh                 # Database backup utility
├── export_data.sh            # Export data to CSV/JSON/SQL
├── inspect.sh                # Interactive database inspector
├── query.sh                  # SQL query runner
├── sql/                      # Pre-written SQL queries
│   ├── user_stats.sql        # User activity statistics
│   ├── daily_analytics.sql   # Daily/weekly/monthly analytics
│   ├── performance_metrics.sql # Performance metrics report
│   ├── error_report.sql      # Error logs analysis
│   ├── conversation_stats.sql # Conversation patterns
│   └── db_info.sql           # Database schema info
├── analytics/                # Analytics scripts
│   └── generate_report.py   # Comprehensive analytics report
└── maintenance/              # Maintenance utilities
    └── cleanup.sh            # Data cleanup and vacuum
```

## Core Scripts

### 🔄 backup.sh

Create timestamped backups of the database.

**Usage:**
```bash
./scripts/backup.sh
```

**Environment Variables:**
- `DATABASE_PATH` - Path to database file (default: `persona.db`)
- `BACKUP_DIR` - Backup directory (default: `backups/`)
- `MAX_BACKUPS` - Number of backups to keep (default: `10`)

**Features:**
- Creates timestamped backup files
- Automatically rotates old backups
- Compresses backups older than 7 days
- Verifies backup integrity

**Example:**
```bash
# Create backup with custom settings
DATABASE_PATH=/path/to/persona.db BACKUP_DIR=/backups MAX_BACKUPS=20 ./scripts/backup.sh
```

### 📊 export_data.sh

Export database tables to various formats.

**Usage:**
```bash
./scripts/export_data.sh [csv|json|sql]
```

**Formats:**
- `csv` - Export all tables to CSV files (default)
- `json` - Export all tables to JSON files
- `sql` - Full SQL dump of entire database

**Environment Variables:**
- `DATABASE_PATH` - Path to database file (default: `persona.db`)
- `EXPORT_DIR` - Export directory (default: `exports/`)

**Examples:**
```bash
# Export to CSV (creates archive)
./scripts/export_data.sh csv

# Export to JSON
./scripts/export_data.sh json

# Full SQL dump
./scripts/export_data.sh sql

# Custom export directory
EXPORT_DIR=/tmp/exports ./scripts/export_data.sh csv
```

### 🔍 inspect.sh

Interactive database inspection tool with a menu-driven interface.

**Usage:**
```bash
./scripts/inspect.sh
```

**Features:**
- Database overview and statistics
- Recent conversations viewer
- User activity analysis
- Command usage statistics
- Error logs viewer
- Performance metrics
- Active reminders list
- Custom SQL query execution
- Direct SQLite shell access

**Example:**
```bash
# Launch interactive inspector
./scripts/inspect.sh
```

### 📝 query.sh

Quick SQL query runner for pre-written queries.

**Usage:**
```bash
./scripts/query.sh [query_name]
```

**Examples:**
```bash
# List available queries
./scripts/query.sh

# Run a specific query
./scripts/query.sh user_stats
./scripts/query.sh daily_analytics

# Run custom query file
./scripts/query.sh /path/to/custom.sql
```

## SQL Queries

Pre-written SQL queries in the `sql/` directory:

### user_stats.sql
Analyzes user activity patterns:
- Top users by message count
- Command usage per user
- Command popularity rankings

**Run:**
```bash
./scripts/query.sh user_stats
```

### daily_analytics.sql
Shows analytics trends over time:
- Last 30 days activity
- Weekly summaries
- Monthly aggregates
- Error rates

**Run:**
```bash
./scripts/query.sh daily_analytics
```

### performance_metrics.sql
Performance analysis:
- Average response times
- API latency metrics
- Performance trends
- Hourly breakdowns

**Run:**
```bash
./scripts/query.sh performance_metrics
```

### error_report.sql
Error analysis:
- Errors by type
- Recent error logs
- Errors by command
- 24-hour error summary

**Run:**
```bash
./scripts/query.sh error_report
```

### conversation_stats.sql
Conversation patterns:
- Total conversations
- Persona usage breakdown
- Active conversation lists
- Message distribution by hour
- Conversation length analysis

**Run:**
```bash
./scripts/query.sh conversation_stats
```

### db_info.sql
Database health and schema:
- Table list
- Row counts per table
- Index information
- Data age ranges

**Run:**
```bash
./scripts/query.sh db_info
```

## Analytics

### generate_report.py

Python script for comprehensive analytics reports.

**Requirements:**
- Python 3.6+
- No external dependencies (uses sqlite3 from stdlib)

**Usage:**
```bash
# Text report (default)
./scripts/analytics/generate_report.py

# JSON output
./scripts/analytics/generate_report.py json

# Save to file
./scripts/analytics/generate_report.py > report.txt
./scripts/analytics/generate_report.py json > report.json
```

**Report Includes:**
- Overview statistics
- Persona usage breakdown
- Top commands
- Error statistics
- Performance metrics
- Daily activity trends

## Maintenance

### cleanup.sh

Database cleanup and optimization script.

**Usage:**
```bash
./scripts/maintenance/cleanup.sh
```

**What it does:**
- Removes old conversation history (default: 90 days)
- Cleans old usage stats (default: 180 days)
- Purges old performance metrics (default: 30 days)
- Removes old error logs (default: 60 days)
- Cleans completed reminders (default: 30 days)
- Removes old message metadata (default: 90 days)
- Cleans ended sessions (default: 30 days)
- Vacuums database to reclaim space
- Analyzes tables for query optimization

**Environment Variables:**
- `DATABASE_PATH` - Path to database file
- `CONVERSATION_RETENTION_DAYS` - Days to keep conversations (default: 90)
- `USAGE_RETENTION_DAYS` - Days to keep usage stats (default: 180)
- `METRICS_RETENTION_DAYS` - Days to keep metrics (default: 30)
- `ERROR_RETENTION_DAYS` - Days to keep errors (default: 60)
- `REMINDER_RETENTION_DAYS` - Days to keep completed reminders (default: 30)
- `METADATA_RETENTION_DAYS` - Days to keep metadata (default: 90)
- `SESSION_RETENTION_DAYS` - Days to keep sessions (default: 30)

**Example:**
```bash
# Custom retention periods
CONVERSATION_RETENTION_DAYS=30 ERROR_RETENTION_DAYS=90 ./scripts/maintenance/cleanup.sh
```

## Quick Reference

### Common Tasks

**Daily backup:**
```bash
./scripts/backup.sh
```

**Weekly cleanup:**
```bash
./scripts/maintenance/cleanup.sh
```

**Generate analytics report:**
```bash
./scripts/analytics/generate_report.py > weekly_report.txt
```

**Check database health:**
```bash
./scripts/query.sh db_info
```

**View recent errors:**
```bash
./scripts/query.sh error_report
```

**Interactive inspection:**
```bash
./scripts/inspect.sh
```

### Automation with Cron

Add to crontab for automated maintenance:

```cron
# Daily backup at 2 AM
0 2 * * * cd /path/to/bot/persona && ./scripts/backup.sh

# Weekly cleanup on Sunday at 3 AM
0 3 * * 0 cd /path/to/bot/persona && ./scripts/maintenance/cleanup.sh

# Weekly report on Monday at 9 AM
0 9 * * 1 cd /path/to/bot/persona && ./scripts/analytics/generate_report.py > reports/weekly_$(date +\%Y\%m\%d).txt
```

## Tips and Best Practices

1. **Always backup before cleanup:**
   ```bash
   ./scripts/backup.sh && ./scripts/maintenance/cleanup.sh
   ```

2. **Monitor database size:**
   ```bash
   du -h persona.db
   ```

3. **Check recent activity:**
   ```bash
   ./scripts/inspect.sh  # Option 3: Recent conversations
   ```

4. **Export before major changes:**
   ```bash
   ./scripts/export_data.sh sql
   ```

5. **Regular analytics reviews:**
   ```bash
   ./scripts/query.sh daily_analytics
   ./scripts/query.sh user_stats
   ```

## Troubleshooting

**Database locked error:**
- Stop the bot before running maintenance scripts
- Or use `PRAGMA busy_timeout = 5000;` in queries

**Permission denied:**
- Ensure scripts are executable: `chmod +x scripts/*.sh`
- Check database file permissions

**Query timeout:**
- Database may be large; consider adding indexes
- Run `ANALYZE;` to update query planner statistics

**Backup failures:**
- Ensure sufficient disk space
- Check write permissions on backup directory

## Environment Variables

All scripts respect these environment variables:

- `DATABASE_PATH` - Database file location (default: `persona.db`)
- `BACKUP_DIR` - Backup storage location (default: `backups/`)
- `EXPORT_DIR` - Export storage location (default: `exports/`)

Set globally in your shell:
```bash
export DATABASE_PATH=/custom/path/persona.db
export BACKUP_DIR=/mnt/backups
```

Or per-command:
```bash
DATABASE_PATH=/custom/path.db ./scripts/backup.sh
```
