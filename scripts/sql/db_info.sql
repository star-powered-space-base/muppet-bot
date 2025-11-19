-- Database Information and Schema
-- Shows table sizes, schema, and database health

.mode column
.headers on
.width 30 15 15

SELECT 'DATABASE TABLES' AS report;
SELECT
    name as table_name,
    type
FROM sqlite_master
WHERE type IN ('table', 'view')
ORDER BY name;

SELECT '';
SELECT 'TABLE ROW COUNTS' AS report;
SELECT 'user_preferences' as table_name, COUNT(*) as rows FROM user_preferences
UNION ALL
SELECT 'usage_stats', COUNT(*) FROM usage_stats
UNION ALL
SELECT 'conversation_history', COUNT(*) FROM conversation_history
UNION ALL
SELECT 'message_metadata', COUNT(*) FROM message_metadata
UNION ALL
SELECT 'interaction_sessions', COUNT(*) FROM interaction_sessions
UNION ALL
SELECT 'user_bookmarks', COUNT(*) FROM user_bookmarks
UNION ALL
SELECT 'reminders', COUNT(*) FROM reminders
UNION ALL
SELECT 'custom_commands', COUNT(*) FROM custom_commands
UNION ALL
SELECT 'daily_analytics', COUNT(*) FROM daily_analytics
UNION ALL
SELECT 'performance_metrics', COUNT(*) FROM performance_metrics
UNION ALL
SELECT 'error_logs', COUNT(*) FROM error_logs
UNION ALL
SELECT 'feature_flags', COUNT(*) FROM feature_flags
UNION ALL
SELECT 'guild_settings', COUNT(*) FROM guild_settings
UNION ALL
SELECT 'extended_user_preferences', COUNT(*) FROM extended_user_preferences
ORDER BY rows DESC;

SELECT '';
SELECT 'DATABASE INDEXES' AS report;
SELECT
    name as index_name,
    tbl_name as table_name
FROM sqlite_master
WHERE type = 'index'
AND name NOT LIKE 'sqlite_%'
ORDER BY tbl_name, name;

SELECT '';
SELECT 'DATA AGE RANGES' AS report;
.width 25 25 25
SELECT
    'conversation_history' as table_name,
    MIN(timestamp) as oldest_record,
    MAX(timestamp) as newest_record
FROM conversation_history
UNION ALL
SELECT
    'usage_stats',
    MIN(timestamp),
    MAX(timestamp)
FROM usage_stats
UNION ALL
SELECT
    'performance_metrics',
    MIN(timestamp),
    MAX(timestamp)
FROM performance_metrics
UNION ALL
SELECT
    'error_logs',
    MIN(timestamp),
    MAX(timestamp)
FROM error_logs;
