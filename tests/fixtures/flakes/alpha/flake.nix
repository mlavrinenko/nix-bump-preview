{
  description = "Alpha fixture flake for nix-bump-preview end-to-end tests";

  inputs = {
    alpha-cache.url = "github:example/alpha-cache";
    alpha-stable.url = "github:example/alpha-stable?ref=stable";
  };

  outputs =
    { alpha-cache, alpha-stable }:
    {
      packages.example-alpha = alpha-cache;
      devShells.example-alpha = alpha-stable;
    };
}
