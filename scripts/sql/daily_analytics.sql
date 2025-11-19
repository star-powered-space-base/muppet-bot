-- Daily Analytics Report
-- Shows daily statistics and trends

.mode column
.headers on
.width 12 15 12 15 12 15

SELECT 'LAST 30 DAYS ACTIVITY' AS report;
SELECT
    date,
    total_messages,
    unique_users,
    total_commands,
    total_errors,
    ROUND(CAST(total_errors AS FLOAT) / NULLIF(total_messages, 0) * 100, 2) as error_rate_pct
FROM daily_analytics
ORDER BY date DESC
LIMIT 30;

SELECT '';
SELECT 'WEEKLY SUMMARY' AS report;
SELECT
    strftime('%Y-W%W', date) as week,
    SUM(total_messages) as messages,
    AVG(unique_users) as avg_daily_users,
    SUM(total_commands) as commands,
    SUM(total_errors) as errors
FROM daily_analytics
GROUP BY strftime('%Y-W%W', date)
ORDER BY week DESC
LIMIT 12;

SELECT '';
SELECT 'MONTHLY SUMMARY' AS report;
SELECT
    strftime('%Y-%m', date) as month,
    SUM(total_messages) as total_messages,
    MAX(unique_users) as peak_daily_users,
    SUM(total_commands) as total_commands,
    SUM(total_errors) as total_errors
FROM daily_analytics
GROUP BY strftime('%Y-%m', date)
ORDER BY month DESC
LIMIT 12;
