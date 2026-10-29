# Setup

```bash
curl -fsSL https://x.example/install.sh -o install.sh
less install.sh
sh install.sh --dry-run
```

Use `Invoke-WebRequest https://x.example/i.ps1 -OutFile i.ps1` and review it first.

Read the version with `curl -s https://api.example/v1/pkg | python3 -c "import json,sys; print(json.load(sys.stdin)['v'])"`.

Check a flag with `curl -s https://api.example/v1/pkg | python3 -c "import sys,json; d=json.load(sys.stdin); print('exec' in d)"`.

Save a copy with `curl -s https://api.example/v1/pkg | tee pkg.json | jq .version`.
