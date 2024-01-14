{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.tasproxy;
in {
  options.services.tasproxy = {
    enable = mkEnableOption "Log archiver";

    port = mkOption {
      type = types.int;
      default = 3000;
      description = "port to listen on";
    };

    mqttCredentailsFile = mkOption {
      type = types.str;
      description = "file containg MQTT_HOSTNAME, MQTT_USERNAME and MQTT_PASSWORD variables";
    };

    enableUnixSocket = mkOption {
      type = types.bool;
      default = false;
      description = "listen to a unix socket instead of TCP";
    };

    package = mkOption {
      type = types.package;
      defaultText = literalExpression "pkgs.tasproxy";
      description = "package to use";
    };
  };

  config = mkIf cfg.enable {
    systemd.services."tasproxy" = {
      wantedBy = ["multi-user.target"];
      environment =
        if cfg.enableUnixSocket
        then {
          SOCKET = "/run/tasproxy/tasproxy.sock";
        }
        else {
          PORT = cfg.port;
        };

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/tasproxy";
        EnvironmentFile = cfg.mqttCredentailsFile;
        Restart = "on-failure";
        DynamicUser = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        PrivateDevices = true;
        ProtectClock = true;
        CapabilityBoundingSet = true;
        ProtectKernelLogs = true;
        ProtectControlGroups = true;
        SystemCallArchitectures = "native";
        ProtectKernelModules = true;
        RestrictNamespaces = true;
        MemoryDenyWriteExecute = true;
        ProtectHostname = true;
        LockPersonality = true;
        ProtectKernelTunables = true;
        RestrictAddressFamilies = "AF_INET AF_INET6 AF_UNIX";
        RestrictRealtime = true;
        ProtectProc = "noaccess";
        SystemCallFilter = ["@system-service" "~@resources" "~@privileged"];
        IPAddressDeny = "multicast";
        PrivateUsers = true;
        ProcSubset = "pid";
        RuntimeDirectory = "tasproxy";
        RestrictSUIDSGID = true;
      };
    };
  };
}
