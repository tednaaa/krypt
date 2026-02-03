{
  pkgs,

  lib,
  config,
  inputs,
  ...
}:
{
  languages = {
    rust.enable = true;
    javascript = {
      enable = true;
      package = pkgs.nodejs_24;
    };
  };

  packages = [ pkgs.openssl ];
}
