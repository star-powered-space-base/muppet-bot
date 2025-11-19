-- User Activity Statistics
-- Shows top users by message count, command usage, and activity

.mode column
.headers on
.width 20 15 15 15 20

SELECT 'TOP USERS BY MESSAGE COUNT' AS category;
SELECT
    user_id,
    COUNT(*) as total_messages,
    COUNT(CASE WHEN role = 'user' THEN 1 END) as user_messages,
    COUNT(CASE WHEN role = 'assistant' THEN 1 END) as bot_responses,
    MAX(timestamp) as last_activity
FROM conversation_history
GROUP BY user_id
ORDER BY total_messages DESC
LIMIT 10;

SELECT '';
SELECT 'TOP USERS BY COMMAND USAGE' AS category;
SELECT
    user_id,
    COUNT(*) as total_commands,
    COUNT(DISTINCT command) as unique_commands,
    MAX(timestamp) as last_command
FROM usage_stats
GROUP BY user_id
ORDER BY total_commands DESC
LIMIT 10;

SELECT '';
SELECT 'COMMAND POPULARITY' AS category;
SELECT
    command,
    COUNT(*) as usage_count,
    COUNT(DISTINCT user_id) as unique_users,
    DATE(MAX(timestamp)) as last_used
FROM usage_stats
GROUP BY command
ORDER BY usage_count DESC;
