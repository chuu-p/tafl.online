{
  config,
  pkgs,
  ...
}: {
  services.prometheus = {
    enable = true;
    port = 9090;
    exporters = {
      node = {
        enable = true;
        enabledCollectors = ["systemd"];
        port = 9002;
      };
      blackbox = {
        enable = true;
        listenAddress = "127.0.0.1";
        port = 9115;
        configFile = pkgs.writeText "blackbox-config.yaml" (builtins.toJSON {
          modules = {
            http_2xx = {
              prober = "http";
              timeout = "5s";
              http = {
                valid_status_codes = [200 201 202 204];
                method = "GET";
                fail_if_ssl = false;
                fail_if_not_ssl = false;
              };
            };
          };
        });
      };
    };
    scrapeConfigs = [
      {
        job_name = "api_uptime_monitors";
        metrics_path = "/probe";
        params.module = ["http_2xx"];
        static_configs = [
          {
            targets = [
              "https://api.github.com"
              "https://httpbin.org"
              "http://127.0.0.1:3000"
            ];
          }
        ];
        relabel_configs = [
          {
            source_labels = ["__address__"];
            target_label = "__param_target";
          }
          {
            source_labels = ["__param_target"];
            target_label = "instance";
          }
          {
            target_label = "__address__";
            replacement = "127.0.0.1:9115";
          }
        ];
      }
      {
        job_name = "chrysalis";
        static_configs = [
          {
            targets = ["127.0.0.1:${toString config.services.prometheus.exporters.node.port}"];
          }
        ];
      }
    ];
  };
}
