{
  description = "Zsh abbreviation expansion, snippet insertion, and custom fuzzy completion configured in YAML";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAllSystems =
        f: nixpkgs.lib.genAttrs supportedSystems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          zrs = pkgs.callPackage ./nix/package.nix { };
        in
        {
          inherit zrs;
          default = zrs;
        }
      );

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = nixpkgs.lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.zrs;
          inherit (self.packages.${pkgs.stdenv.hostPlatform.system}.zrs) meta;
        };
      });

      overlays.default = final: _prev: {
        zrs = final.callPackage ./nix/package.nix { };
      };

      homeManagerModules.default = import ./nix/hm-module.nix self;

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
            zsh
            fzf
            hyperfine
            just
          ];

          env = {
            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          };
        };
      });

      checks = forAllSystems (
        pkgs:
        let
          zrs = self.packages.${pkgs.stdenv.hostPlatform.system}.zrs;
        in
        {
          package = zrs;

          clippy = zrs.overrideAttrs (old: {
            pname = "${old.pname}-clippy";
            nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ pkgs.clippy ];
            buildPhase = ''
              runHook preBuild
              cargo clippy --offline --all-targets -- -D warnings
              runHook postBuild
            '';
            doCheck = false;
            installPhase = ''
              touch "$out"
            '';
            fixupPhase = "";
          });

          fmt = pkgs.runCommand "zrs-fmt" { nativeBuildInputs = [ pkgs.cargo pkgs.rustfmt ]; } ''
            cd ${zrs.src}
            cargo fmt --check
            touch "$out"
          '';

          zsh-syntax = pkgs.runCommand "zrs-zsh-syntax" { nativeBuildInputs = [ pkgs.zsh ]; } ''
            zsh -n ${./shell/zrs.zsh}
            touch "$out"
          '';

          zsh-e2e =
            pkgs.runCommand "zrs-zsh-e2e"
              {
                nativeBuildInputs = [ pkgs.zsh ];
                env = pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
                  LOCALE_ARCHIVE = "${pkgs.glibcLocales}/lib/locale/locale-archive";
                };
              }
              ''
                zsh ${./tests/zsh/run.zsh} ${nixpkgs.lib.getExe zrs}
                touch "$out"
              '';
        }
      );
    };
}
