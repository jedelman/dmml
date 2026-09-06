#!/usr/bin/env bash
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ls -la ~/.ghcup/bin/ 2>&1
echo "---log tail---"
tail -20 ~/.ghcup/logs/*.log 2>&1
