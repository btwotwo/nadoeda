{
  description = "Rust flake";
  inputs =
    {
      nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    };
  
  outputs = { self, nixpkgs, ... }@inputs:
    let
     system = "x86_64-linux";
     pkgs = nixpkgs.legacyPackages.${system};    
    in
    {
      devShells.${system}.default = pkgs.mkShell
      {
        packages = [ pkgs.rustc
                     pkgs.cargo
                     pkgs.rust-analyzer
                     pkgs.rustfmt
                     (pkgs.diesel-cli.override {
                       sqliteSupport = true;
                     })
                   ];
      };
    };
}
