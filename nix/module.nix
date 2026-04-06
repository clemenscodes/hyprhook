self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.hyprhook;

  commandType = lib.types.listOf lib.types.str;

  ruleType = lib.types.submodule {
    options = {
      class = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "gamescope";
        description = ''
          Regex matched against the window class.
          Omit to match any class.
        '';
      };

      title = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "Counter-Strike 2";
        description = ''
          Regex matched against the window title.
          Omit to match any title. AND-ed with class.
        '';
      };

      on_open = lib.mkOption {
        type = lib.types.nullOr commandType;
        default = null;
        example = ["obs-cli" "start-recording"];
        description = "Command to run when a matching window is created. First element is the binary, the rest are args.";
      };

      on_close = lib.mkOption {
        type = lib.types.nullOr commandType;
        default = null;
        example = ["obs-cli" "stop-recording"];
        description = "Command to run when a matching window is destroyed. First element is the binary, the rest are args.";
      };

      on_focus = lib.mkOption {
        type = lib.types.nullOr commandType;
        default = null;
        example = ["hyprctl" "dispatch" "submap" "gaming"];
        description = "Command to run when a matching window gains focus. First element is the binary, the rest are args.";
      };

      on_unfocus = lib.mkOption {
        type = lib.types.nullOr commandType;
        default = null;
        example = ["hyprctl" "dispatch" "submap" "reset"];
        description = "Command to run when a matching window loses focus. First element is the binary, the rest are args.";
      };
    };
  };

  # Strip null and empty-list fields so the TOML stays clean.
  serializeRule = rule:
    lib.filterAttrs (_: v: v != null && v != []) {
      inherit (rule) class title on_open on_close on_focus on_unfocus;
    };

  configAttrs = {
    rule = map serializeRule cfg.rules;
  };

  configFile = (pkgs.formats.toml {}).generate "hyprhook.toml" configAttrs;

  wrappedPackage = pkgs.symlinkJoin {
    name = "hyprhook";
    paths = [cfg.package];
    nativeBuildInputs = [pkgs.makeWrapper];
    postBuild = ''
      wrapProgram $out/bin/hyprhook \
        --add-flags "--config ${configFile}"
    '';
  };
in {
  options.services.hyprhook = {
    enable = lib.mkEnableOption "hyprhook Hyprland window lifecycle hook runner";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.hyprhook;
      defaultText = lib.literalExpression "inputs.hyprhook.packages.\${system}.hyprhook";
      description = "The hyprhook package to use.";
    };

    finalPackage = lib.mkOption {
      type = lib.types.package;
      readOnly = true;
      description = ''
        The hyprhook binary pre-configured with the generated TOML config.
        Launch from Hyprland:

          exec-once = ''${config.services.hyprhook.finalPackage}/bin/hyprhook
      '';
    };

    rules = lib.mkOption {
      type = lib.types.listOf ruleType;
      default = [];
      description = ''
        Window hook rules. Each entry matches windows by class and/or title
        (both are regexes, AND-ed) and runs commands on lifecycle events.
        Each command specifies a binary and optional args separately.
      '';
      example = [
        {
          class      = "gamescope";
          title      = "Counter-Strike 2";
          on_focus   = ["cs2-mode-start"];
          on_unfocus = ["cs2-mode-stop"];
          on_open    = ["obs-cli" "start-recording"];
          on_close   = ["obs-cli" "stop-recording"];
        }
      ];
    };
  };

  config = lib.mkIf cfg.enable {
    services.hyprhook.finalPackage = wrappedPackage;
    environment.systemPackages = [wrappedPackage];
  };
}
