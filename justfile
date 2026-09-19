_default:
    @just --list

# Type-check every workspace against the root tsconfig
check-ts:
    yarn typecheck

# biome.json is generated on devenv shell entry and is gitignored, so a bare
# shell may not have one. Biome's own failure without a config is a stack
# overflow from walking node_modules, which names nothing, hence the guard.
[doc("Lint + format check")]
lint:
    @test -f biome.json || { \
      echo 'no biome.json: it is generated from biome-config.nix on devenv shell entry.'; \
      echo 'run `direnv allow`, or `devenv shell -- just lint`.'; \
      exit 1; \
    }
    yarn lint

# Format everything treefmt owns (nix, shell, ts, json), see treefmt.nix
fmt:
    treefmt

# Same formatters, check-only: fails if anything is unformatted
fmt-check:
    treefmt --ci

# Every gate the tooling can answer for, in one go. check-app needs the wasm32
# target, which the devenv shell provides.
check: lint check-ts check-app

# Synthesize one project's templates: `just synth organization`
synth PROJECT:
    cd {{PROJECT}} && yarn cdk synth

# Needs credentials for the account the stacks name, so run `assume <profile>`
# first (README, step 3).
#
# `*` rather than nothing: dubplate is two stacks, and cdk refuses to guess which
# one an app with more than one means. Pass a stack id as ARGS to narrow it.
[doc("Deploy one project: `just deploy organization`")]
deploy PROJECT *ARGS:
    cd {{PROJECT}} && yarn cdk deploy {{ if ARGS == "" { "'*'" } else { ARGS } }}

# The BPM/key-prefixed wav and flac trees, built from the S3 bundles
music:
    devenv build outputs.music

# dubplate/app: the Leptos application, built to WebAssembly.
#
# Two binaries out of one crate, not two builds. The interface is `main.rs` and
# the analysis is `src/bin/analysis.rs`, which links dubplate's crates as a cargo
# dependency; Cargo.toml at the root says which checkout that is. There is
# nothing to clone and nothing to build first.

# Build the interface. Writes dubplate/app/dist, which is what the stack uploads.
build-app:
    cd dubplate/app && trunk build --release

# The application on localhost:8080, rebuilt as you edit it
serve-app:
    cd dubplate/app && trunk serve

# Type-check the application against the browser target
check-app:
    cargo check --target wasm32-unknown-unknown -p dubplate-web

# `direnv reload` is what actually puts AWS_ORG in the shell, and only it can: a
# recipe runs in a subprocess and cannot export into its parent. This prints what
# a reload would read, for finding out why one said the structure is out of sync
# — the loader names the field that did not match.
[doc("Print the organization as .envrc reads it")]
org:
    @bun organization/scripts/load.ts
