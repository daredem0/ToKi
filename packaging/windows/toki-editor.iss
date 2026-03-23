[Setup]
AppName=ToKi Editor
AppVersion={#AppVersion}
DefaultDirName={autopf}\ToKi Editor
DefaultGroupName=ToKi Editor
OutputBaseFilename=ToKi-Editor-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=LICENSE-TOKI.md

[Files]
Source: "toki-editor.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "toki-runtime.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE-TOKI.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\ToKi Editor"; Filename: "{app}\toki-editor.exe"
Name: "{autodesktop}\ToKi Editor"; Filename: "{app}\toki-editor.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"
