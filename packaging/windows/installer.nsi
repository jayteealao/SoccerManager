; The Soccer Manager setup for Windows: a per-user install with a Start-menu shortcut.
;
; Built by packaging/windows/build.ps1, which passes three defines:
;   VERSION  the version the built program prints
;   STAGE    the staged folder to install
;   OUTFILE  the setup file to write
;
; The install needs no administrator prompt. The uninstaller removes the program and keeps
; the player's matches in %LOCALAPPDATA%\SoccerManager.

!ifndef VERSION
  !error "VERSION is not defined; build with packaging/windows/build.ps1"
!endif
!ifndef STAGE
  !error "STAGE is not defined; build with packaging/windows/build.ps1"
!endif
!ifndef OUTFILE
  !define OUTFILE "SoccerManager-${VERSION}-windows-x64-setup.exe"
!endif

Unicode true
!include "MUI2.nsh"

!define APP_NAME "Soccer Manager"
!define APP_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoccerManager"

Name "${APP_NAME} ${VERSION}"
OutFile "${OUTFILE}"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\SoccerManager"
SetCompressor /SOLID lzma

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} setup"
VIAddVersionKey "LegalCopyright" "MIT OR Apache-2.0"

!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "Start Soccer Manager"
!define MUI_FINISHPAGE_RUN_FUNCTION StartGame
!define MUI_FINISHPAGE_TEXT "Soccer Manager is installed. Start it from the Start menu.$\r$\n$\r$\nYour matches are kept in %LOCALAPPDATA%\SoccerManager. Uninstalling the game keeps them."

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE-MIT"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Function StartGame
  SetOutPath "$INSTDIR"
  Exec '"$INSTDIR\engine-cli.exe" launch --open'
FunctionEnd

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "${STAGE}\*"

  ; The shortcut runs from the install folder, so the page and the content resolve beside it.
  SetOutPath "$INSTDIR"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\engine-cli.exe" "launch --open" "$INSTDIR\engine-cli.exe" 0 SW_SHOWMINIMIZED

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "${APP_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${APP_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${APP_KEY}" "Publisher" "Soccer Manager contributors"
  WriteRegStr HKCU "${APP_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${APP_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${APP_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${APP_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${APP_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  ; Only the program is removed. The player's matches in %LOCALAPPDATA%\SoccerManager stay.
  Delete "$SMPROGRAMS\${APP_NAME}.lnk"
  ; Only what the setup wrote is removed by name, so a folder the player chose that also
  ; holds other files is never emptied.
  Delete "$INSTDIR\engine-cli.exe"
  Delete "$INSTDIR\LICENSE-MIT"
  Delete "$INSTDIR\LICENSE-APACHE"
  RMDir /r "$INSTDIR\content"
  RMDir /r "$INSTDIR\web"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${APP_KEY}"
SectionEnd
