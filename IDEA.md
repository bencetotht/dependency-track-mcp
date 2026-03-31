# Supply-Chain Security Monitoring Stack Setup

## Goal

Build a low-cost, mostly self-hosted vulnerability monitoring system that gives you:

- one central place to view the security state of repositories and Docker images
- continuous monitoring as new advisories appear
- rapid notifications when a tracked dependency or image becomes vulnerable
- automated remediation through dependency update pull requests / merge requests
- support for GitHub and self-hosted GitLab
- support for npm, Python, Go, Rust, and Docker images

## Recommended Stack

### Central monitor
**Dependency-Track**

Use Dependency-Track as the central inventory, monitoring, and alerting system. It should become the place where you can answer questions like:

- which projects are currently vulnerable
- which Docker images are currently vulnerable
- which components are affected by newly disclosed advisories
- which repositories have the highest current risk
- which projects have unresolved critical issues

### Scanner
**Trivy**

Use Trivy in CI/CD for two main scan targets:

- source repository dependency scanning
- Docker image scanning

This gives you one scanning engine for both application dependencies and container images.

### SBOM format
**CycloneDX**

Generate CycloneDX SBOMs for:

- application dependencies
- built container images

Upload those SBOMs to Dependency-Track after each pipeline run on the default branch and on a nightly schedule.

### Remediation bot
**Renovate**

Use Renovate as the automated update engine for:

- npm, pnpm, yarn
- pip / Poetry
- Go modules
- Cargo
- Docker base images and tags

### Optional extra signal
**Dependabot alerts for GitHub repositories**

For repositories hosted on GitHub, keep Dependabot alerts enabled as an extra source of alerts. It is not the central system, but it is a useful supplementary signal.

---

## High-Level Architecture

Each repository and image should flow through the same pattern:

1. CI/CD checks out the repository.
2. CI/CD generates a CycloneDX SBOM for the application.
3. CI/CD scans the repository dependencies with Trivy.
4. CI/CD builds the Docker image, if the project has one.
5. CI/CD scans the built Docker image with Trivy.
6. CI/CD generates an image SBOM.
7. CI/CD uploads the SBOMs to Dependency-Track.
8. Dependency-Track continuously re-analyzes tracked components as vulnerability feeds update.
9. Dependency-Track sends alerts to Slack, email, webhook, or another notification sink.
10. Renovate opens fix PRs / MRs for vulnerable dependencies or outdated base images.

---

## Recommended Rollout Order

Implement the stack in this order:

1. Stand up Dependency-Track
2. Configure notifications
3. Standardize project naming and versioning
4. Add SBOM generation to pipelines
5. Add Trivy repository scanning
6. Add Trivy Docker image scanning
7. Upload SBOMs to Dependency-Track
8. Deploy Renovate
9. Add policies for failure thresholds and remediation
10. Add nightly scheduled scans

---

## Phase 1: Deploy the Central Monitor

## 1. Deploy Dependency-Track

Run Dependency-Track centrally in your infrastructure.

### Recommended deployment characteristics

- persistent storage enabled
- backed up regularly
- protected with authentication
- exposed only through trusted network paths or a reverse proxy
- API access enabled for CI/CD uploads

### What to model in Dependency-Track

You need a consistent project model.

A practical approach is:

- one **project** per repository
- one **version** per release, build, or commit snapshot
- optionally a separate project for the Docker image if you want repo and image findings shown separately

Examples of naming strategy:

- `billing-service`
- `billing-service-image`
- `auth-platform`
- `auth-platform-image`

### Versioning strategy

Choose one versioning model and keep it consistent:

- release tag
- semantic version
- commit SHA
- branch plus build number for rolling development builds

For central monitoring, semantic version or commit SHA is usually easiest.

---

## 2. Configure Dependency-Track Notifications

Set up alert publishing early so the system becomes useful immediately.

### Good notification targets

- Slack for immediate team visibility
- email for backup and escalation
- webhook for custom routing
- ticketing integration if you want automatic issue creation

### Recommended alert routing

- critical findings -> security channel and email
- high findings -> security channel
- newly affected production images -> priority alert route
- summary reports -> weekly or daily digest

### Recommendation

Start with:

- Slack or webhook for fast response
- email for fallback
- optional issue tracking integration later

---

## Phase 2: Standardize Inventory Inputs

## 3. Decide What Every Pipeline Must Produce

Every production repository should produce these artifacts conceptually:

### Always required
- application dependency SBOM
- repository dependency vulnerability scan result

### Required if the project builds an image
- Docker image vulnerability scan result
- Docker image SBOM

### Optional but useful later
- license report
- secrets scan
- infrastructure misconfiguration scan

For your current goal, dependency and image vulnerability coverage are the priority.

---

## 4. Standardize CycloneDX SBOM Generation

Make CycloneDX the one SBOM format used everywhere.

### Why standardize

You want one central monitoring system and one portfolio view. That only works smoothly if every repository uploads a predictable artifact format.

### SBOM scope expectations

The application SBOM should represent:

- direct dependencies
- transitive dependencies where supported
- resolved versions from lockfiles or manifests

The image SBOM should represent:

- OS packages in the image
- language packages present in the image
- final built artifact composition

---

## Phase 3: CI/CD Pipeline Design

This section is intentionally platform-agnostic so you can implement it in both GitHub and GitLab.

## 5. Pipeline Structure

Each repository pipeline should have the following logical stages.

### Stage A: Preparation

Purpose:
- fetch source
- restore caches if needed
- install build and scan tooling
- determine project name and version
- authenticate to package registries if required
- authenticate to container registry if image build is required
- authenticate to Dependency-Track upload endpoint

What should happen in this stage:
- establish the identity of the project and its version
- make scanner and SBOM tooling available
- load secrets needed later in the pipeline

### Stage B: Dependency Resolution / Build Preparation

Purpose:
- ensure dependencies are resolvable
- confirm lockfiles are present and consistent
- prepare the repository for SBOM generation and scanning

What should happen in this stage:
- validate lockfiles
- restore or install dependencies as needed for accurate analysis
- fail early if dependency metadata is broken or missing

### Stage C: Application SBOM Generation

Purpose:
- generate a CycloneDX SBOM representing the source dependency graph

What should happen in this stage:
- create the application SBOM
- store it as a pipeline artifact
- name it consistently, for example as an application SBOM artifact for that project/version

### Stage D: Repository Dependency Scanning

Purpose:
- scan the repository dependency graph using Trivy

What should happen in this stage:
- analyze lockfiles / dependency manifests
- detect vulnerable packages
- output a machine-readable report
- output a human-readable summary for pipeline visibility

Recommended outcome policy:
- pull request / merge request pipelines should show findings clearly
- default branch and nightly runs may enforce failure on critical findings
- production repositories can optionally also fail on high findings

### Stage E: Docker Build

Only include this stage for repositories that build a container image.

Purpose:
- build the deployable image that will actually run

What should happen in this stage:
- build the image exactly as close to production as possible
- tag it deterministically
- keep track of the exact image reference or digest

### Stage F: Docker Image Scanning

Only include this stage if an image exists.

Purpose:
- scan the built image for OS-level and language-level vulnerabilities

What should happen in this stage:
- scan the image artifact, ideally by exact digest
- produce a machine-readable result
- produce a human-readable summary
- classify severity levels
- identify whether findings come from OS packages, runtime packages, or bundled app dependencies

Recommended outcome policy:
- release pipelines should block on critical findings unless there is an exception process
- nightly image scans should always report, even if the image was built earlier
- prioritize base image vulnerabilities separately from app dependency vulnerabilities

### Stage G: Image SBOM Generation

Only include this stage if an image exists.

Purpose:
- generate a CycloneDX SBOM for the image

What should happen in this stage:
- create an image SBOM from the built image
- store it as a pipeline artifact
- use a naming convention that clearly distinguishes app SBOM from image SBOM

### Stage H: Upload to Dependency-Track

Purpose:
- send SBOMs to the central monitor

What should happen in this stage:
- upload the application SBOM
- upload the image SBOM if present
- associate uploads with the correct project and version
- ensure the upload happens on the default branch and on scheduled runs
- optionally skip uploads for temporary pull request pipelines if you do not want short-lived snapshots in the dashboard

### Stage I: Notification / Summary

Purpose:
- make pipeline results visible quickly

What should happen in this stage:
- publish a short vulnerability summary into the CI/CD job logs
- optionally send a summary to chat for critical failures
- point maintainers to the Dependency-Track dashboard for full details

---

## 6. CI/CD Trigger Strategy

You asked for constant monitoring, but there are two different kinds of “constant” here:

- scan when your code changes
- re-check tracked software when the world changes

You need both.

### Trigger type 1: pull request / merge request
Use this for:
- fast feedback before merging
- preventing known-vulnerable dependencies from being merged
- validating dependency updates raised by Renovate

### Trigger type 2: default branch push
Use this for:
- capturing the latest state of the main branch
- generating authoritative SBOMs
- keeping central inventory current

### Trigger type 3: nightly scheduled pipeline
Use this for:
- re-scanning even if the repo has not changed
- rechecking images that may now be affected by newly disclosed vulnerabilities
- refreshing central state

### Trigger type 4: release or tag pipeline
Use this for:
- scanning the exact release artifact
- uploading release-grade SBOMs
- gating deployment or promotion

### Recommendation

At minimum, every important repository should have:
- PR / MR scanning
- default branch scanning
- nightly scheduled scanning

---

## 7. What Should Be in Every CI/CD Pipeline

This is the practical checklist.

### Common required inputs
- repository source
- lockfiles / manifests
- CI secret for Dependency-Track API access
- CI secret for container registry access, if relevant
- deterministic project naming and versioning
- scanner configuration
- severity thresholds

### Common required outputs
- application SBOM artifact
- dependency scan report artifact
- image scan report artifact, if applicable
- image SBOM artifact, if applicable
- summary in logs
- upload confirmation to Dependency-Track

### Common policy controls
- severity threshold for failing builds
- allowlist / suppression process for accepted risk
- branch rules for enforcement
- retention rules for artifacts
- clear distinction between informational scans and blocking scans

### Common governance metadata
- project owner
- environment classification
- repo criticality
- production vs non-production
- notification routing target

Adding metadata helps you triage faster later.

---

## 8. Pipeline Policies by Pipeline Type

### Pull request / merge request pipelines
Recommended behavior:
- run dependency scan
- run image scan if image build is cheap enough
- produce readable summary
- fail on critical findings
- optionally warn but do not fail on high findings early in rollout

### Default branch pipelines
Recommended behavior:
- run full dependency scan
- build and scan the image
- generate and upload SBOMs
- fail on critical findings
- optionally fail on high findings for critical services

### Nightly scheduled pipelines
Recommended behavior:
- run full dependency scan even if nothing changed
- rebuild or at least rescan the current image reference
- upload fresh SBOMs if dependency state changed
- send summary notifications for new critical/high findings
- do not rely only on code-change triggers

### Release pipelines
Recommended behavior:
- scan the exact build artifact
- scan the exact deployable image digest
- upload final release SBOMs
- block promotion on policy failure unless formally overridden

---

## Phase 4: Docker-Specific Guidance

## 9. How to Treat Docker Images in the Model

You said you want to centrally monitor Docker images too. Treat them as first-class assets.

### Recommended image tracking model

For each deployable image, track:

- image name
- exact tag
- digest
- owning repository
- owning service
- environment classification
- last successful scan time

### Important distinction

A repository can be clean while the built image is vulnerable.

Why:
- the image can inherit vulnerable OS packages
- the base image can change risk over time
- the runtime layer may contain components not visible from the source lockfile view

This is why image scanning and image SBOM upload must be separate from source dependency scanning.

### Base image hygiene rules

Your CI/CD process should also include policy awareness around:
- pinning base images predictably
- preferring digests for sensitive production workloads
- making base image updates visible through Renovate

---

## 10. Suggested Docker-Related Pipeline Steps

For repositories with images, ensure the pipeline conceptually does all of the following:

- build the image
- identify the exact output image tag and digest
- scan that exact image artifact
- generate an image SBOM
- upload the image SBOM to Dependency-Track
- store the digest in logs or release metadata

### If image build is expensive

If building on every PR / MR is too expensive:
- still scan source dependencies on every PR / MR
- reserve full image build and image scan for default branch and nightly pipelines

---

## Phase 5: Dependency-Track Operating Model

## 11. How Dependency-Track Should Be Used Day to Day

Dependency-Track is not just a place to upload reports. It should be your daily monitoring console.

### What maintainers should use it for
- check current vulnerability state of a service
- see newly disclosed issues affecting old builds
- identify repeated problem dependencies across projects
- prioritize the noisiest or highest-risk projects
- confirm whether a fix reduced risk after a Renovate merge

### What security operations should use it for
- watch for new critical findings across the portfolio
- watch for vulnerable base images used by multiple projects
- group and prioritize cross-cutting dependency issues
- audit whether important repositories are still uploading fresh SBOMs

### Freshness rule

A project that has not uploaded a fresh SBOM recently should be treated as lower-confidence inventory.

Add internal expectations such as:
- production services must upload fresh SBOMs at least daily
- lower-priority repos must upload at least weekly

---

## 12. Project Organization Suggestions in Dependency-Track

A simple structure works best.

### Option A: Separate app and image projects
Use this if you want the clearest split between:
- source dependency issues
- container image issues

Good for:
- mature teams
- large portfolios
- services with multiple image variants

### Option B: Single project per service version
Use this if you want fewer dashboard objects and are comfortable combining findings.

Good for:
- smaller setups
- early rollout
- fewer moving parts

### Recommendation

Start with separate app and image entries only for:
- production services
- security-critical assets
- images that are reused widely

Use a simpler model for everything else.

---

## Phase 6: Deploy Renovate

## 13. Renovate Deployment Strategy

Run Renovate centrally rather than individually per repository if possible.

### What Renovate should manage
- package ecosystem updates
- lockfile refreshes
- Docker base image updates
- digest pinning where suitable
- grouped non-security updates
- isolated security updates

### Operational expectations
- Renovate should run on a schedule
- security-related updates should be prioritized
- patch and minor fixes can be candidates for automerge after tests
- major version upgrades should usually stay manual

### Best practice categories

#### Security updates
- always separate from normal maintenance updates
- highest priority
- fastest review path

#### Normal dependency maintenance
- grouped by ecosystem or service
- lower priority
- scheduled to reduce noise

#### Docker base image updates
- visible independently
- reviewed carefully for runtime changes
- tested with the same CI/CD security gates

---

## 14. Renovate Policy Guidance

Use governance rules conceptually like these:

- security updates create dedicated PRs / MRs
- production services get tighter schedules than hobby projects
- critical repos allow automerge only after full test and scan success
- non-production repos can batch non-security updates more aggressively
- Docker digest or tag changes should be clearly labeled

Renovate is the “act immediately” part of the stack once a vulnerability is identified.

---

## Phase 7: Notifications and Response Model

## 15. Rapid Notification Design

Your original requirement emphasized rapid notification. Build around that explicitly.

### Alert routing recommendation

#### Immediate route
Send to:
- Slack security channel or equivalent
- webhook integration to your internal systems

Trigger on:
- new critical vulnerability affecting a tracked project
- new critical vulnerability affecting a tracked image
- failed release scan due to policy violation

#### Secondary route
Send to:
- email

Trigger on:
- daily summary
- high findings on important systems
- stale SBOM uploads
- repeated unresolved critical findings

### Keep noise manageable

Do not notify equally for everything. Prioritize by:
- severity
- production impact
- exploitability if available
- whether the asset is internet-facing
- whether the issue affects many projects

---

## 16. Response Workflow

A practical response model is:

1. Dependency-Track or CI identifies the issue.
2. Notification is sent.
3. Maintainer checks whether Renovate already opened a fix PR / MR.
4. If yes, prioritize review and merge.
5. If no automatic update exists, create manual remediation work.
6. After the fix merges, CI rescans.
7. Dependency-Track confirms the issue is resolved or reduced.

This keeps the flow tight and measurable.

---

## Phase 8: Rollout Strategy Across Many Repositories

## 17. How to Roll This Out Incrementally

Do not try to perfect every repo on day one.

### Wave 1
Start with:
- internet-facing production services
- repos that publish critical Docker images
- projects with many dependencies
- repos with frequent releases

### Wave 2
Add:
- internal business-critical services
- shared libraries
- scheduled jobs and workers

### Wave 3
Add:
- lower-priority tools
- hobby or experimental repositories
- archived but still deployed assets

### Why this matters

You want to get the central monitor producing useful alerts early rather than spending weeks building perfect coverage for low-risk repos.

---

## 18. Baseline Standards for All Repositories

Create a lightweight internal standard saying each managed repository must have:

- lockfiles committed where appropriate
- deterministic dependency resolution
- a CI/CD security stage
- SBOM generation on default branch
- scheduled nightly scan
- owner metadata
- notification routing

For Dockerized repositories, also require:
- image build stage
- image scan stage
- image SBOM generation

---

## Phase 9: Governance and Hardening

## 19. Scanner and Tooling Hardening

Because this stack itself is part of your supply-chain defense, harden it too.

### Hardening rules
- pin scanner versions rather than using floating latest
- prefer immutable references where possible
- review tool updates through the same change process as application dependencies
- restrict who can change CI/CD security stages
- protect secrets used for Dependency-Track uploads and registry access

### Why this matters

Security tooling is part of your trust chain. Treat it like production infrastructure.

---

## 20. Suppressions and Risk Acceptance

You will eventually have false positives, deferred fixes, or unfixable upstream issues.

Define a lightweight process for:
- temporary suppressions
- documented risk acceptance
- expiration dates on exceptions
- ownership of unresolved critical issues

Do not let exceptions become invisible. The dashboard should still reflect that a known issue exists, even if temporarily accepted.

---

## 21. Measuring Success

Track whether the system is working by measuring:

- percentage of important repos covered
- percentage of important images covered
- average SBOM freshness
- time from advisory disclosure to alert
- time from alert to fix PR / MR
- time from fix PR / MR to merge
- number of unresolved critical vulnerabilities

These metrics will tell you whether the rollout is actually improving your response time.

---

## Final Recommended Operating Model

Use this as the stable end state:

- Dependency-Track is the central portfolio dashboard
- Trivy scans source dependencies and Docker images in CI/CD
- CycloneDX SBOMs are generated for apps and images
- SBOMs are uploaded on default branch and nightly schedules
- Renovate opens update PRs / MRs for dependencies and base images
- GitHub Dependabot alerts remain enabled only as an extra GitHub-native signal
- production repositories and images are monitored most aggressively

---

## Short Checklist

### Infrastructure
- deploy Dependency-Track
- secure it
- enable notifications
- create API access for CI/CD

### Per repository
- define project name and version mapping
- generate application SBOM
- run dependency scan
- build image if applicable
- run image scan if applicable
- generate image SBOM if applicable
- upload SBOMs to Dependency-Track
- add nightly schedule

### Portfolio automation
- deploy Renovate
- prioritize security updates
- manage Docker base image updates
- route alerts to chat and email

### Governance
- define failure thresholds
- define exception process
- require owners and metadata
- measure coverage and response time

---

## Closing Guidance

Do not think of this as just “add a scanner.” The system works best when you treat it as a continuous loop:

**inventory -> scan -> centralize -> alert -> remediate -> rescan**

That loop is what will let you keep up across many repositories, many ecosystems, GitHub, self-hosted GitLab, and Docker images without relying on memory or manual tracking.
