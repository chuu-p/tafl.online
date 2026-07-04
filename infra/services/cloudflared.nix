{
  config,
  pkgs,
  ...
}: {
  sops.defaultSopsFile = ../secrets/secrets.yaml;
  sops.secrets."cloudflared-creds" = {
    owner = "root";
    group = "root";
    mode = "0400";
  };

  services.cloudflared = {
    enable = true;
    tunnels = {
      "tafl-tunnel" = {
        credentialsFile = config.sops.secrets."cloudflared-creds".path;
        ingress = {
          "tafl.online" = "http://localhost:80";
          "www.tafl.online" = "http://localhost:80";
        };
        default = "http_status:404";
      };
    };
  };
}
