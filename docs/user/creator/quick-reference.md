---
title: Creator quick reference
summary: Day-to-day Atlas CLI commands and compact release runbooks.
persona: creator
order: 4
keywords: ["atlas", "cli", "commands", "quick reference"]
intent: reference
---

# Creator quick reference

Use this page as an operational shortcut for common creator tasks.

## Command index

This list covers the highest-frequency Atlas creator commands.

```bash
atlas login
atlas logout
atlas status
atlas init
atlas reinit
atlas pull
atlas push
atlas validate
atlas build
atlas publish
atlas promote
atlas commit
atlas mod add
atlas mod remove
atlas mod list
atlas mod import
atlas workflow init
atlas workflow update
atlas completion
```

## Start a new pack release

Use this flow for first publish from a repository.

```bash
atlas init
atlas build
atlas publish
```

## Ship a routine update

Use this flow for normal ongoing releases.

```bash
atlas validate
atlas build
atlas publish
atlas promote
```

## Synchronize source changes

Use this flow when coordinating repo updates.

```bash
atlas pull
atlas commit "Describe your change"
atlas push
```

## Release verbs

These definitions keep command intent consistent across teams.

- `build`: create an artifact from source.
- `publish`: upload and register a build in Hub.
- `promote`: repoint a channel to a selected build.
