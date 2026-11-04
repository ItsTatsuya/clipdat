; Cliper installer (Inno Setup 6). Built by scripts/package.ps1, which passes:
;   /DAppVersion=x.y.z  /DSrc=<dist\Cliper folder>  /DOut=<output folder>
; Per-user install: no admin prompt, goes to %LOCALAPPDATA%\Programs\Cliper.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef Src
  #define Src "..\dist\Cliper"
#endif
#ifndef Out
  #define Out "..\dist"
#endif

[Setup]
AppId={{6F1C2A54-9B1E-4C1F-9C11-C11BE9000A01}
AppName=Cliper
AppVersion={#AppVersion}
AppVerName=Cliper {#AppVersion}
AppPublisher=Cliper
DefaultDirName={autopf}\Cliper
DefaultGroupName=Cliper
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#Out}
OutputBaseFilename=Cliper-Setup-{#AppVersion}
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\cliper.exe
UninstallDisplayName=Cliper
WizardStyle=modern
Compression=lzma2/ultra64
SolidCompression=yes
LZMAUseSeparateProcess=yes
LZMANumBlockThreads=4
ChangesAssociations=yes
CloseApplications=force
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: "assoc"; Description: "Register Cliper as a video player (shows up in Windows Default apps)"; GroupDescription: "File types:"

[InstallDelete]
; Old FFmpeg/mpv DLLs from a previous version would otherwise be left next to the new ones.
Type: filesandordirs; Name: "{app}\bin"

[Files]
Source: "{#Src}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Cliper"; Filename: "{app}\cliper.exe"
Name: "{autodesktop}\Cliper"; Filename: "{app}\cliper.exe"; Tasks: desktopicon

[Registry]
; Same keys the app writes from Settings -> Make default (src/backend/util.rs register_file_types).
Root: HKA; Subkey: "Software\Classes\Cliper.Video"; ValueType: string; ValueData: "Video (Cliper)"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Cliper.Video\DefaultIcon"; ValueType: string; ValueData: """{app}\cliper.exe"",0"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Cliper.Video\shell\open\command"; ValueType: string; ValueData: """{app}\cliper.exe"" ""%1"""; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\shell\open\command"; ValueType: string; ValueData: """{app}\cliper.exe"" ""%1"""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Cliper"; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Video player and clipper"; Tasks: assoc
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Cliper"; ValueData: "Software\Cliper\Capabilities"; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mp4"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mp4\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".mp4"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mkv"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mkv\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".mkv"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mov"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mov\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".mov"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".webm"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.webm\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".webm"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".avi"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.avi\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".avi"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".m4v"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.m4v\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".m4v"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ts"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.ts\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".ts"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".m2ts"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.m2ts\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".m2ts"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".flv"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.flv\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".flv"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".wmv"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.wmv\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".wmv"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mpg"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mpg\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".mpg"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mpeg"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mpeg\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".mpeg"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".3gp"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.3gp\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".3gp"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Cliper\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ogv"; ValueData: "Cliper.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.ogv\OpenWithProgids"; ValueType: string; ValueName: "Cliper.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\cliper.exe\SupportedTypes"; ValueType: string; ValueName: ".ogv"; ValueData: ""; Tasks: assoc

[Run]
Filename: "{app}\cliper.exe"; Description: "Launch Cliper"; Flags: nowait postinstall skipifsilent
Filename: "{win}\explorer.exe"; Parameters: "ms-settings:defaultapps?registeredAppUser=Cliper"; Description: "Make Cliper my default video player (opens Windows Settings)"; Flags: nowait postinstall skipifsilent unchecked shellexec; Tasks: assoc

[UninstallDelete]
; Thumbnail cache. Settings in %APPDATA%\Cliper are kept.
Type: filesandordirs; Name: "{localappdata}\Cliper"
