{
  pkgs,
  inputs,
  ...
}: let
  taste = inputs.tafl-online.packages.${pkgs.system}.taste;
in {
  systemd.services.taste = {
    description = "tafl.online backend";
    wantedBy = ["multi-user.target"];
    after = ["network.target" "postgresql.service" "media-at-1.mount"];
    requires = ["postgresql.service"];
    serviceConfig = {
      ExecStart = "${taste}/bin/taste";
      Restart = "always";
      RestartSec = 5;
      User = "taste";
      # ponytail: Unix socket, user=toph, db=toph; postgres trusts local connections
      Environment = "DATABASE_URL=postgres:///toph?host=/run/postgresql&user=toph";
      RequiresMountsFor = ["/run/media/at-1"];
    };
  };

  users.users.taste = {
    isSystemUser = true;
    group = "taste";
    home = "/var/lib/taste";
    createHome = true;
  };
  users.groups.taste = {};
}
