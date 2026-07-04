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
      "94125df0-b4d2-49bc-b0d3-f804465edce7" = {
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
