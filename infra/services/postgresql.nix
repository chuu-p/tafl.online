{
  config,
  pkgs,
  ...
}: {
  sops.secrets."postgres-top-password" = {
    owner = "postgres";
    group = "tafl-web";
    mode = "0440";
  };

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
      local all postgres peer
      local all all scram-sha-256
      host  all all 127.0.0.1/32 scram-sha-256
    '';
    # ponytail: initial password set here, overridden by sops at boot
    initialScript = pkgs.writeText "init.sql" ''
      ALTER USER toph WITH PASSWORD 'bootstrap';
    '';
  };

  systemd.services.postgresql-set-password = {
    description = "Set PostgreSQL password from sops secret";
    after = ["postgresql.service" "sops-nix.service"];
    requires = ["postgresql.service"];
    wantedBy = ["multi-user.target"];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${pkgs.writeShellScript "set-pg-password" ''
        PW=$(cat ${config.sops.secrets."postgres-top-password".path})
        echo "ALTER USER toph WITH PASSWORD '$PW';" | ${config.services.postgresql.package}/bin/psql -U postgres
      ''}";
      User = "postgres";
    };
  };

  services.postgresqlBackup = {
    enable = true;
    databases = ["toph"];
    location = "/run/media/at-1/postgres-backup";
    startAt = "*-*-* 01:15:00";
  };
}
