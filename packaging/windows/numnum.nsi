; NSIS installer for NumNum.
;
; Compiled in CI by makensis. Pass the version with /DVERSION=0.2.1 and run
; makensis with the working directory set to the staging folder that holds
; numnum.exe, numnum.ico, README.md and LICENSE.

Unicode true

!ifndef VERSION
  !define VERSION "0.0.0"
!endif

!define APPNAME "NumNum"
!define PUBLISHER "Rudrabhoj Bhati"
!define APPEXE "numnum.exe"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"

!include "MUI2.nsh"

Name "${APPNAME} ${VERSION}"
OutFile "numnum-setup.exe"
InstallDir "$PROGRAMFILES64\${APPNAME}"
InstallDirRegKey HKLM "Software\${APPNAME}" "InstallDir"
RequestExecutionLevel admin

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APPNAME}"
VIAddVersionKey "FileDescription" "A text editor that does math"
VIAddVersionKey "FileVersion" "${VERSION}.0"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

!define MUI_ICON "numnum.ico"
!define MUI_UNICON "numnum.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APPEXE}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "NumNum"
  SetOutPath "$INSTDIR"
  File "${APPEXE}"
  File "numnum.ico"
  File "README.md"
  File "LICENSE"

  CreateShortcut "$SMPROGRAMS\${APPNAME}.lnk" "$INSTDIR\${APPEXE}" "" "$INSTDIR\numnum.ico"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "Software\${APPNAME}" "InstallDir" "$INSTDIR"

  WriteRegStr HKLM "${UNINSTKEY}" "DisplayName" "${APPNAME}"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTKEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\numnum.ico"
  WriteRegStr HKLM "${UNINSTKEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${APPEXE}"
  Delete "$INSTDIR\numnum.ico"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APPNAME}.lnk"
  DeleteRegKey HKLM "${UNINSTKEY}"
  DeleteRegKey HKLM "Software\${APPNAME}"
SectionEnd
