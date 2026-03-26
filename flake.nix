{
  description = "Slimes Benchmark Server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "slimes-server";
            version = "0.1.0";

            src = ./server;

            cargoLock.lockFile = ./server/Cargo.lock;

            nativeBuildInputs = [ pkgs.pkg-config ];
            # buildInputs = [ pkgs.sqlite ];

            # doCheck = false;
          };
        }
      );

      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          inputs,
          ...
        }:
        let
          cfg = config.services.slimes-server;
        in
        {
          options.services.slimes-server = {
            enable = lib.mkEnableOption "Slimes Server";
            port = lib.mkOption {
              type = lib.types.port;
              default = 9003;
            };
            databaseUrl = lib.mkOption {
              type = lib.types.str;
              default = "postgres://slimes:secret@127.0.0.1:9005/slimes";
            };
          };

          config = lib.mkIf cfg.enable {
            systemd.services.slimes-server = {
              description = "Slimes Benchmark Server";
              after = [
                "network.target"
                "slimes-server-db-container.service"
              ];
              requires = [ "slimes-server-db-container.service" ];
              wantedBy = [ "multi-user.target" ];
              serviceConfig = {
                ExecStart = "${
                  self.packages.${pkgs.system}.default
                }/bin/slimes-server --database-url ${cfg.databaseUrl} --port ${toString cfg.port}";
                Restart = "on-failure";
                # StateDirectory = "slimes-server";
                # DynamicUser = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                NoNewPrivileges = true;
              };
            };

            systemd.services."slimes-server-db-container" = {
              wantedBy = [ "multi-user.target" ];
              serviceConfig = {
                StateDirectory = "slimes-server-db";
                ExecStart = ''
                  ${pkgs.podman}/bin/podman run \
                    --name slimes-server-db \
                    --replace \
                    -p 127.0.0.1:9005:5432 \
                    -e POSTGRES_PASSWORD=secret \
                    -e POSTGRES_USER=slimes \
                    -e POSTGRES_DB=slimes \
                    -v /var/lib/slimes-server-db:/var/lib/postgresql/data \
                    docker.io/library/postgres:15
                '';
                ExecStop = "${pkgs.podman}/bin/podman stop slimes-server-db";
                Restart = "always";
              };
            };
          };
        };
    };
}
