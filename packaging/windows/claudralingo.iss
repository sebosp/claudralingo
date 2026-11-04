; The Windows installer, built by the Builds workflow with Inno Setup:
;   ISCC.exe /DAppVersion=<version> packaging\windows\claudralingo.iss
; after `cargo build --release`. It installs for the user (no administrator, under
; %LOCALAPPDATA%\Programs), with a Start menu entry, an optional desktop icon and an uninstaller.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{22E3D8F7-04FF-4592-AFB5-BBE3CF1E1212}
AppName=Claudralingo
AppVersion={#AppVersion}
AppVerName=Claudralingo {#AppVersion}
AppPublisher=Seb Ospina
AppPublisherURL=https://github.com/sebosp/claudralingo
AppSupportURL=https://github.com/sebosp/claudralingo/issues
AppUpdatesURL=https://github.com/sebosp/claudralingo/releases
DefaultDirName={autopf}\Claudralingo
DefaultGroupName=Claudralingo
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..
OutputBaseFilename=claudralingo-windows-x86_64-setup
SetupIconFile=claudralingo.ico
UninstallDisplayIcon={app}\claudralingo.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\release\claudralingo.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
; The embedded hanzi data: its licenses and attributions.
Source: "..\..\assets\hanzi\LICENSES.md"; DestDir: "{app}\hanzi-licenses"; Flags: ignoreversion
Source: "..\..\assets\hanzi\ARPHICPL.TXT"; DestDir: "{app}\hanzi-licenses"; Flags: ignoreversion
Source: "..\..\assets\hanzi\OFL.txt"; DestDir: "{app}\hanzi-licenses"; Flags: ignoreversion

[Icons]
Name: "{group}\Claudralingo"; Filename: "{app}\claudralingo.exe"
Name: "{group}\Uninstall Claudralingo"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Claudralingo"; Filename: "{app}\claudralingo.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\claudralingo.exe"; Description: "{cm:LaunchProgram,Claudralingo}"; Flags: nowait postinstall skipifsilent

; The progress (%APPDATA%\claudralingo\progress.json) is left in place by the uninstaller.
