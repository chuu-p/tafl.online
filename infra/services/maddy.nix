{pkgs, ...}: {
  services.maddy = {
    enable = true;
    hostname = "mail.chuu.dev";
    primaryDomain = "chuu.dev";
    ensureAccounts = ["alerts@chuu.dev"];
    ensureCredentials."alerts@chuu.dev".passwordFile =
      pkgs.writeText "alerts-password" "changeme"; # ponytail: firewall + localhost protect this, migrate to sops later
  };
}
