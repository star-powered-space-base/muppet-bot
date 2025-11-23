# Scripts Directory

Bot management scripts organized by category.

## Directory Structure

```
scripts/
├── Makefile              # Scoped make targets
├── README.md             # This file
├── analytics/            # Reporting and analytics
│   └── generate_report.py
├── commands/             # Discord command management
│   ├── check.sh         # Check registered commands
│   └── cleanup.sh       # Remove duplicate registrations
├── maintenance/          # Database and system maintenance
│   └── cleanup.sh       # Cleanup old data
├── service/              # Systemd service management
│   └── reload.sh        # Reload service configuration
├── sql/                  # SQL query templates
│   ├── conversation_stats.sql
│   ├── daily_analytics.sql
│   ├── db_info.sql
│   ├── error_report.sql
│   ├── performance_metrics.sql
│   └── user_stats.sql
├── tunnel/               # ngrok tunnel management
│   ├── setup.sh         # Configure ngrok
│   ├── start-http.sh    # Start with HTTP tunnel
│   └── start-gateway.sh # Start with gateway tunnel
└── test/                 # Testing utilities
    ├── env.sh           # Validate environment
    └── openai.sh        # Test OpenAI connectivity
```

## Usage

### From Project Root

```bash
# Using make
make scripts/commands/check
make scripts/service/reload
make scripts/test/all

# Or directly
./scripts/commands/check.sh
./scripts/service/reload.sh
```

### From Scripts Directory

```bash
cd scripts
make commands/check
make service/reload
make test/all
```

## Available Targets

### Command Management

| Target | Description |
|--------|-------------|
| `commands/check` | Check registered Discord commands (global and guild-specific) |
| `commands/cleanup` | Remove duplicate command registrations |

### Service Management

| Target | Description |
|--------|-------------|
| `service/reload` | Reload systemd service after config changes |
| `service/status` | Check current service status |

### Tunnel Management

| Target | Description |
|--------|-------------|
| `tunnel/setup` | Configure ngrok tunnel for HTTP interactions |
| `tunnel/start-http` | Start bot with HTTP endpoint via ngrok |
| `tunnel/start-gateway` | Start bot with gateway connection via ngrok |

### Testing

| Target | Description |
|--------|-------------|
| `test/env` | Validate environment configuration (.env file) |
| `test/openai` | Test OpenAI API connectivity and timeout handling |
| `test/all` | Run all tests |

## Options

### Passing Arguments

Use `ARGS` to pass additional arguments to scripts:

```bash
make commands/check ARGS='--verbose'
make commands/cleanup ARGS='--dry-run'
```

### Test Timeout

Set custom timeout for OpenAI test:

```bash
make test/openai TIMEOUT=30
```

## Migration from Root Scripts

| Old Location | New Location |
|--------------|--------------|
| `check-commands.sh` | `scripts/commands/check.sh` |
| `cleanup-commands.sh` | `scripts/commands/cleanup.sh` |
| `reload-service.sh` | `scripts/service/reload.sh` |
| `setup-ngrok.sh` | `scripts/tunnel/setup.sh` |
| `start-http-with-tunnel.sh` | `scripts/tunnel/start-http.sh` |
| `start-with-tunnel.sh` | `scripts/tunnel/start-gateway.sh` |
| `test_env_loading.sh` | `scripts/test/env.sh` |
| `test_openai_timeout.sh` | `scripts/test/openai.sh` |
