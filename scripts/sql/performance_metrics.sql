-- Performance Metrics Report
-- Shows response times, API latency, and performance trends

.mode column
.headers on
.width 20 10 10 10 15

SELECT 'PERFORMANCE METRICS SUMMARY' AS report;
SELECT
    metric_type,
    COUNT(*) as samples,
    ROUND(AVG(value), 2) as avg_value,
    ROUND(MIN(value), 2) as min_value,
    ROUND(MAX(value), 2) as max_value,
    unit
FROM performance_metrics
GROUP BY metric_type, unit
ORDER BY metric_type;

SELECT '';
SELECT 'LAST 24 HOURS PERFORMANCE' AS report;
SELECT
    metric_type,
    ROUND(AVG(value), 2) as avg_value,
    ROUND(MAX(value), 2) as max_value,
    COUNT(*) as sample_count,
    unit
FROM performance_metrics
WHERE timestamp >= datetime('now', '-1 day')
GROUP BY metric_type, unit
ORDER BY metric_type;

SELECT '';
SELECT 'HOURLY PERFORMANCE TREND (Last 24h)' AS report;
SELECT
    strftime('%Y-%m-%d %H:00', timestamp) as hour,
    metric_type,
    ROUND(AVG(value), 2) as avg_value,
    COUNT(*) as samples
FROM performance_metrics
WHERE timestamp >= datetime('now', '-1 day')
GROUP BY strftime('%Y-%m-%d %H:00', timestamp), metric_type
ORDER BY hour DESC, metric_type
LIMIT 50;
