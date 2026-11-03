# Development guide

How to set up a development environment for NovaVault on Windows, macOS and Linux, which commands to run, and where things live in the repository. Contribution rules (branches, commits, code style, review) are in [CONTRIBUTING.md](../CONTRIBUTING.md).

> [!NOTE]
> **Partly coming soon.** The application scaffolding has not been committed yet. The prerequisites below are final; commands marked *coming soon* will be filled in with the first code drop.

## Prerequisites (all platforms)

- **Git.**
- **Rust** stable toolchain via [rustup](https://www.rust-lang.org/tools/install), with the `rustfmt` and `clippy` components:

  ```sh
  rustup toolchain install stable
  rustup component add rustfmt clippy
  ```

- **Node.js** LTS from [nodejs.org](https://nodejs.org/) or a version manager, with `npm`.
- The **Tauri 2 system prerequisites** for your platform, summarized below. The [official Tauri prerequisites page](https://tauri.app/start/prerequisites/) is the reference if anything here is out of date.

## Platform setup

### Windows

1. Install the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and select the **Desktop development with C++** workload.
2. Make sure **Microsoft Edge WebView2** is installed. It ships with Windows 10 (version 1803 and later) and Windows 11; otherwise install the Evergreen runtime from Microsoft.
3. Install Rust with `rustup` using the default MSVC toolchain (`stable-x86_64-pc-windows-msvc`).
4. Install Node.js LTS.

Line endings are normalized by `.gitattributes`; no special Git configuration is needed.

### macOS

1. Install the Xcode Command Line Tools:

   ```sh
   xcode-select --install
   ```

2. Install Rust with `rustup` and Node.js LTS (for example with Homebrew: `brew install node`).

### Linux

Install the WebKitGTK 4.1 development stack and build tools. Examples for common distributions:

**Debian / Ubuntu**

```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**Fedora**

```sh
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```

**Arch Linux**

```sh
sudo pacman -Syu
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```

Then install Rust with `rustup` and Node.js LTS.

## Getting the code

```sh
git clone https://github.com/acevflow/NovaVault.git
cd NovaVault
```

## Commands

| Task | Command | Status |
| ---- | ------- | ------ |
| Install UI dependencies | *TBD* | Coming soon |
| Full local check (format, lint, test for Rust and TypeScript) | *TBD, a single command* | Coming soon |
| Run the app in development mode | *TBD (Tauri dev)* | Coming soon |
| Build a release bundle | *TBD (Tauri build)* | Coming soon |
| Rust formatting | `cargo fmt --all` | Once the workspace exists |
| Rust lints | `cargo clippy --all-targets -- -D warnings` | Once the workspace exists |
| Rust tests | `cargo test --workspace` | Once the workspace exists |

The full local check is the one command to run before every push. CI ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)) runs the same checks on Windows, macOS and Linux; steps for a component (Rust workspace, UI) are skipped until that component exists in the repository.

## Project layout

```text
crates/novavault-core/   Rust library: crypto, vault format, key slots (UI-independent)
src-tauri/               thin Tauri 2 shell: commands, OS hooks, CSP, capabilities
ui/                      React + TypeScript front end
docs/                    architecture, threat model, vault format, this guide
assets/                  logo and other static assets
.github/                 CI workflow, Dependabot, issue and PR templates
README.md                project overview and status
SECURITY.md              vulnerability reporting policy
CONTRIBUTING.md          contribution rules
CODE_OF_CONDUCT.md       Contributor Covenant 2.1
CHANGELOG.md             Keep a Changelog, Unreleased section
LICENSE                  GPL-3.0-or-later
```

See [architecture.md](architecture.md) for how these components interact.

## Working safely with test data

- Never use real passwords, personal data or real vault files in tests, examples, screenshots or bug reports.
- Vault files, build output (`target/`) and `node_modules/` are ignored by Git and must never be committed.
- Prefer low Argon2id parameters only inside tests; production defaults must not be weakened for convenience.

## Troubleshooting

- **Linux: `webkit2gtk-4.1` not found.** Your distribution may be too old to ship WebKitGTK 4.1. Tauri 2 requires it; use a recent release.
- **Windows: linker `link.exe` not found.** The C++ Build Tools workload is missing or the GNU toolchain is active. Run `rustup default stable-msvc`.
- **Anything else:** check the [Tauri prerequisites](https://tauri.app/start/prerequisites/) and open an issue if the problem is specific to NovaVault.
