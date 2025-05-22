{pkgs}:
pkgs.mkShell {
  name = "swhkdp-devel";
  nativeBuildInputs = with pkgs; [
    # Compilers
    cargo
    rustc
    scdoc

    # libs
    udev

    # Tools
    cargo-audit
    cargo-deny
    pkg-config
    clippy
    gdb
    gnumake
    rust-analyzer
    rustfmt
    strace
    valgrind
    zip
  ];
}
