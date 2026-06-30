# Docker Compose Stack

The repository includes an optional Docker Compose stack for running Dependency-Track with PostgreSQL and a private Trivy server.

## What The Stack Contains

- `apiserver`: Dependency-Track API server
- `frontend`: Dependency-Track frontend
- `postgres`: PostgreSQL database for Dependency-Track
- `trivy`: private Trivy server used by Dependency-Track

The stack uses:

- a private internal Docker network for backend traffic
- an external `traefik_backend` network for public routing
- persistent Docker volumes for Dependency-Track data, PostgreSQL data, and Trivy cache

## Setup

### Prerequisites

- Docker and Docker Compose
- an existing external Docker network named `traefik_backend`
- a running Traefik instance if you want to use the compose file as-is

### Environment File

Copy `docker/.env.example` to `docker/.env` and update the values.

The example defines:

- `DTRACK_VERSION`: pins the Dependency-Track API server and frontend to the same release
- `TRIVY_SERVER_VERSION`: pins the private Trivy server image
- `DTRACK_DOMAIN`: public hostname served by Traefik
- `DTRACK_DB_PASSWORD`: PostgreSQL password for Dependency-Track
- `TRIVY_SERVER_TOKEN`: token used by Dependency-Track to authenticate to the private Trivy server

Use long random values for `DTRACK_DB_PASSWORD` and `TRIVY_SERVER_TOKEN` and keep them secret.

### Start The Stack

From the `docker/` directory:

```bash
docker compose up -d
```

Stop it with:

```bash
docker compose down
```

## Routing Model

The compose file assumes Traefik is already present and uses labels that:

- route `https://<DTRACK_DOMAIN>/api` to the Dependency-Track API server
- route `https://<DTRACK_DOMAIN>/` to the Dependency-Track frontend
- use the `traefik_backend` Docker network
- expect a certificate resolver named `cloudflare`

If your Traefik network name, router setup, or certificate resolver differs, update `docker/compose.yaml` before starting the stack.

## Service Details

### Dependency-Track API Server

- runs in external database mode
- connects to PostgreSQL at `postgres:5432`
- stores application data in the `dtrack-data` volume
- depends on PostgreSQL health checks

### Dependency-Track Frontend

- points `API_BASE_URL` at the same public hostname
- waits for the API server to become healthy

### Trivy Server

- runs as a private server on port `8080`
- authenticates clients with `TRIVY_SERVER_TOKEN`
- persists its cache in the `trivy-cache` volume

### PostgreSQL

- uses `postgres:17-alpine`
- stores database files in the `postgres-data` volume
- enables initdb checksums

## Quick Start Checklist

1. Create the `traefik_backend` network if it does not already exist: `docker network create traefik_backend`.
2. Copy `docker/.env.example` to `docker/.env`.
3. Set the hostname and secret values.
4. Confirm your Traefik installation uses the expected network and `cloudflare` cert resolver, or adjust the compose labels.
5. Start the stack with `docker compose up -d`.

## Notes And Warnings

- This stack is optional and separate from the CLI.
- The compose file is written for a Traefik-based deployment, not a standalone localhost-only setup.
- Keep the Dependency-Track API and frontend pinned to the same `DTRACK_VERSION`.
- Keep the Trivy token secret.
