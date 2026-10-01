{pkgs, ...}: {
  # No projectRootFile: devenv supplies the root, so there is no filename here to
  # go stale.
  programs.alejandra.enable = true; # devenv.nix, music/nix/default.nix
  programs.shfmt.enable = true; # .envrc

  # biome formats and lints .ts/.json; treefmt hands it this configuration from
  # the store with --config-path, so the settings exist only here. The module
  # runs `biome check --write`, which is the lint pass as well as the formatter.
  # treefmt walks the git index and skips ignored paths, so the config carries
  # no `files` block.
  programs.biome = {
    enable = true;
    settings = {
      formatter = {
        enabled = true;
        indentStyle = "space";
        indentWidth = 2;
        lineWidth = 100;
      };

      linter = {
        enabled = true;
        rules = {
          recommended = true;
          # The CDK and AWS SDK surfaces this code drives return optionals that
          # the call site has already narrowed by construction (Roots, Accounts,
          # an OU's Name), and rewriting those as runtime guards adds branches no
          # deploy can reach.
          style.noNonNullAssertion = "off";
          # dubplate/app/style/main.css opens with Tailwind v3's `@tailwind`
          # directives, which are not CSS at-rules.
          suspicious.noUnknownAtRules = {
            level = "error";
            options.ignore = ["tailwind"];
          };
        };
      };

      javascript.formatter = {
        quoteStyle = "single";
        trailingCommas = "all";
      };
    };
    # treefmt-nix maps biome versions it does not know to a 2.1.2 schema. The
    # schema in biome's own source tree always matches the binary that runs.
    validate.schema = "${pkgs.biome.src}/packages/@biomejs/biome/configuration_schema.json";
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
