# Changelog

## 0.1.4 — 2026-11-10

- Config reading, request signing and the HTTP client moved to the new
  [oci-request-signer](https://crates.io/crates/oci-request-signer) crate, shared
  with oci-free-keeper and oci-free-audit. No change in behaviour.
- `oci-arm-catcher-core`: the `config`, `signer` and `error` modules are gone;
  `Profile`, `Client`, `Error` and friends are re-exported from
  oci-request-signer, the Compute/Identity calls are the `Api` trait, and
  `connect()` builds the client with the catcher's user agent and
  `OCI_ARM_CATCHER_ENDPOINT`.

## 0.1.3 — 2026-10-27

- Update notice: at most once a day the binary checks GitHub for a newer
  release and prints how to upgrade (brew, installer or cargo), on stderr. It
  never delays a command; off in CI, without a terminal, or with
  `OCI_ARM_CATCHER_NO_UPDATE_CHECK=1`.

## 0.1.2 — 2026-10-27

- The bash / PowerShell scripts, their helpers and tests moved to `legacy/`,
  with their own README. They still read the `.env` in the repo root. The old
  root layout is kept at the `scripts-final` tag.
- README: one copyable block per install method.
- `check` reports the tenancy's home region and `run` warns when the profile
  points elsewhere: Always Free instances can only be created in the home
  region. The 401 hint now mentions unsubscribed regions too.
- README: home-region requirement, Oracle's idle-instance reclamation, the full
  Always Free compute allowance; boot volume default is 50 GB.

## 0.1.1 — 2026-10-27

- Colour output on terminals: dim headers, red error codes, yellow countdown,
  green success. `NO_COLOR` turns it off.
- `discover` prints `SSH_KEY_FILE="$HOME/.ssh/..."`, so the line works on any
  machine.
- `OCI_ARM_CATCHER_ENDPOINT` points the client at another base URL (mock API
  for tests and the README demos).
- Published to crates.io: `cargo install oci-arm-catcher`.

## 0.1.0 — 2026-10-27

First release of the Rust binary.

- One self-contained binary for macOS, Linux and Windows that talks to the OCI
  API directly — no OCI CLI, no Python.
- `run` (default) retries `LaunchInstance` until capacity appears, rotating
  availability domains; read-only `check` and `discover`.
- Errors are classified by HTTP status: 4xx stops, 429 / 5xx / network retry.
  After a network error the same request is resent with the same
  `opc-retry-token`, so a lost response cannot create a second instance.
- Reads the classic scripts' `.env`; flag > environment variable > `.env`.
