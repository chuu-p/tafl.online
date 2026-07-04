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
      locations."/" = {
        root = "/var/lib/tafl-online/static";
        tryFiles = "$uri $uri/ /index.html";
      };
    };
  };
  networking.firewall.allowedTCPPorts = [80 443];
}
