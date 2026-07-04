{
  description = "tafl.online nixos declarative infra";

  nixConfig = {
    extra-substituters = [
      "https://nixos-raspberrypi.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nixos-raspberrypi.cachix.org-1:4iMO9LXa8BqhU+Rpg6LQKiGa2lsNh/j2oiYLNOQ5sPI="
    ];
  };

  inputs = {
    tafl-online.url = "github:chuu-p/tafl.online?ref=opentafl";
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    nixos-hardware.url = "github:NixOS/nixos-hardware/master";
    deploy-rs = {
      url = "github:serokell/deploy-rs";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    home-manager = {
      url = "github:nix-community/home-manager?ref=release-25.11";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixos-raspberrypi = {
      url = "github:nvmd/nixos-raspberrypi";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    deploy-rs,
    home-manager,
    nixos-hardware,
    sops-nix,
    nixos-raspberrypi,
    rust-overlay,
    ...
  } @ inputs: let
    lib = nixpkgs.lib;
    hosts = {
      toph = {
        system = "aarch64-linux";
        hostname = "toph";
        user = "root";
        modules = [./toph.nix];
      };
    };
  in {
    nixosConfigurations =
      lib.mapAttrs
      (name: cfg:
        lib.nixosSystem {
          system = cfg.system;
          specialArgs = inputs // {inherit inputs;};
          modules =
            cfg.modules
            ++ [
              {
                nixpkgs.overlays = [
                  (import inputs.rust-overlay)
                  (final: prev: {
                    unstable = import inputs.nixpkgs-unstable {
                      system = prev.system;
                      config.allowUnfree = true;
                    };
                  })
                ];
              }
            ];
        })
      hosts;

    deploy.nodes =
      lib.mapAttrs
      (name: cfg: {
        hostname = cfg.hostname;

        profiles.system = {
          user = cfg.user;

          path =
            deploy-rs.lib.${cfg.system}.activate.nixos
            self.nixosConfigurations.${name};
        };
      })
      hosts;

    checks =
      builtins.mapAttrs
      (system: deployLib: deployLib.deployChecks self.deploy)
      deploy-rs.lib;

    installerImages.rpi02-wifi =
      self.nixosConfigurations.rpi02-wifi.config.system.build.sdImage;

    packages = nixpkgs.lib.genAttrs ["aarch64-linux" "x86_64-linux"] (system: {});

    apps = nixpkgs.lib.genAttrs ["aarch64-linux" "x86_64-linux"] (system: {
      deploy = {
        type = "app";
        program = "${deploy-rs.packages.${system}.deploy-rs}/bin/deploy";
      };
    });
  };
}
