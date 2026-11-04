; Clipdat installer (Inno Setup 6). Built by scripts/package.ps1, which passes:
;   /DAppVersion=x.y.z  /DSrc=<dist\Clipdat folder>  /DOut=<output folder>
; Per-user install: no admin prompt, goes to %LOCALAPPDATA%\Programs\Clipdat.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef Src
  #define Src "..\dist\Clipdat"
#endif
#ifndef Out
  #define Out "..\dist"
#endif

[Setup]
AppId={{F9513460-C78E-4272-9DF7-B0CD66C92EB3}
AppName=Clipdat
AppVersion={#AppVersion}
AppVerName=Clipdat {#AppVersion}
AppPublisher=Clipdat
DefaultDirName={autopf}\Clipdat
DefaultGroupName=Clipdat
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#Out}
OutputBaseFilename=Clipdat-Setup-{#AppVersion}
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\clipdat.exe
UninstallDisplayName=Clipdat
LicenseFile={#Src}\LICENSE.txt
AppPublisherURL=https://github.com/ItsTatsuya/clipdat
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
Name: "assoc"; Description: "Register Clipdat as a video player (shows up in Windows Default apps)"; GroupDescription: "File types:"

[InstallDelete]
; Old FFmpeg/mpv DLLs from a previous version would otherwise be left next to the new ones.
Type: filesandordirs; Name: "{app}\bin"

[Files]
Source: "{#Src}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Clipdat"; Filename: "{app}\clipdat.exe"
Name: "{autodesktop}\Clipdat"; Filename: "{app}\clipdat.exe"; Tasks: desktopicon

[Registry]
; Same keys the app writes from Settings -> Make default (src/backend/util.rs register_file_types).
Root: HKA; Subkey: "Software\Classes\Clipdat.Video"; ValueType: string; ValueData: "Video (Clipdat)"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Clipdat.Video\DefaultIcon"; ValueType: string; ValueData: """{app}\clipdat.exe"",0"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Clipdat.Video\shell\open\command"; ValueType: string; ValueData: """{app}\clipdat.exe"" ""%1"""; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\shell\open\command"; ValueType: string; ValueData: """{app}\clipdat.exe"" ""%1"""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Clipdat"; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Video player and clipper"; Tasks: assoc
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Clipdat"; ValueData: "Software\Clipdat\Capabilities"; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mp4"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mp4\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".mp4"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mkv"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mkv\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".mkv"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mov"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mov\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".mov"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".webm"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.webm\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".webm"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".avi"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.avi\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".avi"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".m4v"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.m4v\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".m4v"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ts"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.ts\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".ts"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".m2ts"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.m2ts\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".m2ts"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".flv"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.flv\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".flv"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".wmv"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.wmv\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".wmv"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mpg"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mpg\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".mpg"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mpeg"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.mpeg\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".mpeg"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".3gp"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.3gp\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".3gp"; ValueData: ""; Tasks: assoc
Root: HKA; Subkey: "Software\Clipdat\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ogv"; ValueData: "Clipdat.Video"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\.ogv\OpenWithProgids"; ValueType: string; ValueName: "Clipdat.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Applications\clipdat.exe\SupportedTypes"; ValueType: string; ValueName: ".ogv"; ValueData: ""; Tasks: assoc

[Run]
Filename: "{app}\clipdat.exe"; Description: "Launch Clipdat"; Flags: nowait postinstall skipifsilent
Filename: "{win}\explorer.exe"; Parameters: "ms-settings:defaultapps?registeredAppUser=Clipdat"; Description: "Make Clipdat my default video player (opens Windows Settings)"; Flags: nowait postinstall skipifsilent unchecked shellexec; Tasks: assoc

[UninstallDelete]
; Thumbnail cache. Settings in %APPDATA%\Clipdat are kept.
Type: filesandordirs; Name: "{localappdata}\Clipdat"
