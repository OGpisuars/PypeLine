# Security policy

PypeLine runs Python scripts that players write and share. Anything that lets a script reach outside the game is a security bug.

## Please report privately

If you find a way for a script to:
- read or write files, or run other programs,
- use the network,
- reach Python internals the sandbox is meant to hide, or
- crash or freeze the game in a way that survives a restart,

**do not open a public issue.** Email **KiloKiloGames@gmail.com** with:
- a minimal script that shows the problem,
- what happened, and what you expected,
- your OS and the game version (or commit).

We will confirm we received it, fix it before it is disclosed publicly, and gladly credit you.

## Supported versions

Only the latest build from `main` is supported. Fixes are not backported.

## How the sandbox works

See [`PPL/pypeline/docs/SANDBOX.md`](PPL/pypeline/docs/SANDBOX.md) for the layers that protect players and the gaps that are already known.
