# Dependency-Track CI Template

This repository is a reusable CI template for feeding Dependency-Track from GitHub Actions and GitLab CI.

It gives you one repeatable pipeline for:

- generating an application CycloneDX SBOM from the checked-out repository
- scanning repository dependencies with Trivy
- optionally building and scanning a Docker image
- optionally generating a CycloneDX SBOM for that image
- uploading authoritative SBOMs to Dependency-Track on the default branch and on scheduled runs

The template is designed for multi-language repositories. Trivy can inspect common dependency manifests and lockfiles for ecosystems such as npm, Python, Go, and Rust without needing language-specific scanner setup in the pipeline itself.

## What This Repo Contains

- [`.gitlab-ci.yml`](/private/tmp/synk-alternative-stack/.gitlab-ci.yml): GitLab runner pipeline
- [`.github/workflows/dependency-track.yml`](/private/tmp/synk-alternative-stack/.github/workflows/dependency-track.yml): GitHub Actions workflow
- [`IDEA.md`](/private/tmp/synk-alternative-stack/IDEA.md): design notes and operating model

## Pipeline Behavior

Both CI implementations follow the same model:

1. Derive project metadata.
2. Install a pinned Trivy release.
3. Generate an application SBOM from the repository.
4. Produce a JSON dependency vulnerability report.
5. Upload the application SBOM inventory to Dependency-Track on authoritative runs.
6. Enforce the configured vulnerability gate.
7. If image mode is enabled:
   - build the Docker image
   - scan the image
   - generate an image SBOM
   - upload the image SBOM to a separate Dependency-Track project on authoritative runs

Authoritative runs are:

- default-branch runs
- scheduled nightly runs
- manual runs on the default branch

Pull request and merge request pipelines still scan and publish artifacts, but they do not upload short-lived snapshots to Dependency-Track by default.

## Recommended Operating Model

The best-practice model is to use CI and Dependency-Track for different, complementary jobs:

- CI is the push-time gate. It answers: should this change merge or release right now?
- Dependency-Track is the continuous monitor. It answers: what are we currently exposed to across all tracked apps and images, including issues disclosed after the last push?

For this repo, that means:

- keep Trivy in CI for fast feedback on pull requests, merge requests, and default-branch pushes
- keep uploading CycloneDX SBOMs to Dependency-Track from default-branch and scheduled runs
- enable Dependency-Track analyzers and vulnerability data sources so uploaded SBOMs are continuously re-analyzed
- optionally enable Dependency-Track's Trivy integration using a private Trivy server for closer parity with CI scan results

This two-layer model is what mature teams commonly use: a fast CI gate plus a central continuously re-evaluated SBOM inventory.

## CI Findings vs Dependency-Track Findings

The pipeline does not upload the `repo-vulns.json` or `image-vulns.json` artifacts into Dependency-Track.

Instead:

- CI uses Trivy directly and can fail immediately on critical or high findings
- Dependency-Track ingests the CycloneDX SBOM and performs its own vulnerability analysis asynchronously
- Dependency-Track then keeps re-evaluating the project over time as its analyzers and vulnerability feeds update

That distinction matters:

- a CI job can fail before Dependency-Track has finished analysis
- Trivy and Dependency-Track may not produce identical findings if Dependency-Track analyzers are not fully configured
- enabling Dependency-Track's Trivy datasource is the best way to make Dependency-Track's view look more like your CI Trivy results

## Dependency-Track Project Model

This template uses:

- one app project per repository
- one separate image project per repository when image mode is enabled
- commit SHA as the default project version

Default naming:

- app project: repository name
- image project: `<repository>-image`

You can override both names with CI variables.

## Why The Workflow Avoids Trivy GitHub Actions

On March 19, 2026, Aqua disclosed a supply-chain attack affecting `trivy-action`, `setup-trivy`, and the malicious `trivy` `v0.69.4` release. Aqua’s public guidance says to move to known-safe versions and pin immutable references.

This template therefore:

- does not use `aquasecurity/trivy-action`
- does not use `aquasecurity/setup-trivy`
- installs a pinned Trivy binary directly
- defaults to `TRIVY_VERSION=0.69.3`, which Aqua lists as a known-safe version

If you change the pinned version later, review Aqua’s release and advisory guidance first.

## Required Configuration

### Dependency-Track prerequisites

Before enabling either pipeline, confirm that:

- Dependency-Track is reachable from your CI runners
- you created an API key with permission to upload BOMs
- automatic project creation is acceptable in your environment
- notifications and policy handling are configured on the Dependency-Track side as needed
- Dependency-Track analyzers and data sources are configured, otherwise you may see components without vulnerability findings

The pipeline uploads BOMs with:

- `POST /api/v1/bom`
- `autoCreate=true`
- `projectName`
- `projectVersion`

That means you do not need to pre-create project UUIDs for the template to work.

> [!NOTE]
> `BOM_UPLOAD` alone is not sufficient when the pipeline uses `autoCreate=true`.
> If you want Dependency-Track to auto-create missing projects from CI, the API key's team also needs the `PROJECT_CREATION_UPLOAD` permission, or a broader permission set that includes project creation.
> If you do not want to grant project-creation rights, pre-create the projects in Dependency-Track and adapt the pipeline to upload to existing projects only.

## Self-Hosted Dependency-Track Setup

This repository includes a local Compose stack in [`docker/compose.yaml`](/private/tmp/synk-alternative-stack/docker/compose.yaml) with:

- Dependency-Track API server
- Dependency-Track frontend
- PostgreSQL
- a private Trivy server on the internal Docker network

The Trivy server is not published publicly and is intended only for Dependency-Track's Trivy datasource. The CI pipelines remain unchanged and still install their own pinned Trivy binaries.

### Start the stack

1. Copy [`docker/.env.example`](/private/tmp/synk-alternative-stack/docker/.env.example) to `docker/.env`.
2. Set strong values for `DTRACK_DB_PASSWORD` and `TRIVY_SERVER_TOKEN`.
3. Start the stack from the `docker` directory with `docker compose up -d`.
4. Wait for Dependency-Track's initial mirroring to finish before judging vulnerability results.

> [!NOTE]
> Dependency-Track's initial startup mirrors multiple vulnerability sources and can take 10 to 30 minutes or more. During that period, BOM ingestion may work while vulnerability analysis is still incomplete.

### First login and immediate hardening

Dependency-Track creates a default administrative account on first startup:

- username: `admin`
- password: `admin`

Immediately after first login:

1. Change the admin password.
2. Create a named administrator account for yourself.
3. Reduce use of the default `admin` account.
4. Confirm database backups and persistent volumes are in place.

### Team and API key setup

Create at least one CI automation team, for example `Automation`.

Grant this team:

- `BOM_UPLOAD`
- `PROJECT_CREATION_UPLOAD` if you want the provided pipeline to auto-create missing projects

Create an API key on that team and store it as:

- `DEPENDENCY_TRACK_API_KEY` in GitHub secrets
- `DEPENDENCY_TRACK_API_KEY` in GitLab CI/CD variables

Keep API keys scoped to automation teams rather than user accounts.

### Recommended analyzers and data sources

Dependency-Track's own best-practices guidance recommends at minimum:

- enable `Internal Analyzer`
- enable `Sonatype OSS Index`
- enable `NVD` mirroring
- enable `GitHub Advisories` mirroring

Recommended additional setup for this stack:

- configure `OSS Index` with an API token, because unauthenticated access is no longer supported
- enable the `Trivy` datasource and point it to the private Trivy server included in this repository

Dependency-Track admin panel locations:

- `Administration -> Analyzers -> Internal Analyzer`
- `Administration -> Analyzers -> Sonatype OSS Index`
- `Administration -> Analyzers -> Trivy`
- `Administration -> Vulnerability Sources` or equivalent datasource configuration areas, depending on the exact release

For the Trivy datasource configuration, set:

- `Enable`: `true`
- `Base URL`: `http://trivy:8080`
- `API Token`: the same value as `TRIVY_SERVER_TOKEN` from `docker/.env`
- `Ignore Unfixed`: optional, usually `false` at first so you see the full risk picture

> [!IMPORTANT]
> The Trivy server in this Compose stack is not published publicly and is not routed through Traefik, but it is attached to an outbound-capable Docker network so it can download its vulnerability database updates. Dependency-Track should still talk to it over the internal hostname `trivy` on the internal Docker network.

### Project naming and versioning

Use a consistent model from the beginning:

- app project: repository name
- image project: `<repository>-image`
- project version: commit SHA by default

That matches the pipeline defaults in this repository and makes cross-platform GitHub/GitLab adoption straightforward.

### Notifications and response

Configure Dependency-Track notifications early so continuous monitoring becomes useful:

- Slack, Teams, Mattermost, webhook, or email
- portfolio-level notifications for newly discovered vulnerabilities
- warning/error-level notifications for critical or high-severity findings
- optional stale-SBOM notifications so you know when projects stop reporting fresh inventory

### Repository metadata quality

Dependency-Track works best when components have valid identity data:

- valid PURLs for application dependencies
- valid CPEs where applicable for OS and other non-application components
- committed lockfiles so Trivy can produce stronger SBOMs

If you see components but weak vulnerability coverage, missing or low-quality identifiers are one of the first things to check.

## GitHub Setup

Create these repository or organization settings.

### GitHub secrets

- `DEPENDENCY_TRACK_API_KEY`: required for default-branch and scheduled uploads

### GitHub variables

- `DEPENDENCY_TRACK_URL`: required for default-branch and scheduled uploads
- `DEPENDENCY_TRACK_PROJECT_NAME`: optional, overrides the repository name
- `DEPENDENCY_TRACK_IMAGE_PROJECT_NAME`: optional, overrides `<project>-image`
- `IMAGE_ENABLED`: optional, `true` or `false`, default `false`
- `IMAGE_NAME`: required when `IMAGE_ENABLED=true`
- `IMAGE_TAG`: optional, defaults to the commit SHA
- `IMAGE_BUILD_CONTEXT`: optional, default `.`
- `DOCKERFILE_PATH`: optional, default `Dockerfile`
- `TRIVY_VERSION`: optional, default `0.69.3`
- `TRIVY_TIMEOUT`: optional, default `10m`
- `TRIVY_REPORT_SEVERITIES`: optional, default `UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL`
- `TRIVY_FAIL_SEVERITIES`: optional, default `CRITICAL`

Recommended GitHub setup:

1. Add `DEPENDENCY_TRACK_URL` as a repository variable.
2. Add `DEPENDENCY_TRACK_API_KEY` as a repository secret.
3. If the repo builds a container image, set `IMAGE_ENABLED=true` and `IMAGE_NAME`.
4. Add a nightly schedule if you want a different cadence from the default cron in the workflow.
5. Protect the default branch so authoritative uploads come from trusted runs only.

### GitHub runner requirements

- `ubuntu-latest` hosted runners work out of the box
- self-hosted runners need Docker available if image mode is enabled
- self-hosted runners must have outbound access to both GitHub releases and your Dependency-Track instance

## GitLab Setup

Create these CI/CD variables at project, group, or instance scope.

### Required GitLab variables

- `DEPENDENCY_TRACK_URL`
- `DEPENDENCY_TRACK_API_KEY`

### Optional GitLab variables

- `DEPENDENCY_TRACK_PROJECT_NAME`
- `DEPENDENCY_TRACK_IMAGE_PROJECT_NAME`
- `IMAGE_ENABLED`, default `false`
- `IMAGE_NAME`, required when `IMAGE_ENABLED=true`
- `IMAGE_TAG`, default commit SHA
- `IMAGE_BUILD_CONTEXT`, default `.`
- `DOCKERFILE_PATH`, default `Dockerfile`
- `TRIVY_VERSION`, default `0.69.3`
- `TRIVY_TIMEOUT`, default `10m`
- `TRIVY_REPORT_SEVERITIES`, default `UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL`
- `TRIVY_FAIL_SEVERITIES`, default `CRITICAL`

Recommended GitLab setup:

1. Add `DEPENDENCY_TRACK_API_KEY` as masked and protected.
2. Add `DEPENDENCY_TRACK_URL` as a protected variable.
3. Enable a nightly pipeline schedule.
4. If image mode is enabled, use runners that support Docker-in-Docker with privileged execution.

### GitLab runner requirements

- the repository scan job runs in a lightweight Alpine-based container
- the image scan job uses `docker:dind`
- image mode therefore needs a privileged Docker executor or equivalent runner setup

## Variables and Defaults

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `DEPENDENCY_TRACK_URL` | yes for authoritative uploads | none | Base URL for Dependency-Track, for example `https://dtrack.example.com` |
| `DEPENDENCY_TRACK_API_KEY` | yes for authoritative uploads | none | API key used to upload BOMs |
| `DEPENDENCY_TRACK_PROJECT_NAME` | no | repo name | Dependency-Track application project name |
| `DEPENDENCY_TRACK_IMAGE_PROJECT_NAME` | no | `<project>-image` | Dependency-Track image project name |
| `IMAGE_ENABLED` | no | `false` | Enables Docker build, image scan, and image SBOM upload |
| `IMAGE_NAME` | yes when image mode is enabled | none | Local image reference prefix, for example `registry.example.com/team/service` |
| `IMAGE_TAG` | no | commit SHA | Image tag used for the local build |
| `IMAGE_BUILD_CONTEXT` | no | `.` | Docker build context |
| `DOCKERFILE_PATH` | no | `Dockerfile` | Dockerfile path |
| `TRIVY_VERSION` | no | `0.69.3` | Pinned Trivy version |
| `TRIVY_TIMEOUT` | no | `10m` | Timeout passed to Trivy |
| `TRIVY_REPORT_SEVERITIES` | no | `UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL` | Severities included in JSON artifact reports |
| `TRIVY_FAIL_SEVERITIES` | no | `CRITICAL` | Severities that fail the CI job |

## Expected Artifacts

The jobs publish artifacts even when the vulnerability gate fails.

Repository scan artifacts:

- `artifacts/security/app-sbom.cdx.json`
- `artifacts/security/repo-vulns.json`
- `artifacts/security/repo-gate.txt`

Image scan artifacts when image mode is enabled:

- `artifacts/security/image-sbom.cdx.json`
- `artifacts/security/image-vulns.json`
- `artifacts/security/image-gate.txt`
- `artifacts/security/image-id.txt`
- `artifacts/security/image.iid`

## Adoption Steps

### App-only repositories

1. Copy in the GitHub workflow or GitLab pipeline.
2. Set `DEPENDENCY_TRACK_URL`.
3. Set `DEPENDENCY_TRACK_API_KEY`.
4. Leave `IMAGE_ENABLED=false`.
5. Add a nightly schedule.
6. Run the pipeline once on the default branch and confirm the project appears in Dependency-Track.

### Repositories that build a Docker image

1. Set the same Dependency-Track values.
2. Set `IMAGE_ENABLED=true`.
3. Set `IMAGE_NAME`.
4. Optionally set `IMAGE_BUILD_CONTEXT`, `DOCKERFILE_PATH`, or `IMAGE_TAG`.
5. Ensure the runner can build Docker images.
6. Run the pipeline on the default branch and confirm both the app project and image project appear in Dependency-Track.

## Operating Notes

- The upload happens before the gating step exits non-zero, so Dependency-Track still receives fresh SBOMs from authoritative runs even when the pipeline fails on critical findings.
- The app SBOM and image SBOM are intentionally uploaded to separate Dependency-Track projects to keep source dependency findings separate from container runtime findings.
- The template does not push the Docker image anywhere. It builds locally for scanning. If you also push images in another pipeline, keep the tagging model aligned.
- If your repo has no Docker image, keep image mode disabled instead of relying on file auto-detection.
- Trivy accuracy is best when lockfiles are committed and dependency resolution is deterministic.
- Dependency-Track vulnerability analysis is asynchronous. A project can appear with components and a dependency graph before its findings are fully populated.
- If you want Dependency-Track to align more closely with CI Trivy results, enable Dependency-Track's Trivy datasource and point it at the private Trivy server from the Compose stack.

## Suggested Rollout Policy

Start with:

- `TRIVY_FAIL_SEVERITIES=CRITICAL`
- uploads on default branch and nightly schedules
- image mode only where the service actually produces a deployable container image

Once the rollout is stable, you can tighten:

- `TRIVY_FAIL_SEVERITIES=HIGH,CRITICAL`
- schedule frequency
- suppression governance
- downstream alert routing in Dependency-Track

## Troubleshooting

### Dependency-Track upload fails

Check:

- `DEPENDENCY_TRACK_URL` has no path suffix beyond the base URL
- `DEPENDENCY_TRACK_API_KEY` is valid
- the runner can reach `https://your-dtrack-host/api/v1/bom`
- the CI job is running on the default branch or a schedule if you expected an upload

### Image job fails immediately

Check:

- `IMAGE_ENABLED=true`
- `IMAGE_NAME` is set
- Docker is available on the runner
- the runner is allowed to use Docker-in-Docker on GitLab

### Trivy finds nothing or misses transitive dependencies

Check:

- lockfiles are committed
- the repo uses supported package managers
- the CI job has access to any private registries needed for dependency resolution

### CI found vulnerabilities but Dependency-Track shows none

Check:

- Dependency-Track finished its initial mirroring and project analysis
- `Internal Analyzer`, `OSS Index`, `NVD`, and `GitHub Advisories` are enabled
- `OSS Index` has a configured API token
- the project components have valid PURLs
- the Trivy datasource is enabled in Dependency-Track if you want closer parity with CI Trivy results
- you are waiting for Dependency-Track's asynchronous analysis, not just BOM ingestion
- the BOM was uploaded from an authoritative run and not only from a pull request or merge request

## References

- Dependency-Track CI/CD docs: https://docs.dependencytrack.org/usage/cicd/
- Dependency-Track best practices: https://docs.dependencytrack.org/best-practices/
- Dependency-Track initial startup: https://docs.dependencytrack.org/getting-started/initial-startup/
- Dependency-Track known vulnerability analysis: https://docs.dependencytrack.org/analysis-types/known-vulnerabilities/
- Dependency-Track datasource routing: https://docs.dependencytrack.org/datasources/routing/
- Dependency-Track OSS Index datasource: https://docs.dependencytrack.org/datasources/ossindex/
- Dependency-Track Trivy datasource: https://docs.dependencytrack.org/datasources/trivy/
- Dependency-Track changelog: https://docs.dependencytrack.org/changelog/
- Trivy docs: https://trivy.dev/docs/latest/
- Aqua advisory on the March 2026 incident: https://www.aquasec.com/blog/trivy-supply-chain-attack-what-you-need-to-know/
