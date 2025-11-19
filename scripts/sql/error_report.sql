-- Error Logs Report
-- Shows error statistics and recent errors

.mode column
.headers on
.width 20 15 10 20

SELECT 'ERROR SUMMARY BY TYPE' AS report;
SELECT
    error_type,
    COUNT(*) as occurrences,
    DATE(MIN(timestamp)) as first_seen,
    DATE(MAX(timestamp)) as last_seen
FROM error_logs
GROUP BY error_type
ORDER BY occurrences DESC;

SELECT '';
SELECT 'RECENT ERRORS (Last 50)' AS report;
.width 20 30 15 20
SELECT
    error_type,
    SUBSTR(error_message, 1, 30) as message,
    user_id,
    timestamp
FROM error_logs
ORDER BY timestamp DESC
LIMIT 50;

SELECT '';
SELECT 'ERRORS BY COMMAND' AS report;
.width 20 15 10
SELECT
    command,
    COUNT(*) as error_count,
    COUNT(DISTINCT error_type) as unique_errors
FROM error_logs
WHERE command != ''
GROUP BY command
ORDER BY error_count DESC;

SELECT '';
SELECT 'ERRORS IN LAST 24 HOURS' AS report;
.width 20 15 20
SELECT
    error_type,
    COUNT(*) as count,
    MAX(timestamp) as last_occurrence
FROM error_logs
WHERE timestamp >= datetime('now', '-1 day')
GROUP BY error_type
ORDER BY count DESC;
