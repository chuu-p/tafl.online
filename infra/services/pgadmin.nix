{ pkgs, ... }: {
  services.pgadmin = {
    enable = true;
    openFirewall = true;
    settings = {
      DEFAULT_SERVER = "0.0.0.0";
    };
    initialEmail = "artemis@chuu.dev";
    initialPasswordFile = pkgs.writeText "pgadmin-password" "YourSecurePassword123"; # fixme:security this password should be injected with sops-nix
  };
}
