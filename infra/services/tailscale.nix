{pkgs, ...}: {
  services.tailscale.enable = true;

  systemd.services.tailscale-funnel = {
    description = "Tailscale Funnel";
    after = ["tailscaled.service" "nginx.service"];
    requires = ["tailscaled.service"];
    wantedBy = ["multi-user.target"];
    serviceConfig = {
      Type = "simple";
      ExecStartPre = "${pkgs.bash}/bin/bash -c '${pkgs.tailscale}/bin/tailscale funnel reset || true'";
      ExecStart = "${pkgs.tailscale}/bin/tailscale funnel --bg=false 80";
      ExecStop = "${pkgs.bash}/bin/bash -c '${pkgs.tailscale}/bin/tailscale funnel reset'";
      Restart = "on-failure";
      RestartSec = "10s";
    };
  };
}
