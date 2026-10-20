# Desktop integration (Linux)

DeLOG accepts log files on the command line, so the desktop can hand them
over directly:

```bash
delog flight.ulg logs/telemetry.BIN   # built-in formats, opened on startup
delog capture.exotic                   # a learned custom-parser extension
```

Command-line opens go through the exact same flow as **File ▸ Open**,
including the remembered [extension → parser](custom_parsers.md) routing for
custom Python parsers.

Double-clicking a log in the file manager (or `xdg-open log.ulg`) needs one
more step: the desktop must know DeLOG exists and which file types it owns.
`install-association.sh` registers everything for the **current user** - no
root, no system files:

```bash
packaging/linux/install-association.sh ./DeLOG-v0.4.1-x86_64.AppImage
# or, for a binary/tarball install:
packaging/linux/install-association.sh /opt/delog/delog
```

The script installs three things under `$XDG_DATA_HOME` (`~/.local/share`):

| What | Where | Purpose |
| --- | --- | --- |
| `mime/packages/delog-mime.xml` | `~/.local/share/mime/` | defines the `application/x-ulg`, `x-ardupilot-bin`, `x-mavlink-tlog` types and their `*.ulg`/`*.bin`/`*.tlog` globs |
| `applications/delog.desktop` | `~/.local/share/applications/` | a desktop entry whose `Exec` runs the path you passed, with `%F` so files are forwarded |
| `xdg-mime default` | per-type | makes DeLOG the default handler for the three types |

After it runs, verify and test:

```bash
xdg-mime query default application/x-ulg   # → delog.desktop
xdg-open flight.ulg                          # DeLOG starts with the file
```

## Registering exotic extensions

Custom-parser extensions can be associated the same way: add a
`<mime-type>` block to `delog-mime.xml` with a glob for your extension
(keep the file type lowercase and prefixed, e.g. `application/x-delog-bb`):

```xml
<mime-type type="application/x-delog-bb">
  <comment>DéLOG raw capture</comment>
  <glob pattern="*.bb"/>
</mime-type>
```

Re-run the script (it refreshes the MIME database and re-points the desktop
entry), then add the type to the `MimeType=` line of the generated
`~/.local/share/applications/delog.desktop` and the `xdg-mime default`
loop.

## Caveats

- **`.bin` is a broad glob.** Firmware images and other tools also use
  `.bin`; after registration the file manager will offer DeLOG for every
  `.bin` file. Remove the `x-ardupilot-bin` block from `delog-mime.xml` if
  that is not what you want.
- **AppImageLauncher users**: it registers its own desktop entry from the
  AppImage's internal `packaging/linux/delog.desktop` (which also carries
  `MimeType=`); the script is still the reliable way to get the MIME globs
  and defaults in place, and it re-points `Exec` at the AppImage wherever
  it lives.
- **First double-click** may need the session to pick up the new desktop
  entry; if nothing happens, log out and back in once.

## Uninstall

```bash
rm ~/.local/share/mime/packages/delog-mime.xml ~/.local/share/applications/delog.desktop
update-mime-database ~/.local/share/mime
```
