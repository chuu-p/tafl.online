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
    authentication = ''
      local all all scram-sha-256
      host  all all 127.0.0.1/32 scram-sha-256
    '';
    # ponytail: password set via initialScript, firewall + localhost binding protect it
    initialScript = pkgs.writeText "init.sql" ''
      ALTER USER toph WITH PASSWORD 'tafl-top-password-change-me';
    '';
  };

  services.postgresqlBackup = {
    enable = true;
    databases = ["toph"];
    location = "/run/media/at-1/postgres-backup";
    startAt = "*-*-* 01:15:00";
  };
}
