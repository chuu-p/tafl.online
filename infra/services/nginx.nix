{
  config,
  pkgs,
  lib,
  ...
}: let
  domain = "tafl.chuu.dev";
in {
  services.nginx = {
    enable = true;
    recommendedProxySettings = true;
    virtualHosts."${domain}" = {
      enableACME = lib.mkForce false; # fixme:security TLS disabled, enable ACME for production
      forceSSL = false; # fixme:security HTTP only, enable for production
      locations."/tafl.v1/" = {
        proxyPass = "http://127.0.0.1:50051";
        proxyWebsockets = true;
        extraConfig = ''
          proxy_set_header Upgrade $http_upgrade;
          proxy_set_header Connection "upgrade";
        '';
      };
      locations."/" = {
        root = "/var/lib/tafl-online/static";
        tryFiles = "$uri $uri/ /index.html";
      };
    };
  };
  networking.firewall.allowedTCPPorts = [80 443];
}
