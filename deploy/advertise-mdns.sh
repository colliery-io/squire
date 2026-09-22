#!/bin/sh
# Advertise Squire over mDNS/DNS-SD from the DOCKER HOST (SQUIRE-T-0129).
#
# Why host-side: the container runs in Docker bridge networking, and on Docker Desktop for Mac even
# `network_mode: host` binds to the hidden VM — LAN multicast can never leave the container, so the
# image ships with SQUIRE_MDNS=off. But macOS itself IS a Bonjour machine, so the host advertises on
# the container's behalf. The phone's pairing screen browses `_squire._tcp` to prefill host + port
# (SQUIRE-T-0046), and re-discovers the server by it when a stored address goes stale (SQUIRE-T-0052).
#
# Usage:  ./advertise-mdns.sh [port]
#         Port defaults to SQUIRE_API_PORT from ./.env (else 8080), so it always matches what
#         compose actually publishes; the household TXT record follows SQUIRE_HOUSEHOLD (else
#         `home`). Runs in the FOREGROUND (Ctrl-C to stop); it dies with the terminal/reboot. For
#         boot persistence, install it as a launchd agent — example plist:
#
#   ~/Library/LaunchAgents/io.colliery.squire.mdns.plist
#   <plist version="1.0"><dict>
#     <key>Label</key><string>io.colliery.squire.mdns</string>
#     <key>ProgramArguments</key>
#       <array><string>/usr/bin/dns-sd</string><string>-R</string>
#       <string>Squire</string><string>_squire._tcp</string><string>.</string>
#       <string>8080</string><string>household=home</string></array>
#     <key>RunAtLoad</key><true/><key>KeepAlive</key><true/>
#   </dict></plist>
#   launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/io.colliery.squire.mdns.plist
#
# Not needed when phones are paired with a tailnet address — that name never goes stale.
env_get() { sed -n "s/^$1=//p" "$(dirname "$0")/.env" 2>/dev/null | head -1; }
env_port="$(env_get SQUIRE_API_PORT)"
env_household="$(env_get SQUIRE_HOUSEHOLD)"
PORT="${1:-${env_port:-8080}}"
HOUSEHOLD="${env_household:-home}"
echo "advertising _squire._tcp on port ${PORT} (household=${HOUSEHOLD}; Ctrl-C to stop)"
exec dns-sd -R "Squire" _squire._tcp . "${PORT}" "household=${HOUSEHOLD}"
