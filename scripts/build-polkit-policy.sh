#!/bin/sh

###############
# Defaults
###############

readonly DEFAULT_swhkdp_PATH="/usr/bin/swhkdp"
readonly DEFAULT_POLICY_PATH="com.github.swhkdp.pkexec.policy"
readonly DEFAULT_MESSAGE="Authentication is required to run Simple Wayland Hotkey Daemon"
readonly DEFAULT_ACTION_ID="com.github.swhkdp.pkexec"

###############
# Init
###############

print_help() {
    printf "Usage: build-polkit-policy [OPTIONS]\n\n"
    printf "Generates a polkit policy file for swhkdp.\n\n"
    printf "Optional Arguments:\n"
    printf " --policy-path=<path> Path to save the policy file to.\n"
    printf "                      If set to '-', this tool will output to stdout instead.\n"
    printf "                      Defaults to '%s'.\n" "${DEFAULT_POLICY_PATH}"
    printf " --swhkdp-path=<path>  Path to the swhkdp binary when installed.\n"
    printf "                      Defaults to '%s'.\n" "${DEFAULT_swhkdp_PATH}"
    printf " --action-id=<id>     Polkit action id to use.\n"
    printf "                      Defaults to '%s'.\n" "${DEFAULT_ACTION_ID}"
    printf " --message=<text>     Custom authentication message.\n"
    printf "                      Defaults to '%s'\n" "${DEFAULT_MESSAGE}"
    printf " -h|--help            Show this help.\n"
}

while [ -n "$1" ]; do
    case "$1" in
        --policy-path=*)
            POLICY_PATH=${1#*=}
            shift
            ;;
        --swhkdp-path=*)
            swhkdp_PATH=${1#*=}
            shift
            ;;
        --action-id=*)
            ACTION_ID=${1#*=}
            shift
            ;;
        --message=*)
            MESSAGE=${1#*=}
            shift
            ;;
        -h|--help)
            print_help
            exit 0
            ;;
        *)
            printf "Unknown option '%s'. Aborting.\n" "$1"
            exit 1
            ;;
    esac
done

print_policy() {
cat << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN" "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
<policyconfig>
  <action id="${ACTION_ID}">
    <message>${MESSAGE}</message>
    <defaults>
      <allow_any>no</allow_any>
      <allow_inactive>no</allow_inactive>
      <allow_active>yes</allow_active>
    </defaults>
    <annotate key="org.freedesktop.policykit.exec.path">${swhkdp_PATH}</annotate>
  </action>
</policyconfig>
EOF
}

# No local variables in POSIX sh, so just set these globally
POLICY_PATH="${POLICY_PATH:-${DEFAULT_POLICY_PATH}}"
swhkdp_PATH="${swhkdp_PATH:-${DEFAULT_swhkdp_PATH}}"
ACTION_ID="${ACTION_ID:-${DEFAULT_ACTION_ID}}"
MESSAGE="${MESSAGE:-${DEFAULT_MESSAGE}}"

if [ "${POLICY_PATH}" = "-" ]; then
    print_policy
else
    print_policy > "${POLICY_PATH}"
fi
