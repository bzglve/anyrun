{ testers
, home-anyrun
, packages
, plugins
, home-manager
, ...
}:
testers.runNixOSTest {
  name = "anyrun";
  extraPythonPackages = p: with p; [ pillow colorthief ];
  nodes = {
    machine =
      { pkgs, ... }:
      {
        imports = [ home-manager ];
        virtualisation.qemu.options = [ "-vga virtio" ];

        services.cage = {
          enable = true;
          user = "main";
          program = packages.anyrun + "/bin/anyrun";
        };

        users.users.main = {
          isNormalUser = true;
          home = "/home/main";
        };

        home-manager = {
          sharedModules = [ home-anyrun ];
          users.main = {
            home = {
              stateVersion = "24.11";
              packages = with pkgs; [ firefox ];
            };
            programs.anyrun = {
              enable = true;
              package = packages.anyrun;
              config = {
                margin = [
                  { absolute = 100; }
                  { fraction = 0.5; }
                ];
                edges = [ "Bottom" ];
                hidePluginIcons = true;
                bottomEntry = true;
                saveEntryState = true;
                stealFocus = true;
                ignoreExclusiveZones = true;
                width.absolute = 500;
                height.fraction = 0.5;
                plugins = builtins.map (name: packages.${name}) plugins;
              };
            };
          };
        };
      };
  };

  testScript = ''
    from colorthief import ColorThief # type: ignore
    machine.wait_for_unit("cage-tty1")
    machine.sleep(10)
    machine.screenshot("anyrun")
    machine.copy_from_vm("/home/main/.config/anyrun/", ".")
    color = ColorThief(f"{machine.out_dir}/anyrun.png").get_color(quality=1)
    print(color)
    assert (234, 232, 231) == color
  '';
}
