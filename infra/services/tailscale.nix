{pkgs, ...}: {
  services.tailscale.enable = true;

  systemd.services.tailscale-funnel = {
    description = "Tailscale Funnel";
    after = ["tailscaled.service" "nginx.service"];
    requires = ["tailscaled.service"];
    wantedBy = ["multi-user.target"];
    serviceConfig = {
      Type = "simple";
      ExecStartPre = "${pkgs.tailscale}/bin/tailscale funnel --https=443 off || true";
      ExecStart = "${pkgs.tailscale}/bin/tailscale funnel --bg=false 80";
      ExecStop = "${pkgs.tailscale}/bin/tailscale funnel --https=443 off";
      Restart = "on-failure";
      RestartSec = "10s";
    };
  };
}
