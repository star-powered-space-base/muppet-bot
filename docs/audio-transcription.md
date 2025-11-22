# Audio Transcription

This document describes the audio transcription feature for the Persona Discord Bot.

## Overview

The bot can automatically transcribe audio and video file attachments using OpenAI's Whisper API. When users send audio files in Discord, the bot downloads the file, sends it to OpenAI for transcription, and returns the text content.

## Requirements

- **OpenAI API Key** - Must be set via `OPENAI_API_KEY` environment variable
- **curl** - System command used for downloading files and API calls
- **Write access to `/tmp/`** - Temporary storage for downloaded files

## Supported Formats

The following file extensions are supported:

| Audio | Video |
|-------|-------|
| `.mp3` | `.mp4` |
| `.wav` | `.mov` |
| `.m4a` | `.avi` |
| `.flac` | |
| `.ogg` | |
| `.aac` | |
| `.wma` | |

## How It Works

### Flow Diagram

```
User uploads audio file
         ↓
Bot checks guild audio_transcription setting
         ↓ (if enabled)
Bot sends "🎵 Transcribing your audio..."
         ↓
Download attachment to /tmp/discord_audio_{filename}
         ↓
POST to OpenAI Whisper API (whisper-1 model)
         ↓
Parse transcription from JSON response
         ↓
Delete temp file
         ↓
Send "📝 **Transcription:** {text}" to channel
         ↓ (optional)
If user included text with attachment,
generate AI response using transcription as context
```

### Technical Implementation

The feature is implemented in `src/audio.rs`:

```rust
pub struct AudioTranscriber {
    openai_api_key: String,
}
```

**Key Methods:**

1. `download_and_transcribe_attachment(url, filename)` - Main entry point
   - Downloads Discord attachment to temp file
   - Calls transcription API
   - Cleans up temp file

2. `transcribe_file(file_path)` - Calls OpenAI API
   - Uses curl to POST to `https://api.openai.com/v1/audio/transcriptions`
   - Model: `whisper-1`
   - Returns transcribed text

## Configuration

### Guild Setting

Audio transcription can be enabled or disabled per-guild:

```
/set_guild_setting setting:audio_transcription value:disabled
```

| Value | Behavior |
|-------|----------|
| `enabled` | Transcribe audio attachments (default) |
| `disabled` | Ignore audio attachments |

**Note:** Audio transcription is always enabled in DMs regardless of guild settings.

### Fallback Behavior

1. Check guild's `audio_transcription` setting
2. If not set, default to `enabled`
3. In DMs, always enabled

## Usage Examples

### Basic Transcription

1. User uploads an audio file (e.g., `voice_memo.m4a`)
2. Bot responds:
   ```
   🎵 Transcribing your audio... please wait!
   ```
3. After processing:
   ```
   📝 **Transcription:**
   Hello, this is a test recording. I'm testing the transcription feature.
   ```

### Transcription with Follow-up

If the user includes text along with the audio attachment:

1. User uploads `meeting_notes.mp3` with message: "Summarize this meeting"
2. Bot transcribes the audio
3. Bot also sends an AI-generated summary based on the transcription

## API Details

### OpenAI Whisper API

- **Endpoint:** `https://api.openai.com/v1/audio/transcriptions`
- **Model:** `whisper-1`
- **Method:** POST with multipart/form-data
- **Authentication:** Bearer token via `Authorization` header

### Request Format

```bash
curl https://api.openai.com/v1/audio/transcriptions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: multipart/form-data" \
  -F "file=@/tmp/discord_audio_filename.mp3" \
  -F "model=whisper-1"
```

### Response Format

```json
{
  "text": "Transcribed text content here..."
}
```

## Limitations

| Limitation | Details |
|------------|---------|
| **File Size** | OpenAI Whisper has a 25MB file size limit |
| **API Costs** | Whisper API costs $0.006 per minute of audio |
| **Processing Time** | Longer files take more time to transcribe |
| **Accuracy** | Depends on audio quality, background noise, accents |
| **Language** | Whisper supports 50+ languages but accuracy varies |

## Error Handling

The bot handles various error conditions:

| Error | User Message |
|-------|--------------|
| Download failed | "Sorry, I couldn't transcribe that audio file. Please make sure it's a valid audio format." |
| Unsupported format | Same as above |
| Empty transcription | "I couldn't hear anything in that audio file." |
| API error | Same as download failed |

## Files

| File | Description |
|------|-------------|
| `src/audio.rs` | AudioTranscriber implementation |
| `src/commands.rs` | `handle_audio_attachments()` method |

## Security Considerations

- Temp files are stored in `/tmp/` with unique filenames
- Files are deleted immediately after transcription
- API key is passed via environment variable, not stored in code
- Downloaded files are never persisted long-term

## Future Enhancements

- [ ] Support for language detection/specification
- [ ] Configurable temp directory
- [ ] Progress indicator for long files
- [ ] File size validation before download
- [ ] Support for voice messages (Discord's native format)
