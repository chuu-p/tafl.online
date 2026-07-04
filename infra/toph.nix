{
  inputs,
  config,
  pkgs,
  nixos-hardware,
  ...
}: {
  imports = [
    ./base.nix
    nixos-hardware.nixosModules.raspberry-pi-5
    ./services/openssh.nix
    ./services/postgresql.nix
    ./services/prometheus.nix
    ./services/loki.nix
    ./services/tempo.nix
    ./services/grafana.nix
    ./services/maddy.nix
    ./services/tailscale.nix
    ./services/tafl-online.nix
    ./services/nginx.nix
    ./services/pgadmin.nix
  ];

  boot.kernelPackages = pkgs.linuxPackages_rpi4;

  networking.hostName = "toph";

  boot.kernelParams = [
    "consoleblank=60"
    "cgroup_enable=cpuset"
    "cgroup_memory=1"
    "cgroup_enable=memory"
    "swapaccount=1"
  ];

  fileSystems."/boot/firmware" = {
    device = "/dev/disk/by-uuid/2175-794E";
    fsType = "vfat";
    options = ["noatime" "noauto" "x-systemd.automount" "x-systemd.idle-timeout=1min"];
  };

  fileSystems."/" = {
    device = "/dev/disk/by-uuid/44444444-4444-4444-8888-888888888888";
    fsType = "ext4";
    options = ["noatime"];
  };

  fileSystems."/run/media/at-1" = {
    device = "/dev/disk/by-uuid/3c608d2e-3507-43a1-9dc2-332a95c3d2e2";
    fsType = "btrfs";
    options = ["users" "nofail"];
  };
}
