{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    utils.url = "github:numtide/flake-utils";
  };
  outputs =
    { ... }@inputs:
    with inputs;
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      rec {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
          ];
        };
        packages.default = pkgs.callPackage ./default.nix { };
        apps.default = utils.lib.mkApp { drv = packages.default; };
      }
    );
}
