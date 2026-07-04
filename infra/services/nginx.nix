{
  config,
  pkgs,
  lib,
  ...
}: let
  domain = "tafl.online";
in {
  services.nginx = {
    enable = true;
    recommendedProxySettings = true;
    virtualHosts."${domain}" = {
      locations."/" = {
        proxyPass = "http://127.0.0.1:8080";
        proxyWebsockets = true;
      };
    };
    virtualHosts."www.${domain}" = {
      globalRedirect = domain;
    };
  };
  networking.firewall.allowedTCPPorts = [80];
}
