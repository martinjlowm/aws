# The biome configuration, as data.
#
# `biome.json` at the repo root is GENERATED from this on shell entry (devenv.nix
# enterShell) and is not committed, and treefmt hands biome the same attrset from
# the store. Editing the JSON directly is undone by the next shell entry.
#
# Why generate it at all. The `$schema` line is a version number a human types
# and nothing checks, and this repo had no biome.json at all: the pre-commit hook
# ran `biome check --apply` on nixpkgs defaults, so the formatting the hook
# enforced was whatever the pinned nixpkgs happened to ship and no editor could
# read it. Deriving the schema URL from `pkgs.biome.version` makes the drift
# unrepresentable: bump nixpkgs and the schema, the formatter and the checked-in
# config all move together.
#
# The npm `@biomejs/biome` in package.json is pinned to the SAME version, which
# is what `yarn lint` and every editor's biome LSP run. Keep them equal.
{pkgs}: {
  "$schema" = "https://biomejs.dev/schemas/${pkgs.biome.version}/schema.json";

  # Deliberately NOT `vcs.useIgnoreFile`. Biome resolves the ignore file relative
  # to the CONFIG's directory, so the moment treefmt passes a config out of the
  # Nix store the ignore file is gone and biome starts reading `node_modules`.
  # Enumerating costs a list that has to be maintained against .gitignore, and
  # buys a config that means the same thing wherever it is read from.
  #
  # `.pre-commit-config.yaml` earns its entry: devenv writes it as a SYMLINK
  # into the Nix store pointing at a `.json` file that opens with `#` comments,
  # so biome follows it, tries to parse JSON, and reports parse errors in a file
  # nobody in this repo wrote.
  files.includes = [
    "**"
    "!**/node_modules"
    "!**/cdk.out"
    "!**/.devenv"
    "!**/.direnv"
    "!**/.devenv.flake.nix"
    "!**/devenv.local.nix"
    "!**/.pre-commit-config.yaml"
    "!**/result"
    "!**/result-*"
    # Rewritten by their own tools; reformatting them produces spurious diffs.
    "!**/devenv.lock"
    # cargo's build directory for dubplate/app, whose fingerprint files are JSON
    # and are several hundred of them. Biome has no opinion worth having about a
    # file cargo rewrites on every build.
    "!**/target"
    # Trunk's output and the stylesheet Tailwind generates into the source tree.
    "!**/dist"
    "!**/generated.css"
  ];

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
      # The CDK and AWS SDK surfaces this code drives return optionals that the
      # call site has already narrowed by construction (Roots, Accounts, an OU's
      # Name), and rewriting those as runtime guards adds branches no deploy can
      # reach.
      style.noNonNullAssertion = "off";
    };
  };

  javascript.formatter = {
    quoteStyle = "single";
    trailingCommas = "all";
  };
}
