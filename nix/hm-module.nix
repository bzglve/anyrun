packages:
{ config
, lib
, pkgs
, ...
}:
let
  cfg = config.programs.anyrun;

  inherit (builtins)
    map
    toJSON
    toString
    substring
    stringLength
    ;
  inherit (lib.modules) mkIf mkMerge;
  inherit (lib.options) mkOption mkEnableOption;
  inherit (lib.lists) optional;
  inherit (lib.attrsets) mapAttrs' nameValuePair;
  inherit (lib.strings) toLower toUpper replaceStrings;
  inherit (lib.trivial) boolToString;
  inherit (lib.types)
    nullOr
    package
    submodule
    int
    float
    listOf
    either
    str
    enum
    lines
    bool
    attrs
    ;
in
{
  options.programs.anyrun = {
    enable = mkEnableOption "anyrun";
    package = lib.mkPackageOption packages.${pkgs.system} "anyrun" { };

    config =
      let
        numericOptionType = submodule {
          options = {
            absolute = mkOption {
              type = nullOr int;
              default = null;
            };
            fraction = mkOption {
              type = nullOr float;
              default = null;
            };
          };
        };
        mkNumericOption =
          { default, description, ... }:
          mkOption {
            inherit default description;
            example = ''
              { absolute = 200; };
              or
              { fraction = 0.4; };
            '';
            type = numericOptionType;
          };

        numericInfo = ''
          This is a numeric option - pass either `{ absolute = int; };` or `{ fraction = float; };`.
          when using `absolute` it sets the absolute value in pixels,
          when using `fraction`, it sets a fraction of the width or height of the full screen (depends on exclusive zones and the settings related to them) window
        '';
      in
      {
        plugins = mkOption {
          type = nullOr (listOf (either package str));
          default = null;
          description = ''
            List of anyrun plugins to use. Can either be packages, absolute plugin paths, or strings.
          '';
        };

        margin = mkOption {
          default = [ ];
          example = [
            {
              absolute = 10;
              fraction = 0.1;
            }
          ];
          description = ''
            The margin between the runner and the screen edges.

            ${numericInfo}
          '';
          type = listOf numericOptionType;
        };

        edges = mkOption {
          default = [ "Top" ];
          type = listOf (enum [
            "Top"
            "Bottom"
            "Left"
            "Right"
          ]);
          description = ''
            Array of edges where to anchor window. Window will be stretched if two opposite edges specifyed.
          '';
        };

        width = mkNumericOption {
          default.absolute = 800;
          description = ''
            The width of the runner.

            ${numericInfo}
          '';
        };

        height = mkNumericOption {
          default.absolute = 0;
          description = ''
            The minimum height of the runner, the runner will expand to fit all the entries.

            ${numericInfo}
          '';
        };

        stealFocus = mkOption {
          type = bool;
          default = false;
          description = ''
            If `true` will not let you operate with other windows while Anyrun opened
          '';
        };

        saveEntryState = mkOption {
          type = bool;
          default = false;
          description = ''
            Save entred text on close and have it on open.
          '';
        };

        bottomEntry = mkOption {
          type = bool;
          default = false;
          description = ''
            Place entry to the bottom.
          '';
        };

        hideMatchIcons = mkOption {
          type = bool;
          default = false;
          description = "Hide match icons";
        };

        hidePluginIcons = mkOption {
          type = bool;
          default = false;
          description = "Hide plugin info icons";
        };

        ignoreExclusiveZones = mkOption {
          type = bool;
          default = false;
          description = "ignore exclusive zones, eg. Waybar";
        };

        layer = mkOption {
          type = enum [
            "background"
            "bottom"
            "top"
            "overlay"
          ];
          default = "overlay";
          description = "Layer shell layer (background, bottom, top or overlay)";
        };

        hidePluginInfo = mkOption {
          type = bool;
          default = false;
          description = "Hide the plugin info panel";
        };

        closeOnClick = mkOption {
          type = bool;
          default = false;
          description = "Close window when a click outside the main box is received";
        };

        showResultsImmediately = mkOption {
          type = bool;
          default = false;
          description = "Show search results immediately when Anyrun starts";
        };
      };

    extraCss = mkOption {
      type = nullOr lines;
      default = "";
      description = ''
        Extra CSS lines to add to {file}`~/.config/anyrun/style.css`.
      '';
    };

    extraConfigFiles = mkOption {
      # unfortunately HM doesn't really export the type for files, but hopefully
      # hm will throw errors if the options are wrong here, so I'm being *very* loose
      type = attrs;
      default = { };
      description = ''
        Extra files to put in {file}`~/.config/anyrun`, a wrapper over {option}`xdg.configFile`.
      '';
      example = ''
        programs.anyrun.extraConfigFiles."plugin-name.ron".text = '''
          Config(
            some_option: true,
          )
        '''
      '';
    };
  };

  config = mkIf cfg.enable (
    let
      assertNumeric = numeric: {
        assertion =
          !(
            (numeric ? absolute && numeric.absolute != null) && (numeric ? fraction && numeric.fraction != null)
          );
        message = "Invalid numeric definition, you can only specify one of absolute or fraction.";
      };

      stringifyNumeric =
        numeric:
        if (numeric ? absolute && numeric.absolute != null) then
          "Absolute(${toString numeric.absolute})"
        else
          "Fraction(${toString numeric.fraction})";

      capitalize =
        string: toUpper (substring 0 1 string) + toLower (substring 1 ((stringLength string) - 1) string);

      parsedPlugins =
        if cfg.config.plugins == null then
          [ ]
        else
          map
            (
              entry:
              if lib.types.package.check entry then
                "${entry}/lib/lib${replaceStrings [ "-" ] [ "_" ] entry.pname}.so"
              else
                entry
            )
            cfg.config.plugins;
    in
    {
      assertions = [
        (assertNumeric cfg.config.width)
        (assertNumeric cfg.config.height)
      ] ++ (map (m: assertNumeric m) cfg.config.margin);

      warnings =
        if cfg.config.plugins == null then
          [
            ''
              You haven't enabled any plugins. Anyrun will not show any results, unless you specify plugins with the --override-plugins flag.
              Add plugins to programs.anyrun.config.plugins, or set it to [] to silence the warning.
            ''
          ]
        else
          [ ];

      home.packages = optional (cfg.package != null) cfg.package;

      xdg.configFile = mkMerge [
        (mapAttrs' (name: value: nameValuePair ("anyrun/" + name) value) cfg.extraConfigFiles)

        {
          "anyrun/config.ron".text = ''
            Config(
              margin: [${lib.concatMapStringsSep "," (numeric: stringifyNumeric numeric) cfg.config.margin}],
              edges: [${builtins.concatStringsSep "," cfg.config.edges}],
              width: ${stringifyNumeric cfg.config.width},
              height: ${stringifyNumeric cfg.config.height},
              hide_plugin_icons: ${boolToString cfg.config.hidePluginIcons},
              hide_match_icons: ${boolToString cfg.config.hideMatchIcons},
              ignore_exclusive_zones: ${boolToString cfg.config.ignoreExclusiveZones},
              layer: ${capitalize cfg.config.layer},
              hide_plugin_info: ${boolToString cfg.config.hidePluginInfo},
              show_results_immediately: ${boolToString cfg.config.showResultsImmediately},
              steal_focus: ${boolToString cfg.config.stealFocus},
              save_entry_state: ${boolToString cfg.config.saveEntryState},
              bottom_entry: ${boolToString cfg.config.bottomEntry},
              plugins: ${toJSON parsedPlugins},
            )
          '';
        }

        { "anyrun/style.css" = mkIf (cfg.extraCss != null) { text = cfg.extraCss; }; }
      ];
    }
  );
}
