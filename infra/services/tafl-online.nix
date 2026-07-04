{
  pkgs,
  inputs,
  ...
}: let
  src = "${inputs.tafl-online}";
  rust = pkgs.rust-bin.stable.latest.default.override {targets = ["wasm32-unknown-unknown"];};
  dx = pkgs.unstable.dioxus-cli; # ponytail: pkgs is 0.7.6, incompatible with dioxus crate 0.7.9
  gcc = pkgs.gcc;
  bash = pkgs.bash;
  binaryen = pkgs.binaryen;

  # ponytail: nixpkgs ships 0.2.108, dx serve needs 0.2.126.
  wasm-bindgen-cli = pkgs.rustPlatform.buildRustPackage rec {
    pname = "wasm-bindgen-cli";
    version = "0.2.126";
    src = pkgs.fetchCrate {
      inherit pname version;
      hash = "sha256-H6Is3fiZVxZCfOMWK5dWMSrtn50VGv0sfdnsT+cTtyk=";
    };
    cargoHash = "sha256-VucqkXbCi4qtQzY/HrXiDnbSURsagPsdNVMn1Tw3UiY=";
    doCheck = false;
  };
  staticDir = "/var/lib/tafl-online/static";
  stateDir = "/var/lib/tafl-online";

  buildScript = pkgs.writeShellScript "tafl-online-build" ''
    set -e
    export HOME=${stateDir}/.home
    export PATH=${rust}/bin:${dx}/bin:${wasm-bindgen-cli}/bin:${binaryen}/bin:${gcc}/bin:$PATH
    export CARGO_HOME=${stateDir}/.cargo
    export CARGO_TARGET_DIR=${stateDir}/target

    mkdir -p ${stateDir}/.cargo ${stateDir}/.home ${stateDir}/target

    rm -rf /tmp/tafl-online-build
    cp -r ${src} /tmp/tafl-online-build
    chmod -R u+w /tmp/tafl-online-build

    cd /tmp/tafl-online-build/packages/web
    ${dx}/bin/dx build --release --package web

    rm -rf ${staticDir}
    cp -r ${stateDir}/target/dx/web/release/web ${staticDir}
    chmod -R u+w ${staticDir}

    # ponytail: dx hashes asset filenames but HTML references unhashed names.
    cd ${staticDir}/public/assets
    for f in *-dxh*; do
      base=$(echo "$f" | sed 's/-dxh[0-9a-f]*//')
      if [ "$f" != "$base" ] && [ ! -e "$base" ]; then
        ln -s "$f" "$base"
      fi
    done
  '';
in {
  systemd.services.tafl-online-build = {
    description = "tafl.online frontend build";
    wantedBy = ["multi-user.target"];
    after = ["network.target"];
    serviceConfig = {
      ExecStart = "${buildScript}";
      Type = "oneshot";
      User = "tafl-web";
      StateDirectory = "tafl-online";
    };
  };

  systemd.services.tafl-online = {
    description = "tafl.online fullstack server";
    wantedBy = ["multi-user.target"];
    after = ["network.target" "tafl-online-build.service" "postgresql.service"];
    requires = ["postgresql.service"];
    serviceConfig = {
      ExecStart = "${staticDir}/server";
      Type = "simple";
      User = "tafl-web";
      StateDirectory = "tafl-online";
      Environment = "DATABASE_URL=postgres:///toph?host=/run/postgresql&user=toph&password=tafl-top-password-change-me"; # ponytail: matches postgresql initialScript password
      Restart = "on-failure";
      RestartSec = 5;
    };
  };

  systemd.services.tafl-online-nginx-reload = {
    description = "Reload nginx after tafl-online build";
    after = ["tafl-online-build.service"];
    requires = ["tafl-online-build.service"];
    serviceConfig = {
      ExecStart = "${pkgs.systemd}/bin/systemctl reload nginx";
      Type = "oneshot";
    };
  };

  users.users.tafl-web = {
    isSystemUser = true;
    group = "tafl-web";
    createHome = true;
  };
  users.groups.tafl-web = {};
}
