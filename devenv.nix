{
  pkgs,
  config,
  ...
}: let
  yarn = pkgs.yarn-berry.override {
    nodejs = pkgs.nodejs_24;
  };

  biomeJson = (pkgs.formats.json {}).generate "biome.json" (import ./biome-config.nix {inherit pkgs;});
in {
  # yarn installs, bun runs. The CDK apps (bootstrap, organization, music,
  # dubplate) are TypeScript executed directly, `bun ./deployment/bin/entry.ts`
  # in each cdk.json, so there is no compiler in the loop for those.
  #
  # `dubplate/app` is the exception and the reason for the Rust half below: it is
  # a Leptos application compiled to WebAssembly, and what the CDK stack uploads
  # is what Trunk built.
  packages = [
    pkgs.git
    yarn
    pkgs.bun
    pkgs.awscli2
    pkgs.just
    # `assume <profile>` for the management account, step 3 of the README.
    pkgs.granted
    # The music derivation pins its own copy; this one is for checking what a
    # track analyses to before committing a bundle hash.
    pkgs.keyfinder-cli

    # dubplate/app. Trunk drives cargo, calls wasm-bindgen and then wasm-opt, so
    # all three are here rather than left to whatever is on PATH.
    pkgs.trunk
    pkgs.wasm-bindgen-cli
    pkgs.binaryen
    # The shadcn components are Tailwind classes, and the stylesheet Trunk links
    # is generated from them by the pre_build hook in dubplate/app/Trunk.toml.
    pkgs.tailwindcss
    # The analysis half: dubplate's own crates, built to wasm separately and
    # loaded by the worker. See dubplate/README.md.
    pkgs.wasm-pack
  ];

  # The only C in this build is the SQLite that the Engine exporter compiles, and
  # this shell's clang is nix-wrapped. The wrapper is written for building host
  # binaries and adds two things a wasm target cannot take.
  #
  # `zerocallusedregs` puts `-fzero-call-used-regs=used-gpr` on every compile,
  # which clang refuses for wasm outright, so the compile fails. Dropping it from
  # the hardening list is enough.
  #
  # The stack protector is not, because the wrapper adds it through a path
  # `NIX_HARDENING_ENABLE` does not reach. It compiles and then fails to link with
  # `undefined symbol: __stack_chk_fail`: the guard calls into a libc the wasm
  # shim does not have. `CFLAGS_<target>` is appended last by cc-rs, so this wins
  # over whatever the wrapper put in front of it.
  #
  # Worth knowing that dubplate's own `just check-wasm` cannot catch the second
  # one. `cargo check` never runs the linker, so the symbol is never asked for,
  # and only a real `trunk build` finds it.
  env = {
    NIX_HARDENING_ENABLE = "fortify pic strictoverflow format relro bindnow";
    CFLAGS_wasm32_unknown_unknown = "-fno-stack-protector";
  };

  # rustc, cargo and the wasm32 target. `targets` is what puts the standard
  # library for the browser in the shell; without it every build of the app fails
  # at the first `core` it cannot find.
  languages.rust = {
    enable = true;
    channel = "stable";
    targets = ["wasm32-unknown-unknown"];
  };

  # `languages.javascript.enable` is what registers the yarn install task. This
  # used to be an `if [ ! -d node_modules ]` guard in .envrc, which installed
  # once and then never again: a changed package.json left the tree stale until
  # someone deleted node_modules by hand.
  #
  # Cross-repo sharing comes from .yarnrc.yml (nmMode: hardlinks-global against
  # ~/.yarn/berry/index) and needs nothing here.
  languages.javascript = {
    enable = true;
    package = pkgs.nodejs_24;
    bun.enable = true;
    yarn = {
      enable = true;
      package = pkgs.yarn-berry;
      install.enable = true;
    };
  };

  # `treefmt` on PATH in the shell. The formatter set lives in ./treefmt.nix and
  # is imported rather than restated, so shell and git hook cannot drift apart.
  treefmt = {
    enable = true;
    config.imports = [./treefmt.nix];
  };

  # One hook, and it runs the same formatters as `just fmt`. It replaces a biome
  # hook that passed `--apply` (removed in biome 2) and a prettier hook for
  # `.gql`/`.yaml` files, of which this repo has none.
  git-hooks.hooks.treefmt.enable = true;

  # biome.json is generated, not committed, and this is where it is written.
  #
  # It has to exist as a FILE at the repo root: `yarn lint` and every editor's
  # biome LSP find their config by walking up from the file being edited, and
  # neither will look in the Nix store. Writing it on shell entry rather than
  # committing it means there is one source (biome-config.nix) and no second copy
  # anyone can edit.
  #
  # A copy rather than a symlink into the store: biome resolves the globs in
  # `files.includes` against the config's own directory, and a symlink is one
  # resolution decision away from making that directory the store path.
  #
  # The version is baked into the derivation, so a nixpkgs bump changes the
  # output path and the next shell entry rewrites the file.
  enterShell = ''
    ${pkgs.coreutils}/bin/install -m 0644 ${biomeJson} "${config.devenv.root}/biome.json"
  '';

  # `devenv build outputs.music`: the BPM/key-prefixed wav and flac trees built
  # from the S3 bundles. Reads AWS_ORG, which .envrc populates.
  #
  # dubplate/app has no output here on purpose. Trunk resolves crates from
  # crates.io while it builds, which a derivation cannot do without a vendored
  # crate graph, and the application is deployed from `just build-app` rather
  # than from the store. If it ever needs to be reproducible, the answer is
  # crate2nix and a committed Cargo.nix, which is what the dubplate repo already
  # does for the CLI.
  outputs = {
    music = import ./music/nix/default.nix {inherit pkgs config;};
  };
}
