# Environment Config

Reserved for environment-level policies such as development, research, and CI behavior.

Recommended local pattern:

- keep machine-local secrets in ignored files named `.env.*`
- for example: `configs/environments/.env.seraph-local`
- load them with:

```bash
set -a && source configs/environments/.env.seraph-local && set +a
```

Tracked examples live under `configs/examples/`.
