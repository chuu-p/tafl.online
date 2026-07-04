{
  config,
  pkgs,
  ...
}: {
  sops.secrets."maddy-password" = {
    owner = "root";
    group = "root";
    mode = "0400";
  };

  services.maddy = {
    enable = true;
    hostname = "mail.chuu.dev";
    primaryDomain = "chuu.dev";
    ensureAccounts = ["alerts@chuu.dev"];
    ensureCredentials."alerts@chuu.dev".passwordFile = config.sops.secrets."maddy-password".path;
  };
}
