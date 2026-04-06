{
  lib,
  craneLib,
  installShellFiles,
}: let
  src = lib.fileset.toSource {
    root = ../.;
    fileset = craneLib.fileset.commonCargoSources ../.;
  };

  inherit (craneLib.crateNameFromCargoToml {inherit src;}) pname version;

  commonArgs = {
    inherit pname version src;
    strictDeps = true;
    doCheck = false;
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
  craneLib.buildPackage (
    commonArgs
    // {
      inherit cargoArtifacts;
      CARGO_BUILD_RUSTFLAGS = "-C strip=symbols";
      nativeBuildInputs = [installShellFiles];
      postInstall = ''
        installShellCompletion --cmd hyprhook \
          --bash <($out/bin/hyprhook completions bash) \
          --fish <($out/bin/hyprhook completions fish) \
          --zsh <($out/bin/hyprhook completions zsh)
      '';
      passthru = {inherit src commonArgs cargoArtifacts;};
    }
  )
