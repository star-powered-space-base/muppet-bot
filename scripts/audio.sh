#!/bin/bash

# Check if argument 1 is provided
if [ -z "$1" ]; then
    echo "Error: Provide an audio file"
    echo "Usage: audio.sh <path-to-audio-file>" 
    exit 1
fi

curl https://api.openai.com/v1/audio/transcriptions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: multipart/form-data" \
  -F file="@$1" \
  -F model="whisper-1"

