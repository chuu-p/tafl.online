{
  config,
  pkgs,
  lib,
  ...
}: {
  imports = [
    ./nix.nix
  ];

  # Shared boot setup
  documentation.man.generateCaches = false;

  networking.firewall.enable = false;

  # boot.supportedFilesystems = ["nfs"];

  boot.loader.grub.enable = false;
  boot.loader.generic-extlinux-compatible.enable = true;

  users.extraGroups.docker.members = ["gitea-runner" "chuu"];

  security.pki.certificates = [
    ''
      chuu.dev Root CA
      =================
      -----BEGIN CERTIFICATE-----
      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
      rrBT+1Ky0agHew4=
      -----END CERTIFICATE-----
    ''
  ];

  # Shared user
  users.users.chuu = {
    isNormalUser = true;
    password = "chuu";
    extraGroups = ["networkmanager" "wheel" "docker"];
    shell = pkgs.fish;
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHQHb+VwHnS97Wmu4xpUDlLhzB+Ip11BINatUivsr6+a"
    ];
  };

  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHQHb+VwHnS97Wmu4xpUDlLhzB+Ip11BINatUivsr6+a"
  ];

  security.sudo.extraRules = [
    {
      users = ["chuu"];
      commands = [
        {
          command = "ALL";
          options = ["NOPASSWD"];
        }
      ];
    }
  ];

  nix.settings = {
    trusted-users = ["root" "@wheel" "chuu"];
    experimental-features = ["nix-command" "flakes"];
  };

  nixpkgs.config.allowUnfree = true;

  programs.fish = {
    enable = true;
    promptInit = ''
      function fish_user_key_bindings
          bind -M insert jk "if commandline -P; commandline -f cancel; else; set fish_bind_mode default; commandline -f backward-char force-repaint; end"
      end
      function fish_hybrid_key_bindings
        fish_default_key_bindings -M insert
        fish_vi_key_bindings --no-erase
      end
      set -g fish_key_bindings fish_hybrid_key_bindings
      fish_user_key_bindings
    '';
  };

  programs.ssh.startAgent = true;

  environment = {
    shells = [pkgs.fish];
    systemPackages = with pkgs; [
      alejandra
      bottom
      evil-helix
      fish
      fluxcd
      git
      htop
      jq
      kitty.terminfo
      ncurses
      nodejs
      ranger
      unstable.tailscale
      wget
      yazi
    ];
  };

  services.tailscale = {
    enable = true;
    port = 41641;
    package = pkgs.unstable.tailscale;
  };

  services.openiscsi = {
    enable = true;
    name = "${config.networking.hostName}-initiatorhost";
  };
  systemd.services.iscsid.serviceConfig = {
    PrivateMounts = "yes";
    BindPaths = "/run/current-system/sw/bin:/bin";
  };

  # virtualisation.docker.enable = true;

  # environment.sessionVariables = {
  #   EDITOR = "hx";
  #   PATH = ["$HOME/git/nixos/PATH"];
  # };

  system.stateVersion = "25.05";
}
