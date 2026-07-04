{
  config,
  pkgs,
  ...
}: {
  services.grafana = {
    enable = true;
    settings = {
      panels.disable_sanitize_html = true;
      server = {
        http_addr = "0.0.0.0"; # fixme:security exposed on all interfaces, bind to 127.0.0.1 if proxied via nginx
        http_port = 3000;
      };
      smtp = {
        enabled = true;
        host = "127.0.0.1:587";
        user = "alerts@chuu.dev";
        password = "test"; # fixme:security hardcoded password, use sops-nix
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
