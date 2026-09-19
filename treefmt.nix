{pkgs, ...}: {
  # No projectRootFile: devenv supplies the root, so there is no filename here to
  # go stale.
  programs.alejandra.enable = true; # devenv.nix, music/nix/default.nix
  programs.shfmt.enable = true; # .envrc

  # biome, for .ts/.json. `format` rather than the module's default `check`:
  # linting stays with `yarn lint`, so a formatter run can never rewrite code
  # on a lint rule, while both read the SAME configuration (./biome-config.nix,
  # see there for how biome.json is generated from it).
  programs.biome = {
    enable = true;
    formatCommand = "format";
    settings = import ./biome-config.nix {inherit pkgs;};
    # treefmt-nix maps unknown biome versions to a 2.1.2 schema, which would
    # validate this config against a biome that is not the one running it. Pin
    # the schema to the version in the devenv, which is the whole point.
    validate.schema = pkgs.fetchurl {
      url = "https://biomejs.dev/schemas/${pkgs.biome.version}/schema.json";
      hash = "sha256-YOE0KFzECyivuh3KBmeI1qBcOOx470vQjQBf/Pi9CKw=";
    };
  };

  # Rust and TOML, both of which arrived with dubplate/app. The comment that used
  # to stand here said to add taplo with the first Cargo manifest; this is it.
  programs.rustfmt.enable = true; # dubplate/app/src
  programs.taplo.enable = true; # dubplate/app/Cargo.toml, Trunk.toml

  settings.global.excludes = [
    # Written by their own tools; reformatting them produces spurious diffs on
    # the next `yarn install` or `devenv update`.
    "yarn.lock"
    "devenv.lock"
    # Build output, and the generated shell scripts under .devenv, which shfmt
    # refuses to parse (they are not hand-written shell and nobody reads them).
    ".devenv/**"
    ".direnv/**"
    "cdk.out/**"
    "node_modules/**"
    # The application's build output and the two generated files inside its
    # tree: the stylesheet Tailwind writes on every build, and the analysis
    # module wasm-pack writes into public/.
    "dubplate/app/dist/**"
    "target/**"
    "dubplate/app/public/dubplate/**"
    "dubplate/app/style/generated.css"
  ];
}
