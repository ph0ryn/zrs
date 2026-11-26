self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.zrs-zsh;
  yamlFormat = pkgs.formats.yaml { };
in
{
  options.programs.zrs-zsh = {
    enable = lib.mkEnableOption "zrs-zsh";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.zrs-zsh;
      defaultText = lib.literalExpression "inputs.zrs-zsh.packages.\${pkgs.stdenv.hostPlatform.system}.zrs-zsh";
      description = "The zrs-zsh package to install.";
    };

    enableZshIntegration = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to enable Zsh ZLE integration via `eval \"$(zrs-zsh init)\"`.";
    };

    settings = lib.mkOption {
      type = yamlFormat.type;
      default = { };
      example = lib.literalExpression ''
        {
          abbreviations = [
            {
              keyword = "gs";
              expand = "git status --short --branch";
            }
          ];
        }
      '';
      description = ''
        Declarative YAML configuration written to
        `$XDG_CONFIG_HOME/zrs-zsh/config.yaml`.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."zrs-zsh/config.yaml" = lib.mkIf (cfg.settings != { }) {
      source = yamlFormat.generate "zrs-zsh-config.yaml" cfg.settings;
    };

    programs.zsh.initContent = lib.mkIf cfg.enableZshIntegration (
      lib.mkAfter ''
        eval "$(${lib.getExe cfg.package} init)"
      ''
    );
  };
}
