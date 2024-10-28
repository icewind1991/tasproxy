{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.05";
    flakelight = {
      url = "github:nix-community/flakelight";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    mill-scale = {
      url = "github:icewind1991/mill-scale";
      inputs.flakelight.follows = "flakelight";
    };
  };
  outputs = { mill-scale, ... }: mill-scale ./. {
    packages = {
      tasproxy = import ./package.nix;
      docker = import ./docker.nix;
    };

    withOverlays = import ./overlay.nix;

    nixosModules = { outputs, ... }: {
      default =
        { pkgs
        , config
        , lib
        , ...
        }: {
          imports = [ ./module.nix ];
          config = lib.mkIf config.services.tasproxy.enable {
            nixpkgs.overlays = [ outputs.overlays.default ];
            services.tasproxy.package = lib.mkDefault pkgs.tasproxy;
          };
        };
    };
  };
}
