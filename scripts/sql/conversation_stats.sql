-- Conversation Statistics
-- Analyzes conversation patterns and engagement

.mode column
.headers on
.width 20 15 15 20

SELECT 'CONVERSATION OVERVIEW' AS report;
SELECT
    COUNT(DISTINCT user_id) as total_users,
    COUNT(DISTINCT channel_id) as total_channels,
    COUNT(*) as total_messages,
    COUNT(DISTINCT persona) as personas_used
FROM conversation_history;

SELECT '';
SELECT 'PERSONA USAGE' AS report;
SELECT
    persona,
    COUNT(*) as messages,
    COUNT(DISTINCT user_id) as users,
    ROUND(COUNT(*) * 100.0 / (SELECT COUNT(*) FROM conversation_history), 2) as percentage
FROM conversation_history
WHERE persona != ''
GROUP BY persona
ORDER BY messages DESC;

SELECT '';
SELECT 'MOST ACTIVE CONVERSATIONS' AS report;
.width 15 15 15 15 25
SELECT
    user_id,
    channel_id,
    COUNT(*) as messages,
    MIN(timestamp) as started,
    MAX(timestamp) as last_activity
FROM conversation_history
GROUP BY user_id, channel_id
ORDER BY messages DESC
LIMIT 20;

SELECT '';
SELECT 'CONVERSATION LENGTH DISTRIBUTION' AS report;
.width 20 15
SELECT
    CASE
        WHEN msg_count <= 5 THEN '1-5 messages'
        WHEN msg_count <= 10 THEN '6-10 messages'
        WHEN msg_count <= 20 THEN '11-20 messages'
        WHEN msg_count <= 50 THEN '21-50 messages'
        ELSE '50+ messages'
    END as conversation_length,
    COUNT(*) as count
FROM (
    SELECT user_id, channel_id, COUNT(*) as msg_count
    FROM conversation_history
    GROUP BY user_id, channel_id
)
GROUP BY conversation_length
ORDER BY conversation_length;

SELECT '';
SELECT 'HOURLY MESSAGE DISTRIBUTION' AS report;
.width 10 15 20
SELECT
    strftime('%H', timestamp) as hour,
    COUNT(*) as messages,
    ROUND(COUNT(*) * 100.0 / (SELECT COUNT(*) FROM conversation_history), 2) as percentage
FROM conversation_history
GROUP BY strftime('%H', timestamp)
ORDER BY hour;
