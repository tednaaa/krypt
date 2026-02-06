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

  processes = {
    scanner_api.exec = ''
      cargo run --bin scanner_api
    '';

    frontend.exec = "cd frontend && pnpm dev";
  };

  packages = [ pkgs.openssl ];
}
