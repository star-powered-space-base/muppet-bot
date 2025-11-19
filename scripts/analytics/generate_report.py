#!/usr/bin/env python3
"""
Generate comprehensive analytics report from the persona database.
Outputs a formatted text report or JSON data.
"""

import sqlite3
import sys
import json
from datetime import datetime, timedelta
from collections import defaultdict

DB_PATH = "persona.db"


def get_connection():
    """Get database connection."""
    return sqlite3.connect(DB_PATH)


def get_overview_stats(conn):
    """Get high-level overview statistics."""
    cursor = conn.cursor()

    stats = {}

    # Total users
    cursor.execute("SELECT COUNT(DISTINCT user_id) FROM conversation_history")
    stats['total_users'] = cursor.fetchone()[0]

    # Total messages
    cursor.execute("SELECT COUNT(*) FROM conversation_history")
    stats['total_messages'] = cursor.fetchone()[0]

    # Total commands
    cursor.execute("SELECT COUNT(*) FROM usage_stats")
    stats['total_commands'] = cursor.fetchone()[0]

    # Active users (last 7 days)
    cursor.execute("""
        SELECT COUNT(DISTINCT user_id)
        FROM conversation_history
        WHERE timestamp >= datetime('now', '-7 days')
    """)
    stats['active_users_7d'] = cursor.fetchone()[0]

    # Total conversations
    cursor.execute("SELECT COUNT(DISTINCT user_id || channel_id) FROM conversation_history")
    stats['total_conversations'] = cursor.fetchone()[0]

    return stats


def get_persona_stats(conn):
    """Get persona usage statistics."""
    cursor = conn.cursor()
    cursor.execute("""
        SELECT persona, COUNT(*) as count
        FROM conversation_history
        WHERE persona != ''
        GROUP BY persona
        ORDER BY count DESC
    """)
    return dict(cursor.fetchall())


def get_command_stats(conn):
    """Get command usage statistics."""
    cursor = conn.cursor()
    cursor.execute("""
        SELECT command, COUNT(*) as count
        FROM usage_stats
        GROUP BY command
        ORDER BY count DESC
        LIMIT 20
    """)
    return dict(cursor.fetchall())


def get_daily_activity(conn, days=30):
    """Get daily activity for the last N days."""
    cursor = conn.cursor()
    cursor.execute("""
        SELECT DATE(timestamp) as date, COUNT(*) as count
        FROM conversation_history
        WHERE timestamp >= datetime('now', '-' || ? || ' days')
        GROUP BY DATE(timestamp)
        ORDER BY date
    """, (days,))
    return dict(cursor.fetchall())


def get_error_stats(conn):
    """Get error statistics."""
    cursor = conn.cursor()

    # Total errors
    cursor.execute("SELECT COUNT(*) FROM error_logs")
    total = cursor.fetchone()[0]

    # Errors by type
    cursor.execute("""
        SELECT error_type, COUNT(*) as count
        FROM error_logs
        GROUP BY error_type
        ORDER BY count DESC
        LIMIT 10
    """)
    by_type = dict(cursor.fetchall())

    # Recent errors (last 24h)
    cursor.execute("""
        SELECT COUNT(*)
        FROM error_logs
        WHERE timestamp >= datetime('now', '-1 day')
    """)
    recent = cursor.fetchone()[0]

    return {
        'total': total,
        'by_type': by_type,
        'last_24h': recent
    }


def get_performance_stats(conn):
    """Get performance metrics statistics."""
    cursor = conn.cursor()
    cursor.execute("""
        SELECT metric_type, AVG(value) as avg, MIN(value) as min, MAX(value) as max, unit
        FROM performance_metrics
        GROUP BY metric_type, unit
    """)

    metrics = {}
    for row in cursor.fetchall():
        metric_type, avg, min_val, max_val, unit = row
        metrics[metric_type] = {
            'avg': round(avg, 2),
            'min': round(min_val, 2),
            'max': round(max_val, 2),
            'unit': unit
        }

    return metrics


def print_text_report(data):
    """Print a formatted text report."""
    print("=" * 70)
    print("PERSONA BOT ANALYTICS REPORT")
    print(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)
    print()

    # Overview
    print("OVERVIEW")
    print("-" * 70)
    overview = data['overview']
    print(f"  Total Users:              {overview['total_users']:,}")
    print(f"  Active Users (7d):        {overview['active_users_7d']:,}")
    print(f"  Total Messages:           {overview['total_messages']:,}")
    print(f"  Total Commands:           {overview['total_commands']:,}")
    print(f"  Total Conversations:      {overview['total_conversations']:,}")
    print()

    # Persona usage
    print("PERSONA USAGE")
    print("-" * 70)
    for persona, count in data['persona_stats'].items():
        pct = (count / overview['total_messages']) * 100
        print(f"  {persona:20s}  {count:8,} ({pct:5.1f}%)")
    print()

    # Top commands
    print("TOP COMMANDS")
    print("-" * 70)
    for i, (command, count) in enumerate(list(data['command_stats'].items())[:10], 1):
        print(f"  {i:2d}. {command:30s}  {count:8,}")
    print()

    # Errors
    print("ERRORS")
    print("-" * 70)
    errors = data['error_stats']
    print(f"  Total Errors:             {errors['total']:,}")
    print(f"  Errors (24h):             {errors['last_24h']:,}")
    if errors['by_type']:
        print("  Top Error Types:")
        for error_type, count in list(errors['by_type'].items())[:5]:
            print(f"    {error_type:25s}  {count:6,}")
    print()

    # Performance
    if data['performance_stats']:
        print("PERFORMANCE METRICS")
        print("-" * 70)
        for metric, stats in data['performance_stats'].items():
            print(f"  {metric}:")
            print(f"    Avg: {stats['avg']} {stats['unit']}")
            print(f"    Min: {stats['min']} {stats['unit']}")
            print(f"    Max: {stats['max']} {stats['unit']}")
        print()

    print("=" * 70)


def main():
    """Main function."""
    output_format = sys.argv[1] if len(sys.argv) > 1 else 'text'

    try:
        conn = get_connection()

        # Gather all statistics
        data = {
            'overview': get_overview_stats(conn),
            'persona_stats': get_persona_stats(conn),
            'command_stats': get_command_stats(conn),
            'daily_activity': get_daily_activity(conn),
            'error_stats': get_error_stats(conn),
            'performance_stats': get_performance_stats(conn),
            'generated_at': datetime.now().isoformat()
        }

        conn.close()

        # Output in requested format
        if output_format == 'json':
            print(json.dumps(data, indent=2))
        else:
            print_text_report(data)

    except sqlite3.Error as e:
        print(f"Database error: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
