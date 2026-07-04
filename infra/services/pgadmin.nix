{pkgs, ...}: {
  services.pgadmin = {
    enable = true;
    openFirewall = false;
    settings = {
      DEFAULT_SERVER = "127.0.0.1";
    };
    initialEmail = "artemis@chuu.dev";
    initialPasswordFile = pkgs.writeText "pgadmin-password" "changeme"; # ponytail: firewall + localhost protect this, change on first login
  };
}
