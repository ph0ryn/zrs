{
  description = "A blazing-fast, on-demand Zsh tool for abbreviations, snippets, history search, and custom fuzzy completion";

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
          zrs-zsh = pkgs.callPackage ./nix/package.nix { };
        in
        {
          inherit zrs-zsh;
          default = zrs-zsh;
        }
      );

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = nixpkgs.lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.zrs-zsh;
          inherit (self.packages.${pkgs.stdenv.hostPlatform.system}.zrs-zsh) meta;
        };
      });

      overlays.default = final: _prev: {
        zrs-zsh = final.callPackage ./nix/package.nix { };
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
          zrs-zsh = self.packages.${pkgs.stdenv.hostPlatform.system}.zrs-zsh;
        in
        {
          package = zrs-zsh;

          clippy = zrs-zsh.overrideAttrs (old: {
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

          fmt = pkgs.runCommand "zrs-zsh-fmt" { nativeBuildInputs = [ pkgs.cargo pkgs.rustfmt ]; } ''
            cd ${zrs-zsh.src}
            cargo fmt --check
            touch "$out"
          '';

          zsh-syntax = pkgs.runCommand "zrs-zsh-zsh-syntax" { nativeBuildInputs = [ pkgs.zsh ]; } ''
            zsh -n ${./shell/zrs.zsh}
            touch "$out"
          '';

          zsh-e2e =
            pkgs.runCommand "zrs-zsh-zsh-e2e"
              {
                nativeBuildInputs = [ pkgs.zsh pkgs.fzf ];
                env = pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
                  LOCALE_ARCHIVE = "${pkgs.glibcLocales}/lib/locale/locale-archive";
                };
              }
              ''
                zsh -f ${./tests/zsh/run.zsh} ${nixpkgs.lib.getExe zrs-zsh}
                touch "$out"
              '';
        }
      );
    };
}
