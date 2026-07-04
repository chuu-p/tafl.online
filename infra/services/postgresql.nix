{
  config,
  pkgs,
  ...
}: {
  services.postgresql = {
    enable = true;
    package = pkgs.postgresql_18;
    enableTCPIP = false;
    dataDir = "/run/media/at-1/tafl-db";
    ensureDatabases = ["toph"];
    ensureUsers = [
      {
        name = "toph";
        ensureDBOwnership = true;
      }
      {
        name = "tafl";
        ensureDBOwnership = false;
      }
    ];
     # fixme:security use scram-sha-256 in production
     # fixme:security restrict to specific users
    authentication = ''
      local all all trust
      host  all all 127.0.0.1/32 trust
    '';
  };

  services.postgresqlBackup = {
    enable = true;
    databases = ["toph"];
    location = "/run/media/at-1/postgres-backup";
    startAt = "*-*-* 01:15:00";
  };
}
