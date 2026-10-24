{pkgs ? import <nixpkgs> {}}: let
  cross = pkgs.pkgsCross.aarch64-multiplatform-musl;
  cc = "${cross.stdenv.cc}/bin/${cross.stdenv.cc.targetPrefix}cc";
in
  pkgs.mkShell {
    nativeBuildInputs = [pkgs.rustup cross.stdenv.cc];

    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = cc;
    CC_aarch64_unknown_linux_musl = cc;
    AR_aarch64_unknown_linux_musl = "${cross.stdenv.cc.bintools.bintools}/bin/${cross.stdenv.cc.targetPrefix}ar";
  }
