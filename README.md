# Dependency-Track CLI

`dtrack` is a Rust CLI for querying Dependency-Track from a terminal or an AI agent workflow. It replaces the old MCP-oriented toolkit with direct commands that agents can discover through `--help`, execute safely, and parse as JSON.

The CLI focuses on the Dependency-Track API areas that are useful during vulnerability remediation:

- project discovery and inspection
- Git repository to Dependency-Track project detection
- project finding export with severity/component/state filters
- vulnerability details and affected project lookup
- current project and portfolio metrics

## Install

```bash
cargo build --release
./target/release/dtrack --help
```

During development:

```bash
cargo run -- --help
```

### Nix

The flake provides `packages.<system>.dtrack` (also the default package) for
`x86_64-linux` and `aarch64-linux`. It pins nixpkgs in `flake.lock` and uses
`Cargo.lock` for Rust dependencies. Git is included in the executable's runtime
PATH for repository detection. Linux Secret Service support is built in; an
unlocked Secret Service is needed only when storing keys in the keyring.

Build, check (including the Rust tests), or run from a checkout:

```sh
nix build
nix flake check
nix run . -- --help
```

To install system-wide from a NixOS flake such as `/home/bence/nixnas`, add this
input after publishing the packaging changes to GitHub:

```nix
inputs.dtrack = {
  url = "github:bencetotht/dependency-track-mcp";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

In a host module that receives `inputs` through `specialArgs` (as nixnas does):

```nix
{ inputs, pkgs, ... }:
{
  environment.systemPackages = [
    inputs.dtrack.packages.${pkgs.stdenv.hostPlatform.system}.dtrack
  ];
}
```

In nixnas, the same package can instead go in `nixnas.packages.extra`. Update
the consumer lock file with `nix flake update dtrack`, then rebuild the desired
host using its usual `nixos-rebuild switch --flake .#HOST` command.
For local integration before publishing, use
`--override-input dtrack path:/home/bence/Developer/dependency-track-mcp` on the
consumer's Nix command after adding the input.

## Configuration

The fastest setup is environment variables:

```bash
export DTRACK_URL="https://dependency-track.example.com"
export DTRACK_API_KEY="odt_..."
dtrack projects list --active
```

For compatibility with existing local setups, `DEPENDENCY_TRACK_API_KEY` is also accepted.

You can also use `dtrack.toml` in the current directory, or pass `--config path/to/dtrack.toml`:

```toml
url = "https://dependency-track.example.com"
api_key_env = "DEPENDENCY_TRACK_API_KEY"
timeout_seconds = 30
retries = 2
```

## Agent-Friendly Usage

Use `--output json` when another program or LLM will consume the output:

```bash
dtrack --output json projects list --active --classifier APPLICATION --sort risk --limit 20
dtrack --output json projects show ds --metrics --properties
dtrack --output json findings list ds --min-severity high --sort severity --limit 50
dtrack --output json findings summary ds
dtrack --output json vulnerabilities projects NVD CVE-2025-62718 --exclude-inactive
```

Common aliases are available for shorter agent commands:

```bash
dtrack projects ls --active
dtrack projects get ds --metrics
dtrack findings ls ds --min-severity high
dtrack vuln ls NVD CVE-2025-62718
```

Use human output for interactive triage:

```bash
dtrack projects list --active --sort risk --limit 10
dtrack metrics project ds
dtrack findings list ds --min-severity critical --limit 10
```

If the current repository has matching Dependency-Track project metadata, omit the project and let the CLI detect it:

```bash
dtrack projects detect .
dtrack findings list --repo .
```

Project detection compares the local Git remote with project properties such as:

- `automation.cloneUrl`
- `automation.repositoryUrl`
- `repo.cloneUrl`
- `repo.repositoryUrl`
- `vcs.cloneUrl`
- `vcs.repositoryUrl`

It also checks Git-like project external references.

## Commands

Run `dtrack --help` or any subcommand help for the full reference:

```bash
dtrack projects --help
dtrack findings list --help
dtrack vulnerabilities --help
dtrack metrics --help
```

Primary command groups:

- `projects list`: list projects with filters for active status, classifier, tags, latest version, sorting, and limits.
- `projects show`: inspect one project by UUID, exact name, or unique name fragment. Optional flags include `--properties`, `--metrics`, and `--findings`.
- `projects detect`: resolve a local Git repository to a Dependency-Track project.
- `findings list`: export findings for a project, filter by severity, component, vulnerability ID, analysis state, and suppression status.
- `findings summary`: count findings by severity and analysis state.
- `vulnerabilities show`: fetch vulnerability details by source and ID.
- `vulnerabilities projects`: list projects affected by a vulnerability.
- `metrics project`: show current metrics for one project.
- `metrics portfolio`: show current portfolio metrics.
- `config`: show resolved non-secret configuration.

Aliases: `ls` for list-like commands, `get` for show-like commands, and `vuln`/`vulns` for `vulnerabilities`.

## Reliability

Findings are fetched across all API pages before filtering, sorting, summarizing,
or resolving a finding for triage. A server-imposed page size does not limit the
results when the API reports the total count.

Every HTTP request has a configurable timeout and retry count:

```bash
dtrack --timeout 10s --retries 4 --verbose projects list --active
```

Retries are used for transient network failures, HTTP 429, and HTTP 5xx responses. HTTP errors include the method, API path, status code, and response body where available.

## Development

```bash
cargo fmt
cargo test
cargo run -- --url https://dt.bencetotht.hu --output json projects list --active --limit 5
```

The CLI uses Dependency-Track's documented OpenAPI endpoints under `/api/v1`. The public Dependency-Track docs note that the OpenAPI spec is served by the backend at `/api/openapi.json` or `/api/openapi.yaml`.


## Contexts and credentials

Install from a checkout with `cargo install --path . --locked`. Release binaries
are built by the Rust CI workflow for Linux and macOS.

`dtrack context add home --url https://dt.example.com` prompts for a hidden key,
validates the connection, and stores the key in macOS Keychain or Linux Secret
Service. Linux desktops need a running, unlocked Secret Service (such as GNOME
Keyring). Headless Linux can use `--api-key-env HOME_DT_KEY` instead.
`--api-key-stdin` accepts a piped key; keys are never saved by these commands
in TOML. Fully specified commands do not prompt.

Manage installations with `context list`, `context use NAME`,
`context current`, `context remove NAME`, and `context test [NAME]`.
Select one per invocation with `-c NAME` or `DTRACK_CONTEXT`.
A repository's `dtrack.toml` may contain `context = "company"`.

The URL and API key always come from the same place, so a key is only ever sent
to the server it belongs to. The first match wins:

1. `--url` (key from `--api-key`, `DTRACK_API_KEY`, or `DEPENDENCY_TRACK_API_KEY`)
2. `-c NAME` / `DTRACK_CONTEXT`
3. `DTRACK_URL` (key from `--api-key` or the API key environment variables)
4. `context = "NAME"` in the repository `dtrack.toml`
5. `current_context` from the user config
6. legacy flat `url` in the repository `dtrack.toml`, then in the user config

While a context is selected, `DTRACK_API_KEY`, `DEPENDENCY_TRACK_API_KEY`, and a
repository `url` are ignored (a warning is printed for the latter); only an
explicit `--api-key` overrides the context's key. Repositories should name a
context rather than a URL. Timeouts and retries come from flags, then the
context, then the repository file, then the user config.
`dtrack config` shows which source was used as `connectionSource`.

The user config is `~/Library/Application Support/dtrack/config.toml` on macOS
and `$XDG_CONFIG_HOME/dtrack/config.toml` (normally
`~/.config/dtrack/config.toml`) on Linux. `--config` selects an alternative
file for both context management and command execution.
`dtrack config` works without a URL or key and never displays credentials.

## Record triage and reanalyze

Exact project names take priority over partial matches. When a name has several
versions, the one marked latest is used; select another with the global
`--project-version VERSION` or pass a project UUID.

```sh
dtrack findings triage PROJECT --component lodash --vulnerability CVE-2021-23337 \
  --state NOT_AFFECTED --justification CODE_NOT_REACHABLE \
  --details "Only _.get is imported; template() is never called" \
  --suppressed true --comment "Reviewed in PR #42"
dtrack projects reanalyze PROJECT
```

`--component` accepts a name, `name@version`, PURL, or component UUID;
`--vulnerability` accepts a vulnerability ID, an alias (such as the CVE behind a
GHSA), or a UUID. Suppressed findings are included, so a finding can be
unsuppressed. If several components match, pass the PURL. Triage also accepts
`--response` (for example `UPDATE` or `WILL_NOT_FIX`) and preserves existing
fields you did not specify. Triage needs the VULNERABILITY_ANALYSIS permission;
reanalysis needs VIEW_VULNERABILITY. Reanalysis is asynchronous:
acceptance does not mean processing has finished. Writes are not automatically
retried, to avoid duplicate audit comments. Read retries mean additional attempts:
`--retries 2` makes at most three requests.
