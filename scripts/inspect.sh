#!/bin/bash
# Database Inspection Script
# Interactive tool for querying and inspecting the database

DB_PATH="${DATABASE_PATH:-persona.db}"

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database file not found at $DB_PATH"
    exit 1
fi

# Function to run SQL queries
run_sql() {
    sqlite3 "$DB_PATH" "$1"
}

# Function to show menu
show_menu() {
    echo ""
    echo "======================================"
    echo "  PERSONA DATABASE INSPECTOR"
    echo "======================================"
    echo ""
    echo "Database: $DB_PATH"
    echo ""
    echo "1.  Show database overview"
    echo "2.  Show table row counts"
    echo "3.  Show recent conversations"
    echo "4.  Show user statistics"
    echo "5.  Show command usage"
    echo "6.  Show error logs"
    echo "7.  Show performance metrics"
    echo "8.  Show active reminders"
    echo "9.  Show custom commands"
    echo "10. Show feature flags"
    echo "11. Run custom SQL query"
    echo "12. Open SQLite shell"
    echo "0.  Exit"
    echo ""
    echo -n "Select option: "
}

# Main loop
while true; do
    show_menu
    read choice

    case $choice in
        1)
            echo ""
            echo "=== DATABASE OVERVIEW ==="
            run_sql "
                SELECT 'Total Users' as metric, COUNT(DISTINCT user_id) as value FROM conversation_history
                UNION ALL
                SELECT 'Total Messages', COUNT(*) FROM conversation_history
                UNION ALL
                SELECT 'Total Commands', COUNT(*) FROM usage_stats
                UNION ALL
                SELECT 'Total Errors', COUNT(*) FROM error_logs
                UNION ALL
                SELECT 'Active Reminders', COUNT(*) FROM reminders WHERE completed = 0
                UNION ALL
                SELECT 'Custom Commands', COUNT(*) FROM custom_commands;
            " | column -t -s '|'
            ;;

        2)
            echo ""
            echo "=== TABLE ROW COUNTS ==="
            echo "Table                         | Rows"
            echo "------------------------------+----------"
            for table in user_preferences usage_stats conversation_history message_metadata \
                         interaction_sessions user_bookmarks reminders custom_commands \
                         daily_analytics performance_metrics error_logs feature_flags \
                         guild_settings extended_user_preferences; do
                count=$(run_sql "SELECT COUNT(*) FROM $table;")
                printf "%-30s| %s\n" "$table" "$count"
            done
            ;;

        3)
            echo ""
            echo "=== RECENT CONVERSATIONS (Last 20 messages) ==="
            run_sql "
                .mode column
                .headers on
                .width 15 10 40 20
                SELECT user_id, role, SUBSTR(content, 1, 40) as message, timestamp
                FROM conversation_history
                ORDER BY timestamp DESC
                LIMIT 20;
            "
            ;;

        4)
            echo ""
            echo "=== USER STATISTICS ==="
            run_sql "
                .mode column
                .headers on
                .width 20 10 10 20
                SELECT user_id, COUNT(*) as messages, default_persona as persona, MAX(timestamp) as last_active
                FROM conversation_history
                LEFT JOIN user_preferences USING(user_id)
                GROUP BY user_id
                ORDER BY messages DESC
                LIMIT 20;
            "
            ;;

        5)
            echo ""
            echo "=== COMMAND USAGE ==="
            run_sql "
                .mode column
                .headers on
                .width 30 10 10
                SELECT command, COUNT(*) as uses, COUNT(DISTINCT user_id) as users
                FROM usage_stats
                GROUP BY command
                ORDER BY uses DESC;
            "
            ;;

        6)
            echo ""
            echo "=== RECENT ERRORS (Last 20) ==="
            run_sql "
                .mode column
                .headers on
                .width 20 30 20
                SELECT error_type, SUBSTR(error_message, 1, 30) as message, timestamp
                FROM error_logs
                ORDER BY timestamp DESC
                LIMIT 20;
            "
            ;;

        7)
            echo ""
            echo "=== PERFORMANCE METRICS ==="
            run_sql "
                .mode column
                .headers on
                .width 20 10 10 10 10
                SELECT metric_type, ROUND(AVG(value), 2) as avg, ROUND(MIN(value), 2) as min, ROUND(MAX(value), 2) as max, unit
                FROM performance_metrics
                GROUP BY metric_type, unit;
            "
            ;;

        8)
            echo ""
            echo "=== ACTIVE REMINDERS ==="
            run_sql "
                .mode column
                .headers on
                .width 15 15 30 20
                SELECT user_id, channel_id, SUBSTR(reminder_text, 1, 30) as reminder, remind_at
                FROM reminders
                WHERE completed = 0
                ORDER BY remind_at;
            "
            ;;

        9)
            echo ""
            echo "=== CUSTOM COMMANDS ==="
            run_sql "
                .mode column
                .headers on
                .width 20 30 15
                SELECT command_name, SUBSTR(response_text, 1, 30) as response, guild_id
                FROM custom_commands
                ORDER BY command_name;
            "
            ;;

        10)
            echo ""
            echo "=== FEATURE FLAGS ==="
            run_sql "
                .mode column
                .headers on
                .width 20 10 15 15
                SELECT feature_name, enabled, user_id, guild_id
                FROM feature_flags
                ORDER BY feature_name;
            "
            ;;

        11)
            echo ""
            echo "Enter SQL query (end with semicolon):"
            read -r query
            echo ""
            run_sql "$query" || echo "Error executing query"
            ;;

        12)
            echo ""
            echo "Opening SQLite shell... (type .exit to return)"
            sqlite3 "$DB_PATH"
            ;;

        0)
            echo "Goodbye!"
            exit 0
            ;;

        *)
            echo "Invalid option"
            ;;
    esac

    echo ""
    echo -n "Press Enter to continue..."
    read
done
