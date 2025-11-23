#!/bin/bash

# Test script to verify OpenAI timeout functionality
# This script will run the bot with a test command to verify timeout handling

echo "Testing OpenAI timeout functionality..."

# Test the Gateway bot
echo "Testing Gateway bot..."
timeout 60s cargo run --bin bot &
BOT_PID=$!

# Wait a moment for the bot to start
sleep 5

# You can test by sending a message to the bot in Discord with a command like:
# /hey tell me a very long story about everything you know

echo "Bot started with PID: $BOT_PID"
echo "Test the bot in Discord with commands like:"
echo "- /hey tell me a very long story about everything"
echo "- /explain quantum physics in detail"
echo ""
echo "The bot should now timeout after 45 seconds if OpenAI doesn't respond"
echo "Press Ctrl+C to stop the test"

# Wait for the bot process or user interrupt
wait $BOT_PID