{
  lib,
  rustPlatform,
  makeWrapper,
  pkg-config,
  udev,
  killall,
  rfkillFeature ? false,
}:
rustPlatform.buildRustPackage rec {
  pname = "swhkdp";
  version = "1.3.1-git";

  src = lib.cleanSource ../.;

  cargoLock.lockFile = "${src}/Cargo.lock";

  buildFeatures = [] ++ lib.optional rfkillFeature "rfkill";

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = [
    udev
    killall
  ];

  postInstall = ''
        mkdir -p $out/share/polkit-1/actions
        cat > $out/share/polkit-1/actions/com.github.swhkdp.pkexec.policy <<EOF
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN" "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
    <policyconfig>
      <action id="com.github.swhkdp.pkexec">
        <message>Authentication is required to run Simple Wayland Hotkey Daemon</message>
        <defaults>
          <allow_any>no</allow_any>
          <allow_inactive>no</allow_inactive>
          <allow_active>yes</allow_active>
        </defaults>
        <annotate key="org.freedesktop.policykit.exec.path">$out/bin/swhkdp</annotate>
      </action>
    </policyconfig>
    EOF
  '';
}
