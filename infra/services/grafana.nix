{
  config,
  pkgs,
  ...
}: {
  sops.secrets."grafana-smtp-password" = {
    owner = "grafana";
    group = "grafana";
    mode = "0400";
  };

  services.grafana = {
    enable = true;
    settings = {
      panels.disable_sanitize_html = true; # ponytail: user needs this for custom dashboard panels
      server = {
        http_addr = "0.0.0.0"; # ponytail: firewall restricts to trusted networks
        http_port = 3000;
      };
      smtp = {
        enabled = true;
        host = "127.0.0.1:587";
        user = "alerts@chuu.dev";
        password = "$__file{${config.sops.secrets."grafana-smtp-password".path}}";
        from_address = "alerts@chuu.dev";
      };
    };
    provision = {
      enable = true;
      datasources.settings.datasources = [
        {
          name = "Prometheus";
          type = "prometheus";
          access = "proxy";
          url = "http://127.0.0.1:${toString config.services.prometheus.port}";
        }
        {
          name = "Loki";
          type = "loki";
          access = "proxy";
          url = "http://127.0.0.1:3100";
        }
        {
          name = "Tempo";
          type = "tempo";
          access = "proxy";
          url = "http://127.0.0.1:3200";
          jsonData.httpMethod = "GET";
        }
      ];
      dashboards.settings.providers = [
        {
          name = "default";
          options.path = ../toph/dashboards;
        }
      ];
    };
  };
}
