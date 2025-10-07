# Docker Configuration Directory

This directory should contain your `config.json` file when running Guardia Hub in Docker.

## Setup

1. Copy the example configuration:
```bash
cp ../config.example.json config.json
```

2. Edit `config.json` with your settings:
   - Update server addresses and tokens
   - Configure alert webhooks (Discord, etc.)
   - Set storage path to `/app/data/metrics.db` for persistence

3. Start the container:
```bash
docker compose up -d
```

## Important Notes

- The config directory is mounted as **read-only** (`:ro`) in the container
- For persistent metrics storage, ensure `storage.path` uses `/app/data/` prefix
- Example: `"path": "/app/data/metrics.db"`

See [DOCKER.md](../DOCKER.md) for complete Docker deployment documentation.
