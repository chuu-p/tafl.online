{pkgs, ...}: {
  services.loki = {
    enable = true;
    configFile = ../toph/loki-local-config.yaml;
  };

  systemd.services.promtail = {
    description = "Promtail service for Loki";
    wantedBy = ["multi-user.target"];
    serviceConfig = {
      ExecStart = ''
        ${pkgs.grafana-loki}/bin/promtail --config.file ${../toph/promtail.yaml}
      '';
    };
  };
}
