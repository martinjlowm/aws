# Me on AWS

My personal Infrastructure as Code, IaC, for applications, operational tools and
other goofy projects on AWS.

The `organization/` project can be deployed from a clean root account to set up
the necessary accounts. It expects AWS Organizations to be enabled with
delegated StackSet privileges. `organization/README.md` has the account tree and
the one move that is not code.

Secondly, the CDK bootstrap (`bootstrap/`) project be deployed such that any new accounts added
to the organization will be bootstrapped automatically.

## Projects

- `domains/` holds the hosted zones for `martinjlowm.dk` and `martinlowm.dk`:
  the mail and verification records, plus the role any account in the
  organization assumes to delegate a subdomain to itself. See
  `domains/README.md`, which also covers moving both domains off Cloudflare
  without dropping mail.
- `billing/` watches what the organization costs: Cost Explorer anomaly
  detection over the consolidated bill, and a monthly budget, both mailing the
  management account's root address. See `billing/README.md`, which covers
  enabling Cost Explorer first and why the two alerts are not the same alert.
- `music/` includes a Nix derivation to analyze the beats per minute, BPM, and
  key (Camelot format) of Beatport-downloaded bundles stored in S3 and prepended
  to the file name. The derivation exposes two outputs: raw wav- and FLAC-files
  both which can then be copied straight onto a USB.
- `dubplate/` is the same job done in a browser instead: a bucket, a CloudFront
  distribution, and a Leptos application that runs
  [dubplate](https://github.com/martinjlowm/dubplate) as WebAssembly. Drop a zip
  on the page and it measures every track and writes a FAT32 image both a Pioneer
  and a Denon player will read. Nothing is uploaded and there is no compute in
  the stack. Served at `dubplate.martinjlowm.dk`. See `dubplate/README.md`.

## Getting Started

1. Install Nix, direnv and devenv
2. Enter the project and mark the .envrc safe with `direnv allow`
3. Assume a privileged role for your management account with `assume <profile>`
4. Populate `AWS_ORG` (environmental variable) with the organizational structure
   by reloading direnv: `direnv reload`.

The shell brings its own yarn, bun, Node, awscli2, granted and just, so nothing has to
be installed alongside it, and it installs `node_modules` on entry. It also brings
Rust with the `wasm32-unknown-unknown` target, Trunk, Tailwind and wasm-pack,
which is what `dubplate/app` is built with.

There are two workspaces at this level and they cover different languages:
`package.json` lists the CDK projects, `Cargo.toml` lists the Rust crates. Both
exist so that a lock file and a build directory belong to the repository rather
than to whichever project happened to arrive first.

## Tooling

`just` lists what there is to run. The formatters are one set, declared in
`treefmt.nix` and reachable three ways that cannot disagree: `treefmt` in the
shell, `just fmt`, and the pre-commit hook devenv installs.

rustfmt and taplo cover `dubplate/app`; biome is the formatter and the linter for
TypeScript and JSON. Its configuration
is `biome-config.nix`, from which `biome.json` is written on every shell entry;
the JSON is gitignored, so edit the Nix. The `@biomejs/biome` in
`package.json` is pinned to the version nixpkgs ships, which is what makes an
editor's biome LSP agree with `just fmt`. Bump one and bump the other.
