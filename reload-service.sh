#!/bin/bash

# Script to reload the systemd service after configuration changes

echo "Stopping current service..."
sudo systemctl stop persona.service

echo "Copying updated service file..."
sudo cp /home/caavere/Projects/bot/persona/persona.service /etc/systemd/system/

echo "Reloading systemd daemon..."
sudo systemctl daemon-reload

echo "Starting service with new configuration..."
sudo systemctl start persona.service

echo ""
echo "Service reloaded! Checking status..."
sudo systemctl status persona.service
