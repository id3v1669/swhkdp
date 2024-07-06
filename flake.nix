{
  description = "Swhkd";

  inputs = { 
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs = inputs @ 
  { self
  , nixpkgs
  , systems
  , ... 
  }:
  let
    eachSystem = nixpkgs.lib.genAttrs (import systems);

    pkgsFor = (system: import nixpkgs {
      inherit system;
      overlays = [ ];
    });
  in 
  {
    packages = eachSystem (system: {
      default = nixpkgs.legacyPackages.${system}.callPackage ./nix{ };
    });

    defaultPackage = eachSystem (system: self.packages.${system}.default);

    devShells = eachSystem (system:
    let 
      pkgs = pkgsFor system;
    in 
    {
      default = pkgs.mkShell {
        name = "Swhkd-devel";
        nativeBuildInputs = with pkgs; [
          # Compilers
          cargo
          rustc
          scdoc

          # libs
          udev

          # Tools
          pkg-config
          clippy
          gdb
          gnumake
          rust-analyzer
          rustfmt
          strace
          valgrind
          zip
        ];
      };
    });

    nixosModules.default = import ./nix/module.nix inputs;
  };
}