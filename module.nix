{ config
, lib
, pkgs
, ...
}:
with lib; let
  cfg = config.services.tasproxy;
  format = pkgs.formats.toml { };
  configFile = format.generate "tasproxy-config.toml" {
    listen = {
      inherit (cfg) socket;
    };
    mqtt = {
      inherit (cfg.mqtt) hostname port;
    } // (
      optionalAttrs (cfg.mqtt.passwordFile != null) {
        inherit (cfg.mqtt) username;
        password_file = "$CREDENTIALS_DIRECTORY/mqtt_password";
      }
    );
    tasmota = optionalAttrs (cfg.tasmota.username != null) {
      inherit (cfg.tasmota) username;
      password_file = "$CREDENTIALS_DIRECTORY/tasmota_password";
    };
  };
in
{
  options.services.tasproxy = {
    enable = mkEnableOption "Log archiver";

    port = mkOption {
      type = types.int;
      default = 3000;
      description = "port to listen on, if enableUnixSocket is not set";
    };

    socket = mkOption {
      type = types.str;
      default = "/run/tasproxy/tasproxy.socket";
      description = "socket to listen on, if enableUnixSocket is set";
    };

    mqtt = mkOption {
      type = types.submodule {
        options = {
          hostname = mkOption {
            type = types.str;
            description = "Hostname of the MQTT server";
          };
          port = mkOption {
            type = types.port;
            default = 1883;
            description = "Port of the MQTT server";
          };
          username = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Username for the MQTT server";
          };
          passwordFile = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "File containing the password for the MQTT server";
          };
        };
      };
    };

    tasmota = mkOption {
      type = types.submodule {
        options = {
          username = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Username for the tasmota devices";
          };
          passwordFile = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "File containing the password for the tasmota devices";
          };
        };
      };
      default = { };
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
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        LoadCredential = (optional (cfg.mqtt.passwordFile != null) [
          "mqtt_password:${cfg.mqtt.passwordFile}"
        ]) ++ (optional (cfg.tasmota.passwordFile != null) [
          "tasmota_password:${cfg.tasmota.passwordFile}"
        ]);

        ExecStart = "${cfg.package}/bin/tasproxy ${configFile}";

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
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" ] ++ optionals cfg.enableUnixSocket [ "AF_UNIX" ];
        RestrictRealtime = true;
        ProtectProc = "noaccess";
        SystemCallFilter = [ "@system-service" "~@resources" "~@privileged" ];
        IPAddressDeny = "multicast";
        PrivateUsers = true;
        ProcSubset = "pid";
        RuntimeDirectory = "tasproxy";
        RestrictSUIDSGID = true;
      };
    };
  };
}
