#!/usr/bin/env bash
set -euo pipefail

TAP=tap0
TAP_ADDR=10.0.2.2/24
NET=10.0.2.0/24
DHCP_START=10.0.2.10
DHCP_END=10.0.2.100
LEASE_FILE=/var/run/dnsmasq-${TAP}.leases
PID_FILE=/var/run/dnsmasq-${TAP}.pid

ip link show $TAP >/dev/null 2>&1 && ip link delete $TAP || true

ip tuntap add dev $TAP mode tap user "$(whoami)"
ip addr add $TAP_ADDR dev $TAP
ip link set dev $TAP up

sysctl -w net.ipv4.ip_forward=1

EXT_IF=$(ip route get 8.8.8.8 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="dev") print $(i+1); exit}')
if [ -z "$EXT_IF" ]; then
  echo "Could not auto-detect external interface. Set EXT_IF manually and re-run."
  exit 1
fi
echo "Using external interface: $EXT_IF"

iptables -t nat -D POSTROUTING -o "$EXT_IF" -s $NET -j MASQUERADE 2>/dev/null || true
iptables -D FORWARD -i "$EXT_IF" -o $TAP -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
iptables -D FORWARD -i $TAP -o "$EXT_IF" -j ACCEPT 2>/dev/null || true

iptables -t nat -A POSTROUTING -o "$EXT_IF" -s $NET -j MASQUERADE
iptables -A FORWARD -i "$EXT_IF" -o $TAP -m state --state RELATED,ESTABLISHED -j ACCEPT
iptables -A FORWARD -i $TAP -o "$EXT_IF" -j ACCEPT

pkill -F "$PID_FILE" 2>/dev/null || true
dnsmasq \
  --interface=$TAP \
  --bind-interfaces \
  --except-interface=lo \
  --listen-address=10.0.2.2 \
  --dhcp-range=${DHCP_START},${DHCP_END},12h \
  --dhcp-option=3,10.0.2.2 \
  --dhcp-option=6,8.8.8.8,1.1.1.1 \
  --dhcp-leasefile=$LEASE_FILE \
  --pid-file=$PID_FILE \
  --conf-file= --no-hosts \
  --log-facility=/var/log/dnsmasq-${TAP}.log &

sleep 0.5
if [ -f "$PID_FILE" ]; then
  echo "dnsmasq started for $TAP (lease file: $LEASE_FILE pid: $(cat $PID_FILE))"
else
  echo "dnsmasq failed to start; check /var/log/dnsmasq-${TAP}.log"
  exit 1
fi

echo "TAP $TAP up at $TAP_ADDR, DHCP range ${DHCP_START}-${DHCP_END}."
echo "Guests should get DHCP + DNS and access to the Internet (via $EXT_IF)."
