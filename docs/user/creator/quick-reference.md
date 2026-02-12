# Creator Quick Reference

## Command layout (`atlas`)

```bash
atlas login
atlas logout
atlas status
atlas init
atlas reinit
atlas pull
atlas push
atlas build
atlas publish
atlas promote
atlas validate
atlas commit
atlas mod add
atlas mod remove
atlas mod list
atlas mod import
atlas workflow init
atlas workflow update
atlas completion
```

## Most common flows

### Start a pack

```bash
atlas init
atlas build
atlas publish
```

### Ship an update

```bash
atlas validate
atlas build
atlas publish
atlas promote
```

### Work with repo changes

```bash
atlas pull
atlas commit "Describe your change"
atlas push
```

### Manage mods

```bash
atlas mod add mr sodium
atlas mod list
atlas mod remove sodium
```

## Release verbs

- `build`: produce artifact from source.
- `publish`: upload/register build on Hub.
- `promote`: repoint channel to a selected build.

## Channels

- `dev`: active iteration.
- `beta`: pre-release testing.
- `production`: stable player release.
