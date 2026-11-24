# Install Ilvuno on Linux

Ilvuno runs coding agents on a project you choose. It keeps their work in rounds and shows what they did. You choose when to accept their work.

## Before you start

Install at least one coding-agent command-line app, such as Claude Code or Codex. Sign in to it before you open Ilvuno. The `.deb` uses WebKitGTK 4.1 and GTK 3 from your Linux system; its package installer adds those libraries. The AppImage includes the WebKitGTK files used by this build.

## Install and start

For the `.deb` package, open the file in your software installer and choose **Install**. Then open **Ilvuno** from your Applications menu. The package also installs the `ilvuno` command, so you can check its version with:

```sh
ilvuno --version
```

For the AppImage, download the file, open a terminal in its folder, then run:

```sh
chmod +x Ilvuno_0.0.1_amd64.AppImage
./Ilvuno_0.0.1_amd64.AppImage
```

The window starts its bundled local runner when nothing is answering on `127.0.0.1:4737`. If the window says **Runner not running**, close and open Ilvuno again, then choose **Retry**. If it stays offline, check that another app is not using port `4737`.

## Your data

Ilvuno keeps runs and their files in `~/.local/share/ilvuno` by default. If you set `ILVUNO_HOME`, it uses that folder instead. To remove your data, close Ilvuno, stop `ilvunod` in your system monitor if it is still running, then delete the data folder in your file manager. This removes the run history and saved files in that folder.
