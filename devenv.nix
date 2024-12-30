{ pkgs, lib, config, inputs, ... }:

{
  packages = with pkgs; [ git nodejs_23 keyfinder-cli bun granted ];

  pre-commit.hooks = {
    biome = {
      enable = true;
      entry = "${pkgs.biome}/bin/biome check --apply --no-errors-on-unmatched --diagnostic-level=error";
    };
    prettier = {
      enable = true;
      files = "\\.(gql|ya?ml)$";
    };
  };

  outputs = {
    music = import ./nix/default.nix { inherit pkgs config; };
  };
}
