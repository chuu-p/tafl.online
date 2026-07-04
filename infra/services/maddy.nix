{pkgs, ...}: {
  services.maddy = {
    enable = true;
    hostname = "mail.chuu.dev";
    primaryDomain = "chuu.dev";
    ensureAccounts = ["alerts@chuu.dev"];
    ensureCredentials."alerts@chuu.dev".passwordFile =
      pkgs.writeText "alerts-password" "test"; # fixme:security hardcoded password, use sops-nix
  };
}
