# Install Ilvuno on Linux

Ilvuno runs coding agents on a project you choose. It keeps their work in rounds and shows what they did. You choose when to accept their work.

There are three packages. They hold the same things: the window, the `ilvuno` command, and `ilvunod`, the local runner.

| Package | For | Install with |
|---|---|---|
| `Ilvuno_0.0.1_amd64.deb` | Debian, Ubuntu, Mint | your software installer, or `sudo apt install ./Ilvuno_0.0.1_amd64.deb` |
| `Ilvuno-0.0.1-1.x86_64.rpm` | Fedora, RHEL, openSUSE | `sudo dnf install ./Ilvuno-0.0.1-1.x86_64.rpm` |
| `Ilvuno_0.0.1_amd64.AppImage` | any Linux | `chmod +x` the file, then run it |

## Before you start

Install at least one coding-agent command-line app, such as Claude Code or Codex. Sign in to it before you open Ilvuno.

The `.deb` and `.rpm` use WebKitGTK 4.1 and GTK 3 from your system, and the package manager installs them. The AppImage carries the WebKitGTK files used by this build, which is why it is much larger.

## Install and start

For the `.deb`, open the file in your software installer and choose **Install**, then open **Ilvuno** from your Applications menu.

For the `.rpm`, run the command in the table above. The package is not signed, so `dnf` asks about the key. On openSUSE, use `sudo zypper install ./Ilvuno-0.0.1-1.x86_64.rpm`.

For the AppImage, download the file, open a terminal in its folder, then run:

```sh
chmod +x Ilvuno_0.0.1_amd64.AppImage
./Ilvuno_0.0.1_amd64.AppImage
```

The window starts its bundled local runner when nothing is answering on `127.0.0.1:4737`. If the window says **Runner not running**, close and open Ilvuno again, then choose **Retry**. If it stays offline, check that another app is not using port `4737`.

Both packages put the same three programs in the same places:

| Path | What it is |
|---|---|
| `/usr/bin/ilvuno-window` | the window, also the Ilvuno entry in your menu |
| `/usr/bin/ilvuno` | the command line, for goals, status, the gate and exports |
| `/usr/lib/Ilvuno/ilvunod` | the local runner the window talks to |

Check the command line with:

```sh
ilvuno --version
```

## Your data

Ilvuno keeps runs and their files in `~/.local/share/ilvuno` by default. If you set `ILVUNO_HOME`, it uses that folder instead. To remove your data, close Ilvuno, stop `ilvunod` in your system monitor if it is still running, then delete the data folder in your file manager. This removes the run history and saved files in that folder.
