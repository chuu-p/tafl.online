{
  pkgs,
  ...
}: {
  services.cloudflared = {
    enable = true;
    tunnels = {
      "tafl-tunnel" = {
        credentialsFile = "/var/lib/cloudflared/credentials.json";
        ingress = {
          "tafl.online" = "http://localhost:80";
          "www.tafl.online" = "http://localhost:80";
        };
        default = "http_status:404";
      };
    };
  };
}
