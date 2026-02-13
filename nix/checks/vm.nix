{
  pkgs,
  flake,
  ...
}:
pkgs.testers.nixosTest {
  name = "pictureframe";

  nodes.server = {
    config,
    pkgs,
    ...
  }: {
    imports = [flake.nixosModules.server];

    services.pictureframeServer = {
      enable = true;
      package = flake.packages.${pkgs.system}.default;
      distDir = flake.packages.${pkgs.system}.frontend;
    };
  };

  testScript = ''
    server.wait_for_unit("pictureframe.service")
    server.wait_for_open_port(3000)
    server.succeed("curl --fail http://localhost:3000/")
  '';
}
