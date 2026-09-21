# Garcia Patcher

Patches ALLfiring so it connects to a Garcia server instead of the official
servers. It also restores the game's normal startup class, then aligns and
signs every APK in the XAPK.

## Download

[Download the latest APK](https://apkpure.com/allfiring/com.genmugame.prometheus/download)
[Android Sdk](https://gist.github.com/gugadev/1a4e18b6f2fcd82332e3bac59c10738c)

## Build

```powershell
cargo build --release
```

The patcher reads XAPK/APK archives directly, so 7-Zip is
not required. By default it uses `keytool` from `PATH` and the newest Android
Build Tools under `%LOCALAPPDATA%\Android\Sdk\build-tools`.

## Configure

Edit `config.toml` before patching.

```toml
[endpoints]
host = "10.0.0.186"
game_port = 8888
sdk_port = 18889
hotpatch_port = 18888

[tools]
# zipalign = "C:/Android/Sdk/build-tools/35.0.0/zipalign.exe"
# apksigner = "C:/Android/Sdk/build-tools/35.0.0/apksigner.bat"
# keytool = "C:/Program Files/Java/jdk-23/bin/keytool.exe"
# keystore = "garcia-local.p12"
```

Uncomment tool paths only when the defaults do not work. Relative paths are
resolved from the directory containing the selected configuration file.

## Patch

```powershell
.\target\release\garcia-patcher.exe `
  "C:\path\ALLfiring_1.1.22_APKPure.xapk"
```

Use another configuration file or output path through the CLI:

```powershell
.\target\release\garcia-patcher.exe `
  "C:\path\ALLfiring_1.1.22_APKPure.xapk" `
  --config "C:\path\garcia.toml" `
  --output "C:\path\ALLfiring-garcia.xapk"
```

The output is written next to the input as
`ALLfiring_1.1.22_APKPure-garcia-10.0.0.186.xapk`. The reusable signing key is
stored under the platform data directory (`%LOCALAPPDATA%\GarciaPatcher` on
Windows). Set `tools.keystore` to choose another location.

The patched settings are:

```text
cdn=http://HOST:HOTPATCH_PORT/prod/en/Android
ipaddress=HOST:GAME_PORT
url=http://HOST:SDK_PORT
noticeURL=http://HOST:SDK_PORT
```
