# Sanitize checklist

## Automated scan result

| File | Line | Item | Suggested replacement |
| --- | --- | --- | --- |
| `[CURRENT]` All files under `docs/proposal/` | n/a | No absolute POSIX paths, mounted-volume paths, Windows drive paths, desktop hostnames, email addresses, external URLs, or uppercase environment-variable references were found | No replacement needed |

`[CURRENT]` The required path/hostname scan returned no matches.

`[CURRENT]` An additional scan for web URL schemes, email-shaped text, and
uppercase environment-variable references also returned no matches.

`[CURRENT]` Reproduce both checks without making this checklist match itself:

```text
rg -n '[/]home[/]|[/]mnt[/]|[A-Za-z]:[\\]|[D]ESKTOP-' docs/proposal
rg -n '[h]ttps?[:]//|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+|[$][A-Z][A-Z0-9_]*' docs/proposal
```

## Manual review notes

`[TARGET]` Public software and project names such as Fornax, Adobe, Sony,
Canon, Nikon, LibRaw, darktable, RawTherapee, PyO3, DuckDB, and raw.pixls.us
remain because they are technically relevant to the proposal, not because they
are credentials or private endpoints.

`[TARGET]` The baseline contains the repository-relative submodule path
`external/raw.pixls.us`; it is retained as required worktree evidence and can be
replaced with a generic statement about the pre-existing submodule change if
the maintainer wants a cleaner handoff.

`[TARGET]` No tokens, credentials, usernames, private repository URLs, or
local machine paths were intentionally included.
