self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.zrs;
  yamlFormat = pkgs.formats.yaml { };
in
{
  options.programs.zrs = {
    enable = lib.mkEnableOption "zrs";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.zrs;
      defaultText = lib.literalExpression "inputs.zrs.packages.\${pkgs.stdenv.hostPlatform.system}.zrs";
      description = "The zrs package to install.";
    };

    enableZshIntegration = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to enable Zsh ZLE integration via `eval \"$(zrs init)\"`.";
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
        `$XDG_CONFIG_HOME/zrs/config.yaml`.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."zrs/config.yaml" = lib.mkIf (cfg.settings != { }) {
      source = yamlFormat.generate "zrs-config.yaml" cfg.settings;
    };

    programs.zsh.initContent = lib.mkIf cfg.enableZshIntegration (
      lib.mkAfter ''
        eval "$(${lib.getExe cfg.package} init)"
      ''
    );
  };
}
