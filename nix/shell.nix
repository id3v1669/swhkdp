{pkgs}:
pkgs.mkShell {
  name = "Swhkd-devel";
  nativeBuildInputs = with pkgs; [
    # Compilers
    cargo
    rustc
    scdoc

    # libs
    udev

    # Tools
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