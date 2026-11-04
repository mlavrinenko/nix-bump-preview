{
  description = "Beta fixture flake for nix-bump-preview end-to-end tests";

  inputs = {
    beta-cache.url = "github:example/beta-cache";
    beta-pinned.url = "github:example/beta-pinned?ref=pinned";
  };

  outputs =
    { beta-cache, beta-pinned }:
    {
      packages.example-beta = beta-cache;
      devShells.example-beta = beta-pinned;
    };
}
