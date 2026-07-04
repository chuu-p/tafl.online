{
  config,
  pkgs,
  lib,
  ...
}: let
  domain = "tafl.online";
in {
  security.acme.acceptTerms = true;
  security.acme.defaults.email = "artemis@chuu.dev";

  services.nginx = {
    enable = true;
    recommendedProxySettings = true;
    virtualHosts."${domain}" = {
      enableACME = true;
      forceSSL = true;
      locations."/" = {
        proxyPass = "http://127.0.0.1:8080";
        proxyWebsockets = true;
      };
    };
    virtualHosts."www.${domain}" = {
      enableACME = true;
      forceSSL = true;
      globalRedirect = domain;
    };
  };
  networking.firewall.allowedTCPPorts = [80 443];
}
