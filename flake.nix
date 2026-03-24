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
                        buildInputs = [ pkgs.sqlite ];

                        # doCheck = false;
                    };
                }
            );

            nixosModules.default =
                {
                    config,
                    lib,
                    pkgs,
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
                        databasePath = lib.mkOption {
                            type = lib.types.str;
                            default = "/var/lib/slimes-server/slimes.db";
                        };
                    };

                    config = lib.mkIf cfg.enable {
                        systemd.services.slimes-server = {
                            description = "Slimes Benchmark Server";
                            after = [ "network.target" ];
                            wantedBy = [ "multi-user.target" ];
                            serviceConfig = {
                                ExecStart = "${
                                    self.packages.${pkgs.system}.default
                                }/bin/slimes-server --database-url ${cfg.databasePath} --port ${toString cfg.port}";
                                Restart = "on-failure";
                                StateDirectory = "slimes-server";
                                DynamicUser = true;
                                ProtectSystem = "strict";
                                ProtectHome = true;
                                NoNewPrivileges = true;
                            };
                        };
                    };
                };
        };
}
