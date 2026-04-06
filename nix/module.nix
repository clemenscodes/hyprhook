self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.hyprhook;

  windowRule = lib.types.submodule {
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
        type = lib.types.listOf lib.types.str;
        default = [];
        example = ["obs-cli start-recording"];
        description = "Shell commands to run when a matching window is created.";
      };

      on_close = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [];
        example = ["obs-cli stop-recording"];
        description = "Shell commands to run when a matching window is destroyed.";
      };

      on_focus = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [];
        example = ["hyprctl dispatch submap gaming"];
        description = "Shell commands to run when a matching window gains focus.";
      };

      on_unfocus = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [];
        example = ["hyprctl dispatch submap reset"];
        description = "Shell commands to run when a matching window loses focus.";
      };
    };
  };

  # Strip null fields before serialising so the TOML stays clean.
  serializeRule = rule:
    lib.filterAttrs (_: v: v != null) rule;

  configAttrs = {
    window = map serializeRule cfg.windows;
  };

  configFile = (pkgs.formats.toml {}).generate "hyprhook.toml" configAttrs;

  wrappedPackage = pkgs.writeShellScriptBin "hyprhook" ''
    exec ${cfg.package}/bin/hyprhook --config ${configFile} "$@"
  '';
in {
  options.services.hyprhook = {
    enable = lib.mkEnableOption "hyprhook Hyprland window focus hook runner";

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
        Add this to your PATH and launch it from Hyprland:

          exec-once = hyprhook
      '';
    };

    windows = lib.mkOption {
      type = lib.types.listOf windowRule;
      default = [];
      description = ''
        Window hook rules. Each entry matches a focused window by class and/or
        title (both are regexes, AND-ed) and runs shell commands when that
        window gains or loses focus.
      '';
      example = lib.literalExpression ''
        [
          {
            class    = "gamescope";
            title    = "Counter-Strike 2";
            on_open    = [ "obs-cli start-recording" ];
            on_close   = [ "obs-cli stop-recording" ];
            on_focus   = [ "hyprctl dispatch submap gaming" ];
            on_unfocus = [ "hyprctl dispatch submap reset" ];
          }
        ]
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    services.hyprhook.finalPackage = wrappedPackage;
    environment.systemPackages = [wrappedPackage];

    systemd.user.services.hyprhook = {
      Unit = {
        Description = "hyprhook Hyprland window focus hook runner";
        PartOf = ["graphical-session.target"];
        After = ["graphical-session.target"];
      };
      Service = {
        ExecStart = "${wrappedPackage}/bin/hyprhook";
        Restart = "on-failure";
        PassEnvironment = "PATH";
      };
      Install = {
        WantedBy = ["graphical-session.target"];
      };
    };
  };
}
